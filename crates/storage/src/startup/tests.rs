use std::{fs, path::Path, process::Command, task::Poll};

use automerge::{ReadDoc, transaction::Transactable};
use rusqlite::Transaction;
use tempfile::TempDir;

use super::*;
use crate::migration_backup::tests::{LIMITS, files};
use crate::{
    migration_backup,
    schema::{ArchiveAdapter, Schema, V1_ARCHIVE},
};

const V1: &str = include_str!("../schema/v1.sql");
const V2: &str = concat!(
    include_str!("../schema/v1.sql"),
    "CREATE TABLE migration_a (value TEXT NOT NULL) STRICT;"
);
const V3: &str = concat!(
    include_str!("../schema/v1.sql"),
    "CREATE TABLE migration_a (value TEXT NOT NULL) STRICT;CREATE TABLE migration_b (value TEXT NOT NULL) STRICT;"
);
const V2_ARCHIVE: ArchiveAdapter = ArchiveAdapter {
    spaces: |c, limits| {
        let value: String = c.query_row("SELECT value FROM migration_a", [], |r| r.get(0))?;
        if value != "first transition" {
            return Err(StorageError::InvalidDatabase);
        }
        migration_backup::spaces_v1(c, limits)
    },
    capture: |c, metadata, limits| {
        let _: String = c.query_row("SELECT value FROM migration_a", [], |r| r.get(0))?;
        crate::snapshot::capture(c, metadata, limits)
    },
};
const GOOD: Registry = Registry(&[
    Schema {
        version: 1,
        sql: V1,
        upgrade: None,
        archive: V1_ARCHIVE,
        validate: None,
    },
    Schema {
        version: 2,
        sql: V2,
        upgrade: Some(second),
        archive: V2_ARCHIVE,
        validate: None,
    },
    Schema {
        version: 3,
        sql: V3,
        upgrade: Some(third),
        archive: V1_ARCHIVE,
        validate: None,
    },
]);
const FAIL: Registry = Registry(&[
    Schema {
        version: 1,
        sql: V1,
        upgrade: None,
        archive: V1_ARCHIVE,
        validate: None,
    },
    Schema {
        version: 2,
        sql: V2,
        upgrade: Some(second),
        archive: V2_ARCHIVE,
        validate: None,
    },
    Schema {
        version: 3,
        sql: V3,
        upgrade: Some(failing_third),
        archive: V1_ARCHIVE,
        validate: None,
    },
]);

fn second(tx: &Transaction<'_>) -> StorageResult<()> {
    tx.execute_batch(
        "CREATE TABLE migration_a (value TEXT NOT NULL) STRICT;
        INSERT INTO migration_a VALUES ('first transition');
        INSERT INTO command_results VALUES (x'7f', x'01', x'02');",
    )?;
    Ok(())
}

fn third(tx: &Transaction<'_>) -> StorageResult<()> {
    tx.execute_batch(
        "CREATE TABLE migration_b (value TEXT NOT NULL) STRICT;
        INSERT INTO migration_b VALUES ('second transition');
        UPDATE command_results SET result = x'03' WHERE key = x'7f';",
    )?;
    Ok(())
}

fn failing_third(tx: &Transaction<'_>) -> StorageResult<()> {
    third(tx)?;
    Err(StorageError::Unavailable)
}

fn fixture() -> (TempDir, PathBuf, MigrationOptions) {
    let root = TempDir::new().unwrap();
    let path = root.path().join("device.sqlite");
    fs::write(
        &path,
        include_bytes!("../../tests/fixtures/schema-v1.sqlite"),
    )
    .unwrap();
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO peers VALUES (?1, ?2, ?3)",
            rusqlite::params![
                "Историческое ../ 空間",
                b"historical-peer",
                automerge::sync::State::new().encode()
            ],
        )
        .unwrap();
    drop(connection);
    let folder = root.path().join("backups");
    fs::create_dir(&folder).unwrap();
    let options = MigrationOptions {
        folder,
        created_at_unix_ms: 10,
        limits: LIMITS,
    };
    (root, path, options)
}

fn controller(path: &Path, registry: Registry) -> Startup {
    let mut startup = Startup::new(path);
    startup.registry = registry;
    startup
}

fn version(connection: &Connection) -> i64 {
    connection
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .unwrap()
}

fn candidates(folder: &Path) -> Vec<PathBuf> {
    let mut paths: Vec<_> = fs::read_dir(folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_dir() && p.join("manifest.json").is_file())
        .collect();
    paths.sort();
    paths
}

