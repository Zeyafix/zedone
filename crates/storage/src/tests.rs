use std::{path::Path, process::Command};

use automerge::{Automerge, ROOT, ReadDoc, sync::State, transaction::Transactable};
use rusqlite::Connection;
use serde::Serialize;
use tempfile::TempDir;
use zedone_core::error::{CommandError, ErrorCode};

use super::*;

fn rejected() -> StorageError {
    CommandError {
        code: ErrorCode::StaleState,
    }
    .into()
}

fn text(doc: &Automerge, field: &str) -> String {
    doc.get(ROOT, field)
        .unwrap()
        .unwrap()
        .0
        .as_str()
        .unwrap()
        .to_owned()
}

async fn open() -> (TempDir, Storage) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    (dir, storage)
}

async fn seed(storage: &Storage) {
    storage
        .write(b"seed".to_vec(), b"initial request".to_vec(), |ctx| {
            for space in ["a", "b"] {
                ctx.create_document(space)?;
                ctx.edit(space, |tx| {
                    tx.put(ROOT, "title", "before")?;
                    tx.put(ROOT, "history-0", "before")?;
                    Ok(())
                })?;
            }
            Ok("seeded")
        })
        .await
        .unwrap();
}

fn mutation(ctx: &mut WriteContext<'_>) -> StorageResult<&'static str> {
    for space in ["a", "b"] {
        ctx.edit(space, |tx| {
            tx.put(ROOT, "title", "after")?;
            tx.put(ROOT, "history-1", "after")?;
            Ok(())
        })?;
        let state = State {
            shared_heads: ctx.read().document(space)?.get_heads(),
            ..State::new()
        };
        ctx.set_peer_state(space, b"peer", &state)?;
    }
    ctx.create_document("new-space")?;
    Ok("committed")
}

async fn assert_state(storage: &Storage, changed: bool) {
    storage
        .read(move |ctx| {
            for space in ["a", "b"] {
                let graph = ctx.document(space)?;
                assert_eq!(
                    text(graph, "title"),
                    if changed { "after" } else { "before" }
                );
                assert_eq!(text(graph, "history-0"), "before");
                assert_eq!(graph.get(ROOT, "history-1")?.is_some(), changed);
                let heads = graph.get_heads();
                let peer = ctx.peer_state(space, b"peer")?;
                assert_eq!(!peer.shared_heads.is_empty(), changed);
                if changed {
                    assert_eq!(peer.shared_heads, heads);
                }
            }
            assert_eq!(ctx.document("new-space").is_ok(), changed);
            let record = ctx.recorded(b"mutation")?;
            assert_eq!(record.is_some(), changed);
            if let Some(record) = record {
                assert_eq!(record.request, b"original request");
                assert_eq!(record.result, br#""committed""#);
            }
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn atomic_restart_and_reverse_preserve_actor_history_peer_and_receipt() {
    let (dir, storage) = open().await;
    seed(&storage).await;
    let actor = storage
        .read(|ctx| Ok(ctx.document("a")?.get_actor().clone()))
        .await
        .unwrap();
    storage
        .write(b"mutation".to_vec(), b"original request".to_vec(), mutation)
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    assert_state(&storage, true).await;
    storage
        .write(b"reverse".to_vec(), vec![], move |ctx| {
            assert_eq!(*ctx.read().document("a")?.get_actor(), actor);
            ctx.edit("a", |tx| {
                tx.put(ROOT, "title", "before")?;
                tx.put(ROOT, "history-2", "before")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    storage
        .read(|ctx| {
            assert_eq!(text(ctx.document("a")?, "title"), "before");
            assert_eq!(text(ctx.document("a")?, "history-1"), "after");
            assert_eq!(text(ctx.document("b")?, "title"), "after");
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn checkpoints_never_expose_partial_state_or_false_failure() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (dir, storage) = open().await;
        seed(&storage).await;
        let result = storage
            .write_checked(
                b"mutation".to_vec(),
                b"original request".to_vec(),
                mutation,
                move |point| {
                    if point == stage {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await;
        assert_eq!(
            result,
            Err(if stage == Stage::AfterCommit {
                StorageError::OutcomeUnknown
            } else {
                StorageError::Unavailable
            })
        );
        assert_state(&storage, stage == Stage::AfterCommit).await;
        storage.shutdown().await.unwrap();
        let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
        assert_state(&storage, stage == Stage::AfterCommit).await;
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn real_sql_failures_rollback_changes_peers_actor_and_result() {
    for table in ["documents", "changes", "peers", "command_results"] {
        let (_dir, storage) = open().await;
        seed(&storage).await;
        storage.run(move |c| {
            c.execute_batch(&format!("CREATE TEMP TRIGGER reject_insert BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'test'); END"))?;
            Ok(())
        }).await.unwrap();
        assert_eq!(
            storage
                .write(b"mutation".to_vec(), b"original request".to_vec(), mutation)
                .await,
            Err(StorageError::Unavailable)
        );
        assert_state(&storage, false).await;
        storage
            .run(|c| {
                c.execute_batch("DROP TRIGGER reject_insert")?;
                Ok(())
            })
            .await
            .unwrap();
        storage
            .write(b"mutation".to_vec(), b"original request".to_vec(), mutation)
            .await
            .unwrap();
        assert_state(&storage, true).await;
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn disk_full_does_not_commit_a_partial_command() {
    let (_dir, storage) = open().await;
    seed(&storage).await;
    storage
        .run(|c| {
            let count: i64 = c.pragma_query_value(None, "page_count", |r| r.get(0))?;
            c.pragma_update(None, "max_page_count", count)?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        storage
            .write(b"mutation".to_vec(), b"original request".to_vec(), |ctx| {
                mutation(ctx)?;
                Ok(vec![42_u8; 1024 * 1024])
            })
            .await,
        Err(StorageError::Unavailable)
    );
    assert_state(&storage, false).await;
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn commit_error_is_unknown_and_connection_recovers_without_partial_data() {
    let (_dir, storage) = open().await;
    seed(&storage).await;
    storage.run(|c| {
        c.execute_batch("CREATE TEMP TABLE parent(id INTEGER PRIMARY KEY);
            CREATE TEMP TABLE child(id INTEGER REFERENCES parent(id) DEFERRABLE INITIALLY DEFERRED);
            CREATE TEMP TRIGGER defer_failure AFTER INSERT ON main.command_results BEGIN INSERT INTO child VALUES (1); END;")?;
        Ok(())
    }).await.unwrap();
    assert_eq!(
        storage
            .write(b"mutation".to_vec(), b"original request".to_vec(), mutation)
            .await,
        Err(StorageError::OutcomeUnknown)
    );
    assert_state(&storage, false).await;
    storage
        .run(|c| {
            c.execute_batch("DROP TRIGGER defer_failure; DROP TABLE child; DROP TABLE parent;")?;
            Ok(())
        })
        .await
        .unwrap();
    storage
        .write(b"mutation".to_vec(), b"original request".to_vec(), mutation)
        .await
        .unwrap();
    assert_state(&storage, true).await;
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn caught_mutation_failure_cannot_commit_earlier_edits() {
    let (_dir, storage) = open().await;
    seed(&storage).await;
    assert_eq!(
        storage
            .write(b"mutation".to_vec(), vec![], |ctx| {
                mutation(ctx)?;
                let failure: StorageResult<()> = ctx.edit("a", |tx| {
                    tx.put(ROOT, "title", "must not appear")?;
                    Err(rejected())
                });
                assert_eq!(failure, Err(rejected()));
                Ok("ignored")
            })
            .await,
        Err(StorageError::Unavailable)
    );
    assert_state(&storage, false).await;
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn rejected_work_and_unserializable_result_rollback() {
    struct BadResult;
    impl Serialize for BadResult {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("test failure"))
        }
    }
    let (_dir, storage) = open().await;
    seed(&storage).await;
    let result: StorageResult<()> = storage
        .write(b"mutation".to_vec(), vec![], |ctx| {
            mutation(ctx)?;
            Err(rejected())
        })
        .await;
    assert_eq!(result, Err(rejected()));
    assert_state(&storage, false).await;
    assert!(matches!(
        storage
            .write(b"mutation".to_vec(), vec![], |ctx| {
                mutation(ctx)?;
                Ok(BadResult)
            })
            .await,
        Err(StorageError::Unavailable)
    ));
    assert_state(&storage, false).await;
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn panicking_callback_is_unknown_and_worker_remains_usable() {
    let (_dir, storage) = open().await;
    seed(&storage).await;
    let result: StorageResult<()> = storage
        .write(b"mutation".to_vec(), vec![], |ctx| {
            mutation(ctx)?;
            panic!("test callback panic");
        })
        .await;
    assert_eq!(result, Err(StorageError::OutcomeUnknown));
    assert_state(&storage, false).await;
    storage
        .write(b"mutation".to_vec(), b"original request".to_vec(), mutation)
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancellation_after_admission_keeps_durable_evidence() {
    let (_dir, storage) = open().await;
    seed(&storage).await;
    let (started, waiting) = oneshot::channel();
    let (release, blocked) = std::sync::mpsc::channel();
    let writer = storage.clone();
    let task = tokio::spawn(async move {
        writer
            .write(
                b"mutation".to_vec(),
                b"original request".to_vec(),
                move |ctx| {
                    started.send(()).unwrap();
                    blocked.recv().unwrap();
                    mutation(ctx)
                },
            )
            .await
    });
    waiting.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    assert_state(&storage, true).await;
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn concurrent_writes_are_serial_and_collision_does_not_run_work() {
    let (_dir, storage) = open().await;
    seed(&storage).await;
    let mut tasks = Vec::new();
    for i in 0..48_u32 {
        let writer = storage.clone();
        tasks.push(tokio::spawn(async move {
            writer
                .write(i.to_le_bytes().to_vec(), vec![], |ctx| {
                    ctx.edit("a", |tx| {
                        let n = tx
                            .get(ROOT, "counter")?
                            .map(|v| v.0.as_i64().unwrap())
                            .unwrap_or(0);
                        tx.put(ROOT, "counter", n + 1)?;
                        Ok(n + 1)
                    })
                })
                .await
                .unwrap()
        }));
    }
    for task in tasks {
        task.await.unwrap();
    }
    storage
        .read(|ctx| {
            assert_eq!(
                ctx.document("a")?.get(ROOT, "counter")?.unwrap().0.as_i64(),
                Some(48)
            );
            Ok(())
        })
        .await
        .unwrap();
    for request in [vec![], b"different".to_vec()] {
        assert_eq!(
            storage
                .write(
                    0_u32.to_le_bytes().to_vec(),
                    request,
                    |_| -> StorageResult<()> {
                        panic!("collision must not execute");
                    }
                )
                .await,
            Err(StorageError::AlreadyRecorded)
        );
    }
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn only_selected_space_is_loaded_and_log_is_append_only() {
    let (_dir, storage) = open().await;
    seed(&storage).await;
    let before = storage
        .run(|c| {
            Ok(c.query_row(
                "SELECT sum(length(bytes)) FROM changes WHERE space = 'a'",
                [],
                |r| r.get::<_, i64>(0),
            )?)
        })
        .await
        .unwrap();
    storage
        .run(|c| {
            c.execute("UPDATE changes SET bytes = x'ff' WHERE space = 'b'", [])?;
            Ok(())
        })
        .await
        .unwrap();
    storage
        .write(b"small-edit".to_vec(), vec![], |ctx| {
            ctx.edit("a", |tx| {
                tx.put(ROOT, "title", "small edit")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    storage
        .run(move |c| {
            let (count, bytes): (i64, i64) = c.query_row(
                "SELECT count(*), sum(length(bytes)) FROM changes WHERE space = 'a'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            assert_eq!(count, 2);
            assert!(bytes > before);
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        storage.read(|ctx| ctx.document("b").map(|_| ())).await,
        Err(StorageError::InvalidDatabase)
    );
    assert_eq!(
        storage
            .write(b"mutation".to_vec(), b"original request".to_vec(), mutation)
            .await,
        Err(StorageError::InvalidDatabase)
    );
    storage
        .read(|ctx| {
            assert_eq!(text(ctx.document("a")?, "title"), "small edit");
            assert!(ctx.recorded(b"mutation")?.is_none());
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn orphan_changes_and_conflicting_values_survive_reopen_and_late_delivery() {
    let (dir, storage) = open().await;
    seed(&storage).await;
    let mut remote = storage
        .read(|ctx| Ok(ctx.document("a")?.fork()))
        .await
        .unwrap();
    let mut tx = remote.transaction();
    tx.put(ROOT, "remote", "dependency").unwrap();
    let first = tx.commit().0.unwrap();
    let missing = remote.get_change_by_hash(&first).unwrap();
    let mut tx = remote.transaction();
    tx.put(ROOT, "remote", "late").unwrap();
    let second = tx.commit().0.unwrap();
    let orphan = remote.get_change_by_hash(&second).unwrap();
    storage
        .write(b"orphan".to_vec(), vec![], move |ctx| {
            ctx.apply_changes("a", vec![orphan])
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    storage
        .read(|ctx| {
            assert!(ctx.document("a")?.get(ROOT, "remote")?.is_none());
            Ok(())
        })
        .await
        .unwrap();
    storage
        .write(b"dependency".to_vec(), vec![], move |ctx| {
            ctx.apply_changes("a", vec![missing.clone(), missing])
        })
        .await
        .unwrap();
    let (mut branch1, mut branch2) = storage
        .read(|ctx| {
            let doc = ctx.document("a")?;
            assert_eq!(text(doc, "remote"), "late");
            Ok((doc.fork(), doc.fork()))
        })
        .await
        .unwrap();
    let mut changes = Vec::new();
    for (doc, value) in [(&mut branch1, "one"), (&mut branch2, "two")] {
        let mut tx = doc.transaction();
        tx.put(ROOT, "title", value).unwrap();
        let hash = tx.commit().0.unwrap();
        changes.push(doc.get_change_by_hash(&hash).unwrap());
    }
    storage
        .write(b"conflict".to_vec(), vec![], move |ctx| {
            ctx.apply_changes("a", changes)
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    storage
        .read(|ctx| {
            assert_eq!(ctx.document("a")?.get_all(ROOT, "title")?.len(), 2);
            assert_eq!(text(ctx.document("b")?, "title"), "before");
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn incompatible_or_foreign_schema_is_rejected_without_modifying_file() {
    for sql in [
        "PRAGMA user_version = 2",
        "PRAGMA user_version = 0",
        "PRAGMA application_id = 7",
        "DROP TABLE peers",
        "CREATE TABLE surprise(data BLOB)",
    ] {
        let (dir, storage) = open().await;
        seed(&storage).await;
        storage.shutdown().await.unwrap();
        let path = dir.path().join("device.db");
        let c = Connection::open(&path).unwrap();
        c.execute_batch(sql).unwrap();
        c.close().unwrap();
        let before = std::fs::read(&path).unwrap();
        let error = Storage::open(&path).await.err().unwrap();
        assert_eq!(
            error,
            if sql.starts_with("PRAGMA user_version") {
                StorageError::UnsupportedSchema
            } else {
                StorageError::InvalidDatabase
            }
        );
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
    let dir = TempDir::new().unwrap();
    for (file, sql) in [
        (
            "foreign.db",
            "CREATE TABLE other(data TEXT); INSERT INTO other VALUES ('preserve')",
        ),
        ("future.db", "PRAGMA user_version = 99"),
    ] {
        let path = dir.path().join(file);
        let c = Connection::open(&path).unwrap();
        c.execute_batch(sql).unwrap();
        c.close().unwrap();
        let before = std::fs::read(&path).unwrap();
        assert!(Storage::open(&path).await.is_err());
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}

#[tokio::test]
async fn busy_database_and_shutdown_never_return_success() {
    let (dir, storage) = open().await;
    seed(&storage).await;
    let blocker = Connection::open(dir.path().join("device.db")).unwrap();
    blocker.execute_batch("BEGIN IMMEDIATE").unwrap();
    assert_eq!(
        storage
            .write(b"mutation".to_vec(), b"original request".to_vec(), mutation)
            .await,
        Err(StorageError::Unavailable)
    );
    blocker.execute_batch("ROLLBACK").unwrap();
    assert_state(&storage, false).await;
    let clone = storage.clone();
    storage.shutdown().await.unwrap();
    assert_eq!(
        clone.write(b"mutation".to_vec(), vec![], mutation).await,
        Err(StorageError::Unavailable)
    );
}

#[tokio::test]
async fn independent_workers_serialize_and_recheck_the_persistence_key() {
    let (dir, first) = open().await;
    seed(&first).await;
    let second = Storage::open(dir.path().join("device.db")).await.unwrap();
    let mut tasks = Vec::new();
    for worker in [first.clone(), second.clone()] {
        tasks.push(tokio::spawn(async move {
            worker
                .write(b"same-key".to_vec(), vec![], |ctx| {
                    ctx.edit("a", |tx| {
                        tx.put(ROOT, "title", "exactly once")?;
                        Ok(())
                    })
                })
                .await
        }));
    }
    let results = [
        tasks.remove(0).await.unwrap(),
        tasks.remove(0).await.unwrap(),
    ];
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| **r == Err(StorageError::AlreadyRecorded))
            .count(),
        1
    );
    first
        .read(|ctx| {
            let graph = ctx.document("a")?;
            assert_eq!(text(graph, "title"), "exactly once");
            assert_eq!(graph.get_changes(&[]).len(), 2);
            Ok(())
        })
        .await
        .unwrap();
    first.shutdown().await.unwrap();
    second.shutdown().await.unwrap();
}

#[tokio::test]
async fn small_edit_keeps_old_change_bytes_and_appends_only_the_delta() {
    let (_dir, storage) = open().await;
    storage
        .write(b"large".to_vec(), vec![], |ctx| {
            ctx.create_document("a")?;
            ctx.edit("a", |tx| {
                tx.put(ROOT, "large", "x".repeat(256 * 1024))?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let (old_hash, old_bytes): (Vec<u8>, Vec<u8>) = storage
        .run(|c| {
            Ok(c.query_row("SELECT hash, bytes FROM changes", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })?)
        })
        .await
        .unwrap();
    storage
        .write(b"small".to_vec(), vec![], |ctx| {
            ctx.edit("a", |tx| {
                tx.put(ROOT, "small", "edit")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    storage
        .run(move |c| {
            let same: Vec<u8> = c.query_row(
                "SELECT bytes FROM changes WHERE space = 'a' AND hash = ?1",
                [&old_hash],
                |r| r.get(0),
            )?;
            assert_eq!(same, old_bytes);
            let delta: i64 = c.query_row(
                "SELECT sum(length(bytes)) FROM changes WHERE hash != ?1",
                [&old_hash],
                |r| r.get(0),
            )?;
            assert!(delta < 1024, "small edits must not save the whole document");
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_peer_progress_poisons_the_write_even_if_error_is_caught() {
    let (_dir, storage) = open().await;
    seed(&storage).await;
    assert_eq!(
        storage
            .write(b"mutation".to_vec(), b"original request".to_vec(), |ctx| {
                mutation(ctx)?;
                let invalid = State {
                    shared_heads: vec![automerge::ChangeHash([7; 32])],
                    ..State::new()
                };
                assert_eq!(
                    ctx.set_peer_state("a", b"peer", &invalid),
                    Err(StorageError::InvalidDatabase)
                );
                Ok("ignored")
            })
            .await,
        Err(StorageError::Unavailable)
    );
    assert_state(&storage, false).await;
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn sqlite_policy_and_core_error_mapping_are_explicit() {
    let (_dir, storage) = open().await;
    storage
        .run(|c| {
            let mode: String = c.pragma_query_value(None, "journal_mode", |r| r.get(0))?;
            let sync: i64 = c.pragma_query_value(None, "synchronous", |r| r.get(0))?;
            let fk: bool = c.pragma_query_value(None, "foreign_keys", |r| r.get(0))?;
            assert_eq!(mode, "wal");
            assert_eq!(sync, 2);
            assert!(fk);
            assert_ne!(
                thread::current().name(),
                Some("sqlite_policy_and_core_error_mapping_are_explicit")
            );
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        CommandError::from(StorageError::OutcomeUnknown).code,
        ErrorCode::OutcomeUnknown
    );
    assert_eq!(CommandError::from(rejected()).code, ErrorCode::StaleState);
    assert_eq!(
        CommandError::from(StorageError::Unavailable).code,
        ErrorCode::Unavailable
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn abrupt_process_exit_recovers_all_or_nothing_at_every_boundary() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (dir, storage) = open().await;
        seed(&storage).await;
        let actor = storage
            .read(|ctx| Ok(ctx.document("a")?.get_actor().clone()))
            .await
            .unwrap();
        storage.shutdown().await.unwrap();
        let path = dir.path().join("device.db");
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "tests::crash_child", "--nocapture"])
            .env("ZEDONE_STORAGE_CRASH_DB", &path)
            .env("ZEDONE_STORAGE_CRASH_STAGE", format!("{stage:?}"))
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(71));
        let storage = Storage::open(&path).await.unwrap();
        assert_state(&storage, stage == Stage::AfterCommit).await;
        storage
            .write(b"after-crash".to_vec(), vec![], move |ctx| {
                assert_eq!(*ctx.read().document("a")?.get_actor(), actor);
                ctx.edit("a", |tx| {
                    tx.put(ROOT, "after-crash", true)?;
                    Ok(())
                })
            })
            .await
            .unwrap();
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn crash_child() {
    let Some(path) = std::env::var_os("ZEDONE_STORAGE_CRASH_DB") else {
        return;
    };
    // Only a parent-created isolated fixture can enable this process exit.
    assert_eq!(Path::new(&path).file_name().unwrap(), "device.db");
    let stage = std::env::var("ZEDONE_STORAGE_CRASH_STAGE").unwrap();
    let storage = Storage::open(path).await.unwrap();
    let _ = storage
        .write_checked(
            b"mutation".to_vec(),
            b"original request".to_vec(),
            mutation,
            move |point| {
                if format!("{point:?}") == stage {
                    std::process::exit(71);
                }
                Ok(())
            },
        )
        .await;
    panic!("crash checkpoint not reached");
}
