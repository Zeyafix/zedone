use super::*;
use crate::{Stage, Storage, StorageError};
use automerge::{ROOT, sync::State, transaction::Transactable};
use rusqlite::{Connection, types::Value};
use tempfile::TempDir;

const PEER: &str = "11111111111111111111111111111111";
const OTHER: &str = "22222222222222222222222222222222";

async fn setup() -> (TempDir, Storage) {
    let dir = TempDir::new().unwrap();
    let db = Storage::open(dir.path().join("device.sqlite"))
        .await
        .unwrap();
    db.transact(|tx| {
        for space in ["a", "b"] {
            tx.create_document(space)?;
            tx.edit(space, |graph| {
                graph.put(ROOT, "title", "saved")?;
                graph.put(ROOT, "history", "previous")?;
                Ok(())
            })?;
            let progress = State {
                shared_heads: tx.read().document(space)?.get_heads(),
                ..State::new()
            };
            tx.set_peer_state(space, PEER.as_bytes(), &progress)?;
        }
        Ok(())
    })
    .await
    .unwrap();
    (dir, db)
}

async fn controls(db: &Storage, space: &str, peer: &str) -> (SpaceSyncControl, SyncSelection) {
    let (space, peer) = (space.to_owned(), peer.to_owned());
    db.read(move |tx| {
        Ok((
            tx.space_sync_control(&space)?,
            tx.sync_selection(&space, &peer)?,
        ))
    })
    .await
    .unwrap()
}

async fn preserved(db: &Storage) -> Vec<(String, Vec<Vec<Value>>)> {
    db.read(|tx| {
        let c = tx.connection();
        let tables = c
            .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' AND name NOT LIKE 'sync_%' ORDER BY name")?
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        tables.into_iter().map(|name| {
            let mut statement = c.prepare(&format!("SELECT * FROM {name} ORDER BY 1, 2"))?;
            let count = statement.column_count();
            let rows = statement.query_map([], |r| (0..count).map(|i| r.get(i)).collect::<Result<Vec<Value>, _>>())?
                .collect::<Result<Vec<_>, _>>()?;
            Ok((name, rows))
        }).collect()
    }).await.unwrap()
}

fn stale() -> StorageError {
    CommandError {
        code: ErrorCode::StaleState,
    }
    .into()
}

#[tokio::test]
async fn defaults_reverse_restart_and_scopes_preserve_every_other_table() {
    let (dir, db) = setup().await;
    let before = preserved(&db).await;
    let (initial, off) = controls(&db, "a", PEER).await;
    assert!(!initial.paused && !off.selected);
    let b = controls(&db, "b", PEER).await;
    let other = controls(&db, "a", OTHER).await.1;
    let on = db
        .transact(move |tx| tx.set_sync_selected("a", PEER, true, off.version))
        .await
        .unwrap();
    let paused = db
        .transact(move |tx| tx.set_space_sync_paused("a", true, initial.version))
        .await
        .unwrap();
    assert_eq!(controls(&db, "a", OTHER).await, (paused.clone(), other));
    assert_eq!(controls(&db, "b", PEER).await, b);
    assert_eq!(controls(&db, "a", PEER).await, (paused.clone(), on.clone()));
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("device.sqlite"))
        .await
        .unwrap();
    assert_eq!(controls(&db, "a", PEER).await, (paused.clone(), on.clone()));
    let off = db
        .transact(move |tx| tx.set_sync_selected("a", PEER, false, on.version))
        .await
        .unwrap();
    assert_eq!(controls(&db, "a", PEER).await.0, paused);
    let on = db
        .transact(move |tx| tx.set_sync_selected("a", PEER, true, off.version))
        .await
        .unwrap();
    assert!(controls(&db, "a", PEER).await.0.paused);
    db.transact(move |tx| {
        tx.set_space_sync_paused("a", false, paused.version)?;
        tx.set_sync_selected("a", PEER, false, on.version)?;
        Ok(())
    })
    .await
    .unwrap();
    assert!(!controls(&db, "a", PEER).await.0.paused);
    assert!(!controls(&db, "a", PEER).await.1.selected);
    assert_eq!(preserved(&db).await, before);
}

