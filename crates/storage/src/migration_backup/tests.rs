use std::{fs, sync::mpsc};

use automerge::{Automerge, ROOT, ReadDoc, sync::State, transaction::Transactable};
use rusqlite::types::Value;

use super::*;

const LIMITS: BackupLimits = BackupLimits {
    max_database_bytes: 2 * 1024 * 1024,
    max_spaces: 10,
    max_total_snapshot_bytes: 1024 * 1024,
    snapshot: SnapshotLimits {
        max_bytes: 128 * 1024,
        max_changes: 100,
    },
};
const SPACE: &str = "../пространство/ё🙂";

async fn fixture() -> (TempDir, Storage, PathBuf) {
    let root = TempDir::new().unwrap();
    let folder = root.path().join("backups");
    fs::create_dir(&folder).unwrap();
    let storage = Storage::open(root.path().join("device.sqlite"))
        .await
        .unwrap();
    let mut graph = Automerge::new();
    let mut tx = graph.transaction();
    tx.put(ROOT, "history", "missing").unwrap();
    tx.commit();
    let mut tx = graph.transaction();
    tx.put(ROOT, "history", "orphan").unwrap();
    let (hash, _) = tx.commit();
    let orphan = graph.get_change_by_hash(&hash.unwrap()).unwrap();
    storage
        .write(b"seed".to_vec(), b"private-request".to_vec(), move |ctx| {
            ctx.create_document(SPACE)?;
            ctx.edit(SPACE, |tx| {
                tx.put(ROOT, "title", "before ё🙂")?;
                Ok(())
            })?;
            ctx.apply_changes(SPACE, vec![orphan])?;
            ctx.set_peer_state(SPACE, b"private-peer", &State::new())?;
            ctx.create_document("empty")?;
            ctx.create_document("other")?;
            Ok("private-result")
        })
        .await
        .unwrap();
    (root, storage, folder)
}

fn rows(connection: &Connection, table: &str) -> Vec<Vec<Value>> {
    let mut statement = connection
        .prepare(&format!("SELECT * FROM {table} ORDER BY 1, 2"))
        .unwrap();
    let columns = statement.column_count();
    statement
        .query_map([], |r| (0..columns).map(|i| r.get(i)).collect())
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}

fn files(folder: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut result = Vec::new();
    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            result.extend(files(&path));
        } else {
            result.push((path.clone(), fs::read(path).unwrap()));
        }
    }
    result.sort();
    result
}

fn source_bytes(root: &Path) -> (Vec<u8>, Vec<u8>) {
    (
        fs::read(root.join("device.sqlite")).unwrap(),
        fs::read(root.join("device.sqlite-wal")).unwrap(),
    )
}

