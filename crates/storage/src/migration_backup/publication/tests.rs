use std::{
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
        mpsc,
    },
};

use super::*;
use crate::migration_backup::tests::{LIMITS, files, fixture};

fn apply(tx: &Transaction<'_>) -> StorageResult<()> {
    tx.execute(
        "INSERT INTO command_results VALUES (x'01', x'02', x'03')",
        [],
    )?;
    Ok(())
}

fn target(connection: &Connection) -> StorageResult<()> {
    validate_database(connection)?;
    let value: Vec<u8> = connection.query_row(
        "SELECT result FROM command_results WHERE key = x'01'",
        [],
        |r| r.get(0),
    )?;
    if value != [3] {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}

fn mutated(connection: &Connection) -> bool {
    connection
        .query_row(
            "SELECT count(*) FROM command_results WHERE key = x'01'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap()
        == 1
}

async fn open(path: impl Into<PathBuf>) -> StorageResult<Storage> {
    let path = path.into();
    let folder = path.parent().unwrap().join("backups");
    crate::startup::Startup::new(path)
        .automatic(Some(crate::startup::MigrationOptions {
            folder,
            created_at_unix_ms: 0,
            limits: LIMITS,
        }))
        .await
}

async fn initial(storage: &Storage, folder: &Path) -> PublishedBackup {
    storage
        .with_migration_backup(folder, 1, LIMITS, |_| Ok(()), validate_database)
        .await
        .unwrap()
}

#[test]
fn folder_lock_is_released_even_while_a_duplicate_descriptor_survives() {
    let folder = TempDir::new().unwrap();
    let held = lock(folder.path()).unwrap();
    // A concurrent fork can inherit this same open file description until exec.
    let duplicate = held.0.try_clone().unwrap();
    assert!(lock(folder.path()).is_err());
    drop(held);
    let next = lock(folder.path()).expect("the completed operation must release its lock");
    assert!(lock(folder.path()).is_err());
    drop(duplicate);
    assert!(
        lock(folder.path()).is_err(),
        "the successor still owns its lock"
    );
    drop(next);
    assert!(lock(folder.path()).is_ok());
}

#[tokio::test]
async fn successful_replacement_is_complete_and_reverse_edit_keeps_backup() {
    let (root, storage, folder) = fixture().await;
    let first = initial(&storage, &folder).await;
    let copy = readonly(&first.path.join("database.sqlite")).unwrap();
    assert!(!mutated(&copy));
    drop(copy);
    let result = storage
        .with_migration_backup(&folder, 2, LIMITS, apply, target)
        .await
        .unwrap();
    assert!(!result.cleanup_pending);
    assert!(!first.path.exists());
    assert_ne!(first.path, result.path);
    verify(&result.path, LIMITS).unwrap();
    let copy = readonly(&result.path.join("database.sqlite")).unwrap();
    assert!(!mutated(&copy), "copy is strictly pre-operation");
    drop(copy);
    let before = files(&result.path);
    storage
        .run(|connection| {
            target(connection)?;
            connection.execute("DELETE FROM command_results WHERE key = x'01'", [])?;
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let reopened = open(root.path().join("device.sqlite")).await.unwrap();
    let recovered = reopened
        .recover_migration_backup(&folder, LIMITS, validate_database)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(recovered.path, result.path);
    assert_eq!(before, files(&result.path));
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn errors_at_each_boundary_preserve_old_until_confirmed_publication() {
    for point in [
        Point::CopyWritten,
        Point::AfterCapture,
        Point::BeforeCommit,
        Point::AfterCommit,
        Point::BeforePublish,
        Point::AfterPublish,
        Point::BeforeCleanup,
    ] {
        let (_root, storage, folder) = fixture().await;
        let old = initial(&storage, &folder).await;
        let before = files(&old.path);
        let path = folder.clone();
        let reached = Arc::new(AtomicUsize::new(0));
        let observed = reached.clone();
        let result = storage
            .run(move |connection| {
                execute(connection, &path, 2, LIMITS, apply, target, |stage| {
                    if stage == point {
                        observed.fetch_add(1, Ordering::SeqCst);
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                })
            })
            .await;
        assert_eq!(
            reached.load(Ordering::SeqCst),
            1,
            "requested fault {point:?} must be reached; result: {result:?}"
        );
        if point == Point::BeforeCleanup {
            assert!(result.unwrap().cleanup_pending);
        } else {
            assert_eq!(
                result.unwrap_err(),
                if matches!(
                    point,
                    Point::AfterCommit | Point::BeforePublish | Point::AfterPublish
                ) {
                    StorageError::OutcomeUnknown
                } else {
                    StorageError::Unavailable
                },
                "fault at {point:?}"
            );
        }
        assert_eq!(before, files(&old.path));
        let after_commit = matches!(
            point,
            Point::AfterCommit | Point::BeforePublish | Point::AfterPublish | Point::BeforeCleanup
        );
        storage
            .run(move |c| {
                assert_eq!(mutated(c), after_commit);
                Ok(())
            })
            .await
            .unwrap();
        let recovered = storage
            .recover_migration_backup(&folder, LIMITS, move |c| {
                if after_commit {
                    target(c)
                } else {
                    validate_database(c)
                }
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(recovered.path == old.path, !after_commit);
        assert_eq!(old.path.exists(), !after_commit);
        assert!(!recovered.cleanup_pending);
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn apply_validation_and_real_commit_failure_roll_back_without_replacing() {
    for failure in 0..4 {
        let (_root, storage, folder) = fixture().await;
        let old = initial(&storage, &folder).await;
        let before = files(&old.path);
        let result = storage.with_migration_backup(&folder, 2, LIMITS, move |tx| {
            apply(tx)?;
            match failure {
                0 => Err(StorageError::Unavailable),
                1 => Ok(()),
                2 => { tx.execute_batch("PRAGMA defer_foreign_keys = ON; INSERT INTO peers VALUES ('missing', x'01', x'02');")?; Ok(()) },
                _ => { tx.execute("DELETE FROM changes", [])?; Ok(()) },
            }
        }, move |c| {
            // The deferred-FK case must reach SQLite COMMIT itself.
            if failure == 1 || failure == 3 { Err(StorageError::InvalidDatabase) } else if failure == 2 { Ok(()) } else { target(c) }
        }).await;
        assert!(result.is_err());
        if failure == 2 {
            assert_eq!(result.unwrap_err(), StorageError::OutcomeUnknown);
        }
        assert_eq!(before, files(&old.path));
        assert_eq!(
            index(&folder).unwrap().unwrap().name,
            old.path.file_name().unwrap().to_str().unwrap()
        );
        storage
            .run(|c| {
                assert!(!mutated(c));
                validate_database(c)
            })
            .await
            .unwrap();
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn postcommit_validation_failure_requires_recovery_and_never_repeats_mutation() {
    let (_root, storage, folder) = fixture().await;
    let old = initial(&storage, &folder).await;
    let count = Arc::new(AtomicUsize::new(0));
    let attempts = count.clone();
    let result = storage
        .with_migration_backup(&folder, 2, LIMITS, apply, move |c| {
            target(c)?;
            if attempts.fetch_add(1, Ordering::SeqCst) >= 2 {
                Err(StorageError::InvalidDatabase)
            } else {
                Ok(())
            }
        })
        .await;
    assert_eq!(result.unwrap_err(), StorageError::OutcomeUnknown);
    assert_eq!(count.load(Ordering::SeqCst), 3);
    assert!(old.path.exists());
    assert!(
        storage
            .with_migration_backup(
                &folder,
                3,
                LIMITS,
                |_| panic!("pending must not run"),
                validate_database
            )
            .await
            .is_err()
    );
    assert!(
        storage
            .recover_migration_backup(&folder, LIMITS, |_| Err(StorageError::InvalidDatabase))
            .await
            .is_err()
    );
    let result = storage
        .recover_migration_backup(&folder, LIMITS, target)
        .await
        .unwrap()
        .unwrap();
    assert_ne!(result.path, old.path);
    assert!(!old.path.exists());
    storage.run(|c| target(c)).await.unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn real_publication_failure_can_be_recovered_without_another_operation() {
    let (_root, storage, folder) = fixture().await;
    let old = initial(&storage, &folder).await;
    let old_index = fs::read(folder.join(INDEX)).unwrap();
    let path = folder.clone();
    let result = storage
        .run(move |connection| {
            execute(connection, &path, 2, LIMITS, apply, target, |stage| {
                if stage == Point::BeforePublish {
                    fs::remove_file(path.join(INDEX)).unwrap();
                    fs::create_dir(path.join(INDEX)).unwrap();
                }
                Ok(())
            })
        })
        .await;
    assert_eq!(result.unwrap_err(), StorageError::OutcomeUnknown);
    assert!(old.path.exists());
    fs::remove_dir(folder.join(INDEX)).unwrap();
    fs::write(folder.join(INDEX), old_index).unwrap();
    let result = storage
        .recover_migration_backup(&folder, LIMITS, target)
        .await
        .unwrap()
        .unwrap();
    assert!(!old.path.exists());
    verify(&result.path, LIMITS).unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn missing_corrupt_committed_candidate_and_forged_index_never_use_fallback() {
    for failure in 0..3 {
        let (_root, storage, folder) = fixture().await;
        let old = initial(&storage, &folder).await;
        let before = files(&old.path);
        let path = folder.clone();
        assert!(
            storage
                .run(move |c| execute(c, &path, 2, LIMITS, apply, target, |p| {
                    if p == Point::AfterCommit {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                }))
                .await
                .is_err()
        );
        let path = folder.clone();
        let current = storage
            .run(move |c| Ok(path.join(committed(c)?.unwrap().current.name)))
            .await
            .unwrap();
        match failure {
            0 => fs::remove_dir_all(&current).unwrap(),
            1 => fs::write(current.join("database.sqlite"), b"bad").unwrap(),
            _ => {
                fs::write(
                    folder.join(INDEX),
                    br#"{"name":"../escape","manifest":{"bytes":0,"sha256":[]}}"#,
                )
                .unwrap();
            }
        }
        assert!(
            storage
                .recover_migration_backup(&folder, LIMITS, target)
                .await
                .is_err()
        );
        assert_eq!(before, files(&old.path));
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn abandoned_candidates_are_not_promoted_and_only_explicit_orphans_can_be_deleted() {
    let (_root, storage, folder) = fixture().await;
    let candidate = storage
        .prepare_migration_backup(&folder, 1, LIMITS)
        .await
        .unwrap();
    assert!(
        storage
            .recover_migration_backup(&folder, LIMITS, validate_database)
            .await
            .unwrap()
            .is_none()
    );
    assert!(candidate.path().exists());
    let name = candidate
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    storage
        .discard_migration_candidate(&folder, name)
        .await
        .unwrap();
    assert!(!candidate.path().exists());
    let accepted = initial(&storage, &folder).await;
    for name in [
        accepted
            .path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned(),
        "../escape".into(),
        "migration.lock".into(),
    ] {
        assert!(
            storage
                .discard_migration_candidate(&folder, name)
                .await
                .is_err()
        );
    }
    assert!(accepted.path.exists());
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn writer_exclusion_continues_from_capture_through_commit_and_folder_lock_excludes_publishers()
 {
    let (root, storage, folder) = fixture().await;
    let _old = initial(&storage, &folder).await;
    let source = root.path().join("device.sqlite");
    let other_folder = folder.clone();
    let path = folder.clone();
    storage
        .run(move |c| {
            execute(c, &path, 2, LIMITS, apply, target, |point| {
                if matches!(point, Point::AfterCapture | Point::BeforeCommit) {
                    let other = Connection::open(&source).unwrap();
                    other.busy_timeout(std::time::Duration::ZERO).unwrap();
                    assert!(other.execute_batch("BEGIN IMMEDIATE").is_err());
                }
                assert!(lock(&other_folder).is_err());
                Ok(())
            })
        })
        .await
        .unwrap();
    assert!(lock(&folder).is_ok(), "RAII must release folder lock");
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn dropped_response_keeps_committed_witness_for_reconciliation() {
    let (_root, storage, folder) = fixture().await;
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, blocked) = mpsc::channel();
    let writer = storage.clone();
    let path = folder.clone();
    let task = tokio::spawn(async move {
        writer
            .with_migration_backup(
                path,
                1,
                LIMITS,
                move |tx| {
                    entered.send(()).unwrap();
                    blocked.recv().unwrap();
                    apply(tx)
                },
                target,
            )
            .await
    });
    // A current-thread runtime must yield until admission, not block its sender.
    waiting.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    let recovered = storage
        .recover_migration_backup(&folder, LIMITS, target)
        .await
        .unwrap()
        .unwrap();
    verify(&recovered.path, LIMITS).unwrap();
    storage.run(|c| target(c)).await.unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn crash_child() {
    let Ok(root) = std::env::var("ZEDONE_BACKUP_CRASH_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let point = std::env::var("ZEDONE_BACKUP_CRASH_POINT").unwrap();
    let storage = open(root.join("device.sqlite")).await.unwrap();
    storage
        .run(move |c| {
            execute(
                c,
                &root.join("backups"),
                2,
                LIMITS,
                apply,
                target,
                |stage| {
                    if format!("{stage:?}") == point {
                        std::process::exit(73);
                    }
                    Ok(())
                },
            )
        })
        .await
        .unwrap();
    panic!("crash point not reached");
}

#[tokio::test]
async fn two_workers_cannot_publish_over_each_other_or_repeat_the_pending_operation() {
    let (root, storage, folder) = fixture().await;
    let other = open(root.path().join("device.sqlite")).await.unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, blocked) = mpsc::channel();
    let writer = storage.clone();
    let path = folder.clone();
    let task = tokio::spawn(async move {
        writer
            .with_migration_backup(
                path,
                1,
                LIMITS,
                move |tx| {
                    entered.send(()).unwrap();
                    blocked.recv().unwrap();
                    apply(tx)
                },
                target,
            )
            .await
    });
    waiting.await.unwrap();
    let result = other
        .with_migration_backup(
            &folder,
            2,
            LIMITS,
            |_| panic!("must not enter second operation"),
            validate_database,
        )
        .await;
    release.send(()).unwrap();
    assert_eq!(result.unwrap_err(), StorageError::Unavailable);
    let published = task.await.unwrap().unwrap();
    let recovered = other
        .recover_migration_backup(&folder, LIMITS, target)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(published.path, recovered.path);
    storage.shutdown().await.unwrap();
    other.shutdown().await.unwrap();
}

#[tokio::test]
async fn corrupt_candidate_before_commit_rolls_back_mutation_and_keeps_old_index() {
    let (_root, storage, folder) = fixture().await;
    let old = initial(&storage, &folder).await;
    let before = files(&old.path);
    let old_path = old.path.clone();
    let path = folder.clone();
    let result = storage
        .run(move |c| {
            execute(c, &path, 2, LIMITS, apply, target, |point| {
                if point == Point::AfterCapture {
                    for entry in fs::read_dir(&path).unwrap() {
                        let candidate = entry.unwrap().path();
                        if candidate.is_dir() && candidate != old_path {
                            fs::write(candidate.join("space-0.bin"), b"corrupt").unwrap();
                        }
                    }
                }
                Ok(())
            })
        })
        .await;
    assert!(result.is_err());
    storage
        .run(|c| {
            assert!(!mutated(c));
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(before, files(&old.path));
    assert_eq!(
        index(&folder).unwrap().unwrap().name,
        old.path.file_name().unwrap().to_str().unwrap()
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_missing_index_is_rebuilt_from_commit_but_an_uncommitted_index_is_rejected() {
    let (_root, storage, folder) = fixture().await;
    let orphan = storage
        .prepare_migration_backup(&folder, 1, LIMITS)
        .await
        .unwrap();
    let reference = BackupRef {
        name: orphan
            .path()
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned(),
        manifest: digest(&orphan.path().join("manifest.json"), manifest_limit(LIMITS)).unwrap(),
    };
    publish(&folder, &reference).unwrap();
    assert!(
        storage
            .recover_migration_backup(&folder, LIMITS, validate_database)
            .await
            .is_err()
    );
    fs::remove_file(folder.join(INDEX)).unwrap();
    let old = initial(&storage, &folder).await;
    let published = storage
        .with_migration_backup(&folder, 2, LIMITS, apply, target)
        .await
        .unwrap();
    assert!(!old.path.exists());
    fs::remove_file(folder.join(INDEX)).unwrap();
    let recovered = storage
        .recover_migration_backup(&folder, LIMITS, target)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(published.path, recovered.path);
    assert_eq!(
        index(&folder).unwrap().unwrap().name,
        published.path.file_name().unwrap().to_str().unwrap()
    );
    assert!(orphan.path().exists());
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn process_termination_restart_never_promotes_an_uncommitted_candidate() {
    for point in [
        Point::CopyWritten,
        Point::AfterCapture,
        Point::BeforeCommit,
        Point::AfterCommit,
        Point::BeforePublish,
        Point::AfterPublish,
        Point::BeforeCleanup,
    ] {
        let (root, storage, folder) = fixture().await;
        let old = initial(&storage, &folder).await;
        let before = files(&old.path);
        storage.shutdown().await.unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "migration_backup::publication::tests::crash_child",
                "--nocapture",
            ])
            .env("ZEDONE_BACKUP_CRASH_ROOT", root.path())
            .env("ZEDONE_BACKUP_CRASH_POINT", format!("{point:?}"))
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73), "{point:?}");
        assert_eq!(before, files(&old.path), "old files survive {point:?}");
        let reopened = open(root.path().join("device.sqlite")).await.unwrap();
        let after_commit = matches!(
            point,
            Point::AfterCommit | Point::BeforePublish | Point::AfterPublish | Point::BeforeCleanup
        );
        let result = reopened
            .recover_migration_backup(&folder, LIMITS, move |c| {
                assert_eq!(mutated(c), after_commit);
                if after_commit {
                    target(c)
                } else {
                    validate_database(c)
                }
            })
            .await
            .unwrap()
            .unwrap();
        assert_eq!(result.path == old.path, !after_commit);
        assert_eq!(old.path.exists(), !after_commit);
        verify(&result.path, LIMITS).unwrap();
        let again = reopened
            .recover_migration_backup(&folder, LIMITS, validate_database)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(again.path, result.path);
        reopened.shutdown().await.unwrap();
    }
}