#[tokio::test]
async fn aba_tokens_never_revive_and_noops_do_not_advance_them() {
    let (_dir, db) = setup().await;
    let (initial, off) = controls(&db, "a", PEER).await;
    let (initial_noop, off_noop) = (initial.clone(), off.clone());
    db.transact(move |tx| {
        assert_eq!(
            tx.set_space_sync_paused("a", false, initial_noop.version)?,
            initial_noop
        );
        assert_eq!(
            tx.set_sync_selected("a", PEER, false, off_noop.version)?,
            off_noop
        );
        Ok(())
    })
    .await
    .unwrap();
    for value in [true, false] {
        let (pause, select) = controls(&db, "a", PEER).await;
        db.transact(move |tx| {
            tx.set_sync_selected("a", PEER, value, select.version)?;
            tx.set_space_sync_paused("a", value, pause.version)?;
            Ok(())
        })
        .await
        .unwrap();
    }
    assert_ne!(
        controls(&db, "a", PEER).await,
        (initial.clone(), off.clone())
    );
    assert_eq!(
        db.transact(move |tx| tx.set_sync_selected("a", PEER, true, off.version))
            .await,
        Err(stale())
    );
    assert_eq!(
        db.transact(move |tx| tx.set_space_sync_paused("a", true, initial.version))
            .await,
        Err(stale())
    );
    let (pause, select) = controls(&db, "a", PEER).await;
    assert_eq!(
        db.transact(move |tx| tx.set_sync_selected("a", OTHER, true, select.version))
            .await,
        Err(stale())
    );
    assert_eq!(
        db.transact(move |tx| tx.set_space_sync_paused("b", true, pause.version))
            .await,
        Err(stale())
    );
}