#[tokio::test]
async fn full_wal_database_and_all_archives_are_one_state_without_touching_source() {
    let (root, storage, folder) = fixture().await;
    let before = source_bytes(root.path());
    let candidate = storage
        .prepare_migration_backup(&folder, 42, LIMITS)
        .await
        .unwrap();
    assert_eq!(before, source_bytes(root.path()));
    candidate.verify(LIMITS).unwrap();
    let copied = readonly(&candidate.path().join("database.sqlite")).unwrap();
    let source = readonly(&root.path().join("device.sqlite")).unwrap();
    for table in [
        "sqlite_schema",
        "documents",
        "changes",
        "peers",
        "command_results",
    ] {
        assert_eq!(rows(&source, table), rows(&copied, table), "{table}");
    }
    assert!(!candidate.path().join("database.sqlite-wal").exists());
    let names = spaces(&copied, LIMITS).unwrap();
    assert_eq!(names.len(), 3);
    for (i, name) in names.into_iter().enumerate() {
        let bytes = fs::read(candidate.path().join(format!("space-{i}.bin"))).unwrap();
        let snapshot = SpaceSnapshot::decode(&bytes, LIMITS.snapshot).unwrap();
        assert_eq!(snapshot.metadata().space, name);
        if name == SPACE {
            assert_eq!(snapshot.changes().len(), 2);
        }
        for secret in [
            b"private-peer".as_slice(),
            b"private-request",
            b"private-result",
        ] {
            assert!(!bytes.windows(secret.len()).any(|w| w == secret));
        }
    }
    drop(copied);
    drop(source);
    candidate.discard().unwrap();
    assert_eq!(fs::read_dir(folder).unwrap().count(), 0);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn reverse_restart_and_later_candidate_preserve_previous_files() {
    let (root, storage, folder) = fixture().await;
    let first = storage
        .prepare_migration_backup(&folder, 42, LIMITS)
        .await
        .unwrap();
    let old = files(first.path());
    for (key, value) in [(b"forward".to_vec(), true), (b"reverse".to_vec(), false)] {
        storage
            .write(key, vec![], move |ctx| {
                ctx.edit(SPACE, |tx| {
                    tx.put(ROOT, "done", value)?;
                    Ok(())
                })
            })
            .await
            .unwrap();
    }
    storage.shutdown().await.unwrap();
    let storage = Storage::open(root.path().join("device.sqlite"))
        .await
        .unwrap();
    let second = storage
        .prepare_migration_backup(&folder, 43, LIMITS)
        .await
        .unwrap();
    assert_ne!(first.path(), second.path());
    assert_eq!(old, files(first.path()));
    first.verify(LIMITS).unwrap();
    second.verify(LIMITS).unwrap();
    let path = second.path().to_owned();
    drop(second);
    assert!(
        path.exists(),
        "dropping a completed handle must retain the candidate"
    );
    first.discard().unwrap();
    assert!(path.exists());
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn all_admission_limits_fail_without_changing_source_or_previous_copies() {
    let (root, storage, folder) = fixture().await;
    let previous = storage
        .prepare_migration_backup(&folder, 1, LIMITS)
        .await
        .unwrap();
    let old = files(&folder);
    let before = source_bytes(root.path());
    for limits in [
        BackupLimits {
            max_database_bytes: 1,
            ..LIMITS
        },
        BackupLimits {
            max_spaces: 2,
            ..LIMITS
        },
        BackupLimits {
            max_total_snapshot_bytes: 1,
            ..LIMITS
        },
        BackupLimits {
            snapshot: SnapshotLimits {
                max_bytes: 1,
                ..LIMITS.snapshot
            },
            ..LIMITS
        },
        BackupLimits {
            snapshot: SnapshotLimits {
                max_changes: 0,
                ..LIMITS.snapshot
            },
            ..LIMITS
        },
    ] {
        assert!(
            storage
                .prepare_migration_backup(&folder, 2, limits)
                .await
                .is_err()
        );
        assert_eq!(files(&folder), old);
        assert_eq!(source_bytes(root.path()), before);
        assert!(previous.verify(limits).is_err());
    }
    previous.discard().unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn missing_or_non_directory_destination_never_falls_back() {
    let (root, storage, folder) = fixture().await;
    let old = files(&folder);
    let missing = root.path().join("missing");
    let file = root.path().join("file");
    fs::write(&file, b"keep").unwrap();
    for path in [&missing, &file] {
        assert!(
            storage
                .prepare_migration_backup(path, 1, LIMITS)
                .await
                .is_err()
        );
    }
    assert!(!missing.exists());
    assert_eq!(fs::read(file).unwrap(), b"keep");
    assert_eq!(files(&folder), old);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn faults_at_every_creation_boundary_keep_source_and_old_files() {
    let (root, storage, folder) = fixture().await;
    let previous = storage
        .prepare_migration_backup(&folder, 1, LIMITS)
        .await
        .unwrap();
    let old = files(&folder);
    let before = source_bytes(root.path());
    for fail in [
        Checkpoint::BeforeCopy,
        Checkpoint::AfterCopy,
        Checkpoint::AfterSpace,
        Checkpoint::BeforeVerify,
        Checkpoint::AfterVerify,
    ] {
        let target = folder.clone();
        assert!(
            storage
                .run(
                    move |connection| prepare(connection, &target, 2, LIMITS, |stage| {
                        if stage == fail {
                            Err(StorageError::Unavailable)
                        } else {
                            Ok(())
                        }
                    })
                )
                .await
                .is_err()
        );
        assert_eq!(files(&folder), old);
        assert_eq!(source_bytes(root.path()), before);
    }
    previous.discard().unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn verification_rejects_missing_corrupt_and_incoherent_files() {
    let (_root, storage, folder) = fixture().await;
    let candidate = storage
        .prepare_migration_backup(&folder, 1, LIMITS)
        .await
        .unwrap();
    let pristine = files(candidate.path());
    for (path, original) in &pristine {
        fs::remove_file(path).unwrap();
        assert!(candidate.verify(LIMITS).is_err());
        let mut damaged = original.clone();
        damaged[0] ^= 1;
        fs::write(path, damaged).unwrap();
        assert!(candidate.verify(LIMITS).is_err());
        fs::write(path, original).unwrap();
    }
    let manifest_path = candidate.path().join("manifest.json");
    let original = fs::read(&manifest_path).unwrap();
    let mut manifest: Manifest = serde_json::from_slice(&original).unwrap();
    manifest.created_at_unix_ms += 1;
    fs::write(&manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
    assert!(candidate.verify(LIMITS).is_err());
    fs::write(&manifest_path, original).unwrap();
    candidate.verify(LIMITS).unwrap();
    for sidecar in [
        "database.sqlite-wal",
        "database.sqlite-shm",
        "database.sqlite-journal",
    ] {
        let path = candidate.path().join(sidecar);
        fs::write(&path, b"unhashed state").unwrap();
        assert!(candidate.verify(LIMITS).is_err());
        fs::remove_file(path).unwrap();
    }
    let extra = candidate.path().join("unexpected-directory");
    fs::create_dir(&extra).unwrap();
    assert!(candidate.verify(LIMITS).is_err());
    fs::remove_dir(extra).unwrap();
    candidate.verify(LIMITS).unwrap();
    candidate.discard().unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn concurrent_writer_cannot_commit_inside_capture() {
    let (root, storage, folder) = fixture().await;
    let path = root.path().join("device.sqlite");
    let (sent, received) = mpsc::channel();
    let (ready, wait) = mpsc::channel();
    let (finished, observed) = mpsc::channel();
    let writer = std::thread::spawn(move || {
        let c = Connection::open(path).unwrap();
        c.busy_timeout(std::time::Duration::ZERO).unwrap();
        ready.send(()).unwrap();
        received.recv().unwrap();
        let error = c
            .execute(
                "UPDATE documents SET actor = x'99' WHERE space = 'other'",
                [],
            )
            .unwrap_err();
        assert_eq!(
            error.sqlite_error_code(),
            Some(rusqlite::ErrorCode::DatabaseBusy)
        );
        finished.send(()).unwrap();
    });
    wait.recv().unwrap();
    let candidate = storage
        .run(move |connection| {
            prepare(connection, &folder, 1, LIMITS, |stage| {
                if stage == Checkpoint::AfterCopy {
                    sent.send(()).unwrap();
                    observed.recv().unwrap();
                }
                Ok(())
            })
        })
        .await
        .unwrap();
    writer.join().unwrap();
    candidate.verify(LIMITS).unwrap();
    candidate.discard().unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_schema_or_change_log_is_never_a_verified_candidate() {
    for sql in [
        "PRAGMA user_version = 999",
        "DROP TABLE peers",
        "UPDATE changes SET bytes = x'01'",
        "UPDATE changes SET space = 'missing'",
    ] {
        let (root, storage, folder) = fixture().await;
        let c = Connection::open(root.path().join("device.sqlite")).unwrap();
        c.pragma_update(None, "foreign_keys", false).unwrap();
        c.execute_batch(sql).unwrap();
        let before = source_bytes(root.path());
        assert!(
            storage
                .prepare_migration_backup(&folder, 1, LIMITS)
                .await
                .is_err(),
            "{sql}"
        );
        assert!(files(&folder).is_empty());
        assert_eq!(source_bytes(root.path()), before);
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn rehashed_database_still_requires_layout_graph_and_archive_agreement() {
    let (_root, storage, folder) = fixture().await;
    for sql in [
        "PRAGMA user_version = 999",
        "DROP TABLE peers",
        "UPDATE changes SET bytes = x'01'",
        "UPDATE changes SET space = 'missing'",
        "DELETE FROM documents WHERE space = 'empty'",
    ] {
        let candidate = storage
            .prepare_migration_backup(&folder, 1, LIMITS)
            .await
            .unwrap();
        let path = candidate.path().join("database.sqlite");
        let c = Connection::open(&path).unwrap();
        c.pragma_update(None, "foreign_keys", false).unwrap();
        c.execute_batch(sql).unwrap();
        c.close().unwrap();
        let manifest_path = candidate.path().join("manifest.json");
        let mut manifest: Manifest =
            serde_json::from_slice(&fs::read(&manifest_path).unwrap()).unwrap();
        manifest.database = digest(&path, LIMITS.max_database_bytes).unwrap();
        fs::write(manifest_path, serde_json::to_vec(&manifest).unwrap()).unwrap();
        assert!(candidate.verify(LIMITS).is_err(), "{sql}");
        candidate.discard().unwrap();
    }
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn empty_database_candidate_and_incomplete_crash_remnants_are_independent() {
    let root = TempDir::new().unwrap();
    let storage = Storage::open(root.path().join("device.sqlite"))
        .await
        .unwrap();
    let remnant = root.path().join("zedone-migration-candidate-interrupted");
    fs::create_dir(&remnant).unwrap();
    fs::write(remnant.join("database.sqlite"), b"partial copy").unwrap();
    let candidate = storage
        .prepare_migration_backup(
            root.path(),
            0,
            BackupLimits {
                max_spaces: 0,
                ..LIMITS
            },
        )
        .await
        .unwrap();
    candidate
        .verify(BackupLimits {
            max_spaces: 0,
            ..LIMITS
        })
        .unwrap();
    assert_eq!(fs::read_dir(candidate.path()).unwrap().count(), 2);
    assert!(verify(&remnant, LIMITS).is_err());
    candidate.discard().unwrap();
    assert_eq!(
        fs::read(remnant.join("database.sqlite")).unwrap(),
        b"partial copy"
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn lost_response_retains_completed_candidate_without_unlocking_early() {
    let (_root, storage, folder) = fixture().await;
    let worker = storage.clone();
    let target = folder.clone();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, resume) = mpsc::channel();
    let task = tokio::spawn(async move {
        worker
            .run(move |connection| {
                entered.send(()).unwrap();
                resume.recv().unwrap();
                prepare(connection, &target, 1, LIMITS, |_| Ok(()))
            })
            .await
    });
    waiting.await.unwrap();
    task.abort();
    assert!(task.await.unwrap_err().is_cancelled());
    release.send(()).unwrap();
    // An ordered worker barrier waits until preparation and response disposal.
    storage.read(|_| Ok(())).await.unwrap();
    let candidates: Vec<_> = fs::read_dir(&folder)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    assert_eq!(candidates.len(), 1);
    verify(&candidates[0], LIMITS).unwrap();
    storage.shutdown().await.unwrap();
}