fn assert_original(connection: &Connection) {
    assert_eq!(version(connection), 1);
    schema::REGISTRY.validate(connection).unwrap();
    let count: i64 = connection
        .query_row("SELECT count(*) FROM command_results", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        count, 1,
        "failed chain must roll back service records as well"
    );
}

#[tokio::test]
async fn direct_update_from_every_known_schema_keeps_data_services_and_archives() {
    for source in 1..=3 {
        let (_root, path, options) = fixture();
        if source > 1 {
            let mut connection = Connection::open(&path).unwrap();
            let tx = connection.transaction().unwrap();
            second(&tx).unwrap();
            if source == 3 {
                third(&tx).unwrap();
            }
            tx.pragma_update(None, "user_version", source).unwrap();
            tx.commit().unwrap();
        }
        let mut startup = controller(&path, GOOD);
        let storage = startup.automatic(Some(options.clone())).await.unwrap();
        assert_eq!(startup.state(), StartupState::Ready);
        storage
            .read(|ctx| {
                assert_eq!(
                    ctx.document("Историческое ../ 空間")?
                        .get_actor()
                        .to_bytes(),
                    &[0x31]
                );
                let old = ctx.recorded(b"old-command")?.unwrap();
                assert_eq!(old.request, b"original request");
                assert_eq!(old.result, b"\"original result\"");
                Ok(())
            })
            .await
            .unwrap();
        let archives = candidates(&options.folder);
        assert_eq!(archives.len(), usize::from(source < 3));
        if source < 3 {
            let copy = migration_backup::readonly(&archives[0].join("database.sqlite")).unwrap();
            assert_eq!(version(&copy), source);
            migration_backup::verify_with(&archives[0], LIMITS, GOOD).unwrap();
        }
        storage
            .run(|c| {
                GOOD.target(c)?;
                let state: Vec<u8> = c.query_row(
                    "SELECT state FROM peers WHERE peer=?1",
                    [b"historical-peer"],
                    |r| r.get(0),
                )?;
                assert_eq!(state, automerge::sync::State::new().encode());
                let value: Vec<u8> = c.query_row(
                    "SELECT result FROM command_results WHERE key=x'7f'",
                    [],
                    |r| r.get(0),
                )?;
                assert_eq!(value, [3]);
                Ok(())
            })
            .await
            .unwrap();
        let before: Vec<_> = archives.iter().flat_map(|p| files(p)).collect();
        storage
            .write(b"edit".to_vec(), vec![], |ctx| {
                ctx.edit("Историческое ../ 空間", |tx| {
                    tx.put(automerge::ROOT, "title", "edit")?;
                    Ok(())
                })
            })
            .await
            .unwrap();
        storage.shutdown().await.unwrap();
        let mut restarted = controller(&path, GOOD);
        let storage = restarted.automatic(Some(options.clone())).await.unwrap();
        storage
            .write(b"reverse".to_vec(), vec![], |ctx| {
                ctx.edit("Историческое ../ 空間", |tx| {
                    tx.delete(automerge::ROOT, "title")?;
                    Ok(())
                })
            })
            .await
            .unwrap();
        storage
            .read(|ctx| {
                let graph = ctx.document("Историческое ../ 空間")?;
                assert!(graph.get(automerge::ROOT, "title")?.is_none());
                assert_eq!(graph.get_changes(&[]).len(), 2);
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(
            before,
            archives.iter().flat_map(|p| files(p)).collect::<Vec<_>>()
        );
        assert_eq!(
            candidates(&options.folder),
            archives,
            "current version must not recapture"
        );
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn failed_second_transition_rolls_back_everything_and_explicit_retry_captures_again() {
    let (_root, path, mut options) = fixture();
    let mut startup = controller(&path, FAIL);
    assert_eq!(
        startup.automatic(Some(options.clone())).await.err(),
        Some(StorageError::Unavailable)
    );
    assert_eq!(
        startup.state(),
        StartupState::Failed(StorageError::Unavailable)
    );
    assert_original(&Connection::open(&path).unwrap());
    let old = candidates(&options.folder);
    assert_eq!(old.len(), 1);
    let before = files(&old[0]);
    assert_eq!(
        startup.automatic(Some(options.clone())).await.err(),
        Some(StorageError::RetryRequired)
    );
    assert_eq!(old, candidates(&options.folder));
    // A retry uses new budgets and rechecks its source, rather than reusing a candidate.
    startup.registry = GOOD;
    options.limits.max_database_bytes = 1;
    assert!(startup.retry(Some(options.clone())).await.is_err());
    assert_original(&Connection::open(&path).unwrap());
    let connection = Connection::open(&path).unwrap();
    connection
        .execute(
            "INSERT INTO command_results VALUES (x'77', x'01', x'02')",
            [],
        )
        .unwrap();
    drop(connection);
    options.limits = LIMITS;
    options.created_at_unix_ms = 20;
    let storage = startup.retry(Some(options.clone())).await.unwrap();
    let new = candidates(&options.folder);
    assert_eq!(new.len(), 2, "one orphan plus a new successful candidate");
    assert_eq!(
        before,
        files(&old[0]),
        "failure's candidate is never silently deleted"
    );
    let active = new.iter().find(|p| **p != old[0]).unwrap();
    let copy = migration_backup::readonly(&active.join("database.sqlite")).unwrap();
    let count: i64 = copy
        .query_row(
            "SELECT count(*) FROM command_results WHERE key=x'77'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 1, "retry captures the new source state");
    drop(copy);
    let snapshot = crate::snapshot::SpaceSnapshot::decode(
        &fs::read(active.join("space-0.bin")).unwrap(),
        LIMITS.snapshot,
    )
    .unwrap();
    assert_eq!(snapshot.metadata().created_at_unix_ms, 20);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn automatic_retry_requires_a_new_run_and_rechecks_changed_database() {
    let (_root, path, options) = fixture();
    let mut first = controller(&path, FAIL);
    assert!(first.automatic(Some(options.clone())).await.is_err());
    let connection = Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    drop(connection);
    assert_eq!(
        first.retry(Some(options.clone())).await.err(),
        Some(StorageError::UnsupportedSchema)
    );
    let connection = Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", 1).unwrap();
    drop(connection);
    let mut next_run = controller(&path, GOOD);
    let storage = next_run.automatic(Some(options)).await.unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn missing_folder_no_options_and_unverified_source_never_start_migration() {
    let (root, path, options) = fixture();
    let original = fs::read(&path).unwrap();
    let mut startup = controller(&path, GOOD);
    assert!(startup.automatic(None).await.is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    let mut unavailable = options.clone();
    unavailable.folder = root.path().join("missing");
    assert!(startup.retry(Some(unavailable)).await.is_err());
    assert!(!root.path().join("missing").exists());
    assert!(candidates(&options.folder).is_empty());
    let connection = Connection::open(&path).unwrap();
    connection.execute_batch("DROP TABLE peers").unwrap();
    drop(connection);
    assert_eq!(
        startup.retry(Some(options.clone())).await.err(),
        Some(StorageError::InvalidDatabase)
    );
    assert!(candidates(&options.folder).is_empty());
}

#[test]
fn entire_chain_excludes_writers_and_failure_at_each_precommit_boundary_rolls_back() {
    for boundary in [
        publication::Point::CopyWritten,
        publication::Point::AfterCapture,
        publication::Point::BeforeCommit,
    ] {
        let (_root, path, options) = fixture();
        let mut connection = Connection::open(&path).unwrap();
        let error = initialize(&mut connection, Some(&options), GOOD, |point| {
            if matches!(
                point,
                publication::Point::AfterCapture | publication::Point::BeforeCommit
            ) {
                let other = Connection::open(&path).unwrap();
                other.busy_timeout(Duration::ZERO).unwrap();
                assert!(other.execute_batch("BEGIN IMMEDIATE").is_err());
            }
            if point == boundary {
                Err(StorageError::Unavailable)
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error, StorageError::Unavailable);
        assert_original(&connection);
        assert!(!options.folder.join("migration-current.json").exists());
    }
}

#[tokio::test]
async fn committed_migration_with_missing_or_corrupt_backup_blocks_every_storage_entry() {
    for corrupt in [false, true] {
        let (_root, path, options) = fixture();
        let mut connection = Connection::open(&path).unwrap();
        let error = initialize(&mut connection, Some(&options), GOOD, |p| {
            if p == publication::Point::AfterCommit {
                Err(StorageError::Unavailable)
            } else {
                Ok(())
            }
        })
        .unwrap_err();
        assert_eq!(error, StorageError::OutcomeUnknown);
        assert_eq!(version(&connection), 3);
        drop(connection);
        let archive = candidates(&options.folder).pop().unwrap();
        let saved = fs::read(archive.join("database.sqlite")).unwrap();
        if corrupt {
            fs::write(archive.join("database.sqlite"), b"corrupt").unwrap();
        } else {
            fs::remove_file(archive.join("database.sqlite")).unwrap();
        }
        let mut startup = controller(&path, GOOD);
        assert!(startup.automatic(Some(options.clone())).await.is_err());
        assert!(!options.folder.join("migration-current.json").exists());
        assert!(startup.retry(None).await.is_err());
        fs::write(archive.join("database.sqlite"), saved).unwrap();
        let storage = startup.retry(Some(options.clone())).await.unwrap();
        assert_eq!(candidates(&options.folder), [archive]);
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn legacy_open_cannot_bypass_commit_witness_reconciliation() {
    let (_root, path, options) = fixture();
    let storage = Startup::new(&path)
        .automatic(Some(options.clone()))
        .await
        .unwrap();
    storage
        .with_migration_backup(
            &options.folder,
            0,
            LIMITS,
            |_| Ok(()),
            |c| schema::REGISTRY.target(c),
        )
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    assert!(Storage::open(&path).await.is_err());
    let mut startup = Startup::new(&path);
    let storage = startup.automatic(Some(options)).await.unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelled_wait_retains_one_attempt_and_retry_waits_for_its_result() {
    let (_root, path, options) = fixture();
    let mut startup = controller(&path, GOOD);
    // Hold the writer lock while the admitted worker is trying to initialize.
    let connection = Connection::open(&path).unwrap();
    connection.execute_batch("BEGIN IMMEDIATE").unwrap();
    let mut attempt = Box::pin(startup.automatic(Some(options.clone())));
    std::future::poll_fn(|cx| {
        assert!(attempt.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    drop(attempt);
    assert_eq!(startup.state(), StartupState::Running);
    assert_eq!(
        startup.automatic(Some(options.clone())).await.err(),
        Some(StorageError::RetryRequired)
    );
    connection.execute_batch("ROLLBACK").unwrap();
    // Changed retry options cannot replace the still-running attempt.
    let storage = startup.retry(None).await.unwrap();
    assert_eq!(candidates(&options.folder).len(), 1);
    storage.shutdown().await.unwrap();
}

#[test]
fn migration_crash_child() {
    let Ok(root) = std::env::var("ZEDONE_MIGRATION_CRASH_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let point = std::env::var("ZEDONE_MIGRATION_CRASH_POINT").unwrap();
    let options = MigrationOptions {
        folder: root.join("backups"),
        created_at_unix_ms: 10,
        limits: LIMITS,
    };
    let mut connection = Connection::open(root.join("device.sqlite")).unwrap();
    initialize(&mut connection, Some(&options), GOOD, |p| {
        if format!("{p:?}") == point {
            std::process::exit(73);
        }
        Ok(())
    })
    .unwrap();
    panic!("crash point not reached");
}

#[tokio::test]
async fn process_crash_restarts_at_source_or_target_and_never_commits_intermediate_version() {
    for point in [
        "CopyWritten",
        "AfterCapture",
        "BeforeCommit",
        "AfterCommit",
        "BeforePublish",
        "AfterPublish",
    ] {
        let (root, path, options) = fixture();
        let output = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "startup::tests::migration_crash_child",
                "--nocapture",
            ])
            .env("ZEDONE_MIGRATION_CRASH_ROOT", root.path())
            .env("ZEDONE_MIGRATION_CRASH_POINT", point)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(73), "{point}: {output:?}");
        let committed = matches!(point, "AfterCommit" | "BeforePublish" | "AfterPublish");
        // May need hot-journal recovery after precommit crash; no ordinary handle
        // may perform that write merely to identify the schema.
        if !committed && schema::preflight_with(&path, GOOD).is_err() {
            let mut startup = controller(&path, GOOD);
            assert!(startup.automatic(Some(options.clone())).await.is_err());
            // Test-only external recovery of our synthetic fixture. Production
            // does NOT do this: explicit service recovery belongs to P07.09.
        }
        let connection = Connection::open(&path).unwrap();
        assert_eq!(
            version(&connection),
            if committed { 3 } else { 1 },
            "{point}"
        );
        GOOD.validate(&connection).unwrap();
        drop(connection);
        let before = candidates(&options.folder);
        let mut startup = controller(&path, GOOD);
        let storage = startup.automatic(Some(options.clone())).await.unwrap();
        assert_eq!(version(&Connection::open(&path).unwrap()), 3);
        if committed {
            assert_eq!(candidates(&options.folder), before, "no second migration");
        }
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn historical_adapter_failure_blocks_upgrade_without_publishing_an_invalid_backup() {
    let (_root, path, options) = fixture();
    let mut connection = Connection::open(&path).unwrap();
    let tx = connection.transaction().unwrap();
    second(&tx).unwrap();
    tx.pragma_update(None, "user_version", 2).unwrap();
    tx.commit().unwrap();
    connection
        .execute("UPDATE migration_a SET value='unreadable old metadata'", [])
        .unwrap();
    drop(connection);
    let mut startup = controller(&path, GOOD);
    assert_eq!(
        startup.automatic(Some(options.clone())).await.err(),
        Some(StorageError::InvalidDatabase)
    );
    assert_eq!(version(&Connection::open(&path).unwrap()), 2);
    assert!(!options.folder.join("migration-current.json").exists());
    let connection = Connection::open(&path).unwrap();
    connection
        .execute("UPDATE migration_a SET value='first transition'", [])
        .unwrap();
    drop(connection);
    let storage = startup.retry(Some(options)).await.unwrap();
    storage.shutdown().await.unwrap();
}

const THROUGH_V2: Registry = Registry(&[
    Schema {
        version: 1,
        sql: V1,
        upgrade: None,
        archive: V1_ARCHIVE,
        validate: None,
    },
    Schema {
        version: 2,
        sql: V2,
        upgrade: Some(second),
        archive: V2_ARCHIVE,
        validate: None,
    },
]);

#[tokio::test]
async fn failed_next_upgrade_keeps_last_successful_backup_then_success_replaces_it() {
    let (_root, path, options) = fixture();
    let mut first = controller(&path, THROUGH_V2);
    let storage = first.automatic(Some(options.clone())).await.unwrap();
    storage.shutdown().await.unwrap();
    let old = candidates(&options.folder).pop().unwrap();
    let before = files(&old);
    let mut next = controller(&path, FAIL);
    assert!(next.automatic(Some(options.clone())).await.is_err());
    assert_eq!(version(&Connection::open(&path).unwrap()), 2);
    assert_eq!(before, files(&old));
    let orphan = candidates(&options.folder)
        .into_iter()
        .find(|p| *p != old)
        .unwrap();
    let orphan_bytes = files(&orphan);
    next.registry = GOOD;
    let storage = next.retry(Some(options.clone())).await.unwrap();
    assert!(
        !old.exists(),
        "only successful replacement cleans the previous reference"
    );
    assert_eq!(
        files(&orphan),
        orphan_bytes,
        "failed candidate remains an explicit orphan"
    );
    let active = candidates(&options.folder)
        .into_iter()
        .find(|p| *p != orphan)
        .unwrap();
    let copy = migration_backup::readonly(&active.join("database.sqlite")).unwrap();
    assert_eq!(version(&copy), 2);
    storage.shutdown().await.unwrap();
}

#[test]
fn real_commit_failure_rolls_back_the_entire_chain_and_preserves_original() {
    let (_root, path, options) = fixture();
    let mut c = Connection::open(&path).unwrap();
    c.pragma_update(None, "foreign_keys", true).unwrap();
    // The shared commit path must encounter the deferred violation at COMMIT;
    // this fault-only validator deliberately admits it before then.
    let error = publication::execute_with(&mut c, &options.folder, 10, LIMITS, GOOD, false,
        |tx| {
            GOOD.migrate(tx)?;
            tx.execute_batch("PRAGMA defer_foreign_keys = ON; INSERT INTO peers VALUES ('missing', x'01', x'02')")?;
            Ok(())
        }, |_| Ok(()), |_| Ok(())).unwrap_err();
    assert_eq!(error, StorageError::OutcomeUnknown);
    assert_original(&c);
    assert!(!options.folder.join("migration-current.json").exists());
}

#[test]
fn released_registry_keeps_every_transition_and_archive_adapter() {
    let registry = schema::REGISTRY;
    assert!(!registry.0.is_empty());
    assert!(registry.0[0].upgrade.is_none());
    for pair in registry.0.windows(2) {
        assert!(pair[0].version < pair[1].version);
        assert!(pair[1].upgrade.is_some());
    }
}

#[tokio::test]
async fn successful_callback_with_wrong_target_layout_still_rolls_back_the_chain() {
    const WRONG: Registry = Registry(&[
        Schema {
            version: 1,
            sql: V1,
            upgrade: None,
            archive: V1_ARCHIVE,
            validate: None,
        },
        Schema {
            version: 2,
            sql: V2,
            upgrade: Some(second),
            archive: V2_ARCHIVE,
            validate: None,
        },
        Schema {
            version: 3,
            sql: V3,
            upgrade: Some(|tx| {
                // Claims success after a partial service mutation, without creating b.
                tx.execute("DELETE FROM command_results", [])?;
                Ok(())
            }),
            archive: V1_ARCHIVE,
            validate: None,
        },
    ]);
    let (_root, path, options) = fixture();
    let mut startup = controller(&path, WRONG);
    assert_eq!(
        startup.automatic(Some(options.clone())).await.err(),
        Some(StorageError::InvalidDatabase)
    );
    assert_original(&Connection::open(&path).unwrap());
    assert!(!options.folder.join("migration-current.json").exists());
}