#[tokio::test]
async fn every_precommit_failure_rolls_back_controls_graph_progress_and_receipt() {
    let (_dir, db) = setup().await;
    let before = preserved(&db).await;
    let initial = controls(&db, "a", PEER).await;
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
    ] {
        let (pause, select) = initial.clone();
        let result = db
            .transact_checked(
                move |tx| {
                    tx.set_sync_selected("a", PEER, true, select.version)?;
                    tx.set_space_sync_paused("a", true, pause.version)?;
                    tx.edit("a", |graph| {
                        graph.put(ROOT, "title", "uncommitted")?;
                        Ok(())
                    })?;
                    tx.set_peer_state("a", PEER.as_bytes(), &State::new())?;
                    tx.record(
                        b"control",
                        &crate::RecordedResult {
                            request: vec![1],
                            result: vec![2],
                        },
                    )?;
                    Ok(())
                },
                move |at| {
                    if at == stage {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await;
        assert_eq!(result, Err(StorageError::Unavailable));
        assert_eq!(controls(&db, "a", PEER).await, initial);
        assert_eq!(preserved(&db).await, before);
    }
}

#[tokio::test]
async fn caught_sql_and_stale_errors_poison_the_whole_mutation() {
    let (dir, db) = setup().await;
    let initial = controls(&db, "a", PEER).await;
    let c = Connection::open(dir.path().join("device.sqlite")).unwrap();
    c.execute_batch("CREATE TRIGGER reject_pause BEFORE INSERT ON sync_spaces BEGIN SELECT RAISE(ABORT, 'fault'); END;").unwrap();
    let (pause, select) = initial.clone();
    assert_eq!(
        db.transact(move |tx| {
            tx.set_sync_selected("a", PEER, true, select.version)?;
            assert!(tx.set_space_sync_paused("a", true, pause.version).is_err());
            Ok(())
        })
        .await,
        Err(StorageError::Unavailable)
    );
    c.execute_batch("DROP TRIGGER reject_pause;").unwrap();
    assert_eq!(controls(&db, "a", PEER).await, initial);
    let (_, select) = initial.clone();
    assert_eq!(
        db.transact(move |tx| {
            tx.set_sync_selected("a", PEER, true, select.version)?;
            assert_eq!(
                tx.set_sync_selected("a", PEER, false, select.version),
                Err(stale())
            );
            Ok(())
        })
        .await,
        Err(StorageError::Unavailable)
    );
    assert_eq!(controls(&db, "a", PEER).await, initial);
}

#[tokio::test]
async fn unknown_committed_result_survives_restart_without_implicit_resume() {
    let (dir, db) = setup().await;
    let (pause, select) = controls(&db, "a", PEER).await;
    let old = pause.version;
    assert_eq!(
        db.transact_checked(
            move |tx| {
                tx.set_sync_selected("a", PEER, true, select.version)?;
                tx.set_space_sync_paused("a", true, pause.version)?;
                Ok(())
            },
            |stage| if stage == Stage::AfterCommit {
                Err(StorageError::Unavailable)
            } else {
                Ok(())
            }
        )
        .await,
        Err(StorageError::OutcomeUnknown)
    );
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("device.sqlite"))
        .await
        .unwrap();
    let (pause, select) = controls(&db, "a", PEER).await;
    assert!(pause.paused && select.selected);
    assert_eq!(
        db.transact(move |tx| tx.set_space_sync_paused("a", false, old))
            .await,
        Err(stale())
    );
    assert_eq!(controls(&db, "a", PEER).await, (pause, select));
}

#[tokio::test]
async fn two_workers_cannot_overwrite_the_same_read_intention() {
    let (dir, db) = setup().await;
    let other = Storage::open(dir.path().join("device.sqlite"))
        .await
        .unwrap();
    let (_, initial) = controls(&db, "a", PEER).await;
    let expected = initial.version;
    let (one, two) = tokio::join!(
        db.transact(move |tx| tx.set_sync_selected("a", PEER, true, expected)),
        other.transact(move |tx| tx.set_sync_selected("a", PEER, true, expected))
    );
    assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
    assert_eq!(one.err().or(two.err()), Some(stale()));
    assert!(controls(&db, "a", PEER).await.1.selected);
}

#[tokio::test]
async fn invalid_pairings_missing_spaces_and_overflow_never_commit_partial_policy() {
    let (dir, db) = setup().await;
    let initial = controls(&db, "a", PEER).await;
    for peer in [
        "",
        "peer",
        "00000000000000000000000000000000",
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    ] {
        assert_eq!(
            db.read(move |tx| tx.sync_selection("a", peer)).await,
            Err(CommandError {
                code: ErrorCode::InvalidRequest
            }
            .into())
        );
    }
    assert!(
        !db.read(|tx| tx.space_sync_control("missing"))
            .await
            .unwrap()
            .paused
    );
    let c = Connection::open(dir.path().join("device.sqlite")).unwrap();
    c.execute("INSERT INTO sync_spaces VALUES ('a', 1, ?1)", [i64::MAX])
        .unwrap();
    let (pause, select) = controls(&db, "a", PEER).await;
    assert_eq!(
        db.transact(move |tx| {
            tx.set_sync_selected("a", PEER, true, select.version)?;
            tx.set_space_sync_paused("a", false, pause.version)?;
            Ok(())
        })
        .await,
        Err(StorageError::Unavailable)
    );
    assert_eq!(controls(&db, "a", PEER).await.1, initial.1);
    assert!(controls(&db, "a", PEER).await.0.paused);
}

#[tokio::test]
async fn process_exit_before_or_after_commit_keeps_one_complete_policy() {
    for stage in [Stage::BeforeCommit, Stage::AfterCommit] {
        let (dir, db) = setup().await;
        let before = preserved(&db).await;
        db.shutdown().await.unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "sync_control::tests::crash_child", "--nocapture"])
            .env("ZEDONE_SYNC_CRASH_DB", dir.path().join("device.sqlite"))
            .env("ZEDONE_SYNC_CRASH_STAGE", format!("{stage:?}"))
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        let db = Storage::open(dir.path().join("device.sqlite"))
            .await
            .unwrap();
        let (pause, select) = controls(&db, "a", PEER).await;
        assert_eq!(pause.paused, stage == Stage::AfterCommit);
        assert_eq!(select.selected, stage == Stage::AfterCommit);
        assert_eq!(preserved(&db).await, before);
        db.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn crash_child() {
    let Some(path) = std::env::var_os("ZEDONE_SYNC_CRASH_DB") else {
        return;
    };
    let stage = std::env::var("ZEDONE_SYNC_CRASH_STAGE").unwrap();
    let db = Storage::open(std::path::PathBuf::from(path)).await.unwrap();
    let (pause, select) = controls(&db, "a", PEER).await;
    db.transact_checked(
        move |tx| {
            tx.set_sync_selected("a", PEER, true, select.version)?;
            tx.set_space_sync_paused("a", true, pause.version)?;
            Ok(())
        },
        move |here| {
            if format!("{here:?}") == stage {
                std::process::exit(73);
            }
            Ok(())
        },
    )
    .await
    .unwrap();
    panic!("checkpoint not reached");
}
