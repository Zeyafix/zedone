use super::*;
use crate::access::PendingAccess;
use std::sync::Arc;
use zedone_core::spaces::SpaceStore;
const SPACE: &str = "00000000000000000000000000000001";
async fn setup() -> (tempfile::TempDir, Storage, Arc<PendingAccess>) {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("source.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    db.transact(move |ctx| {
        ctx.bootstrap_access(&a)?;
        ctx.create_space(SPACE, "Архив ё🙂", a.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    db.daily_tick(true).await.unwrap();
    (dir, db, admin)
}
async fn grant(db: &Storage, a: &PendingAccess, path: Option<String>, write: bool) -> BackupFolder {
    db.select_backup_folder(a.credential().0.to_vec(), path, write)
        .await
        .unwrap()
}
#[tokio::test]
async fn catalog_without_old_database_reads_files_and_never_restores_or_changes_location() {
    let (dir, db, _a) = setup().await;
    let root = dir.path().join("source.db.backups");
    let clean = Storage::open(dir.path().join("clean.db")).await.unwrap();
    let b = Arc::new(PendingAccess::generate().unwrap());
    let boot = b.clone();
    clean
        .transact(move |ctx| {
            ctx.bootstrap_access(&boot)?;
            Ok(())
        })
        .await
        .unwrap();
    let cap = grant(&clean, &b, Some(root.to_str().unwrap().into()), false).await;
    let subject = b.subject().to_owned();
    clean
        .transact(move |ctx| {
            let before = location(ctx.connection())?;
            let c = ctx.backup_catalog(
                &subject,
                &CatalogFolder {
                    folder: cap.id.clone(),
                },
            )?;
            assert_eq!(c.copies.len(), 1);
            assert_eq!(c.copies[0].names, ["Архив ё🙂"]);
            assert_eq!(c.copies[0].space, SPACE);
            let archive = ctx.backup_read(
                &subject,
                &CatalogRead {
                    folder: cap.id.clone(),
                    copy: c.copies[0].id.clone(),
                    expected: c.copies[0].digest,
                },
            )?;
            assert!(archive.archive.starts_with("5a45445350414345"));
            assert_eq!(version(&before)?, version(&location(ctx.connection())?)?);
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    assert_eq!(
        clean
            .read(|ctx| Ok(ctx.connection().query_row(
                "SELECT count(*) FROM documents",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
    db.shutdown().await.unwrap();
    clean.shutdown().await.unwrap();
}
#[tokio::test]
async fn copy_verify_switch_cleanup_restart_and_return_preserve_contents_and_policy() {
    let (dir, db, a) = setup().await;
    let target = dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::write(target.join("foreign.txt"), b"keep").unwrap();
    let cap = grant(&db, &a, Some(target.to_str().unwrap().into()), true).await;
    let subject = a.subject().to_owned();
    db.transact(move |ctx| {
        let loc = location_view(ctx.connection(), &subject)?;
        let result = ctx.backup_change(
            &subject,
            &ChangeBackupLocation {
                folder: cap.id,
                expected: loc.version,
            },
        )?;
        assert!(result.cleanup_pending);
        Ok(())
    })
    .await
    .unwrap();
    let source = dir.path().join("source.db.backups");
    assert_eq!(std::fs::read_dir(&source).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 2);
    db.shutdown().await.unwrap();
    let reopened = Storage::open(dir.path().join("source.db")).await.unwrap();
    reopened.cleanup_backup_location().await.unwrap();
    assert_eq!(std::fs::read_dir(&source).unwrap().count(), 0);
    assert_eq!(std::fs::read(target.join("foreign.txt")).unwrap(), b"keep");
    reopened.daily_tick(true).await.unwrap();
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 2);
    let cap = grant(&reopened, &a, None, true).await;
    let subject = a.subject().to_owned();
    reopened
        .transact(move |ctx| {
            let loc = location_view(ctx.connection(), &subject)?;
            ctx.backup_change(
                &subject,
                &ChangeBackupLocation {
                    folder: cap.id,
                    expected: loc.version,
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    reopened.cleanup_backup_location().await.unwrap();
    assert_eq!(std::fs::read_dir(&source).unwrap().count(), 1);
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
    reopened.shutdown().await.unwrap();
}
#[tokio::test]
async fn corrupt_destination_refuses_switch_and_original_survives() {
    let (dir, db, a) = setup().await;
    let root = dir.path().join("source.db.backups");
    let name = std::fs::read_dir(&root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name();
    let target = dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    std::fs::create_dir(target.join(&name)).unwrap();
    std::fs::write(target.join(&name).join("space.bin"), b"broken").unwrap();
    let cap = grant(&db, &a, Some(target.to_str().unwrap().into()), true).await;
    let subject = a.subject().to_owned();
    assert!(
        db.transact(move |ctx| {
            let before = location_view(ctx.connection(), &subject)?;
            ctx.backup_change(
                &subject,
                &ChangeBackupLocation {
                    folder: cap.id,
                    expected: before.version,
                },
            )?;
            Ok(())
        })
        .await
        .is_err()
    );
    assert!(root.join(name).join("space.bin").is_file());
    db.read(|ctx| {
        assert!(location(ctx.connection())?.current.is_none());
        Ok(())
    })
    .await
    .unwrap();
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn explicit_delete_keeps_working_space_and_last_success_quota() {
    let (dir, db, a) = setup().await;
    let cap = grant(&db, &a, None, true).await;
    let subject = a.subject().to_owned();
    db.transact(move |ctx| {
        let copies = ctx.backup_catalog(
            &subject,
            &CatalogFolder {
                folder: cap.id.clone(),
            },
        )?;
        let c = &copies.copies[0];
        ctx.backup_delete(
            &subject,
            &CatalogDelete {
                folder: cap.id,
                copy: c.id.clone(),
                expected: c.digest,
            },
        )?;
        Ok(())
    })
    .await
    .unwrap();
    db.cleanup_backup_location().await.unwrap();
    db.daily_tick(true).await.unwrap();
    assert_eq!(
        std::fs::read_dir(dir.path().join("source.db.backups"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(
        db.read(|ctx| Ok(ctx.connection().query_row(
            "SELECT count(*) FROM documents",
            [],
            |r| r.get::<_, i64>(0)
        )?))
        .await
        .unwrap(),
        1
    );
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn restart_at_both_commit_boundaries_keeps_a_verified_copy() {
    for committed in [false, true] {
        let (dir, db, a) = setup().await;
        let target = dir.path().join("target");
        std::fs::create_dir(&target).unwrap();
        let cap = grant(&db, &a, Some(target.to_str().unwrap().into()), true).await;
        let config = serde_json::json!({"subject":a.subject(),"folder":cap.id});
        std::fs::write(
            dir.path().join("crash.json"),
            serde_json::to_vec(&config).unwrap(),
        )
        .unwrap();
        db.shutdown().await.unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "backup_catalog::tests::transfer_crash_child",
                "--ignored",
                "--nocapture",
            ])
            .env("ZEDONE_CATALOG_CRASH", dir.path())
            .env(
                "ZEDONE_CATALOG_COMMITTED",
                if committed { "1" } else { "0" },
            )
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(77),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let reopened = Storage::open(dir.path().join("source.db")).await.unwrap();
        let old = dir.path().join("source.db.backups");
        assert_eq!(std::fs::read_dir(&old).unwrap().count(), 1);
        assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
        let t = std::fs::canonicalize(&target).unwrap();
        reopened
            .read(move |ctx| {
                assert_eq!(folder(ctx.connection())? == t, committed);
                Ok(())
            })
            .await
            .unwrap();
        if committed {
            reopened.cleanup_backup_location().await.unwrap();
            assert_eq!(std::fs::read_dir(&old).unwrap().count(), 0);
        } else {
            let subject = a.subject().to_owned();
            let folder_id = config["folder"].as_str().unwrap().to_owned();
            reopened
                .transact(move |ctx| {
                    let loc = location_view(ctx.connection(), &subject)?;
                    ctx.backup_change(
                        &subject,
                        &ChangeBackupLocation {
                            folder: folder_id,
                            expected: loc.version,
                        },
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
            reopened.cleanup_backup_location().await.unwrap();
            assert_eq!(std::fs::read_dir(&old).unwrap().count(), 0);
        }
        reopened.shutdown().await.unwrap();
    }
}

#[tokio::test]
#[ignore]
async fn transfer_crash_child() {
    let dir =
        PathBuf::from(std::env::var_os("ZEDONE_CATALOG_CRASH").expect("only from crash driver"));
    let config: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("crash.json")).unwrap()).unwrap();
    let db = Storage::open(dir.join("source.db")).await.unwrap();
    let committed = std::env::var("ZEDONE_CATALOG_COMMITTED").unwrap() == "1";
    db.transact_checked(
        move |ctx| {
            let subject = config["subject"].as_str().unwrap();
            let loc = location_view(ctx.connection(), subject)?;
            ctx.backup_change(
                subject,
                &ChangeBackupLocation {
                    folder: config["folder"].as_str().unwrap().into(),
                    expected: loc.version,
                },
            )?;
            Ok(())
        },
        move |stage| {
            if matches!(
                (stage, committed),
                (crate::Stage::BeforeCommit, false) | (crate::Stage::AfterCommit, true)
            ) {
                std::process::exit(77);
            }
            Ok(())
        },
    )
    .await
    .unwrap();
    panic!("crash checkpoint not reached");
}

#[tokio::test]
async fn migration_package_and_safety_snapshot_move_without_rewriting_their_bytes() {
    use crate::migration_backup::BackupLimits;
    use zedone_core::recovery::{RecoveryStore, RestoreSnapshot};
    let (dir, db, a) = setup().await;
    let root = dir.path().join("source.db.backups");
    db.transact(|ctx| {
        let snapshot =
            crate::snapshot::export::capture_export(ctx.connection(), SPACE.into(), 100, LIMITS)?;
        let bytes = snapshot.encode(LIMITS).unwrap();
        let state = ctx.recovery_state(SPACE)?;
        ctx.restore_snapshot(
            &RestoreSnapshot {
                space: SPACE.into(),
                archive: bytes.iter().map(|b| format!("{b:02x}")).collect(),
                expected: state.version,
            },
            101,
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let limits = BackupLimits {
        max_database_bytes: 16 * 1024 * 1024,
        max_spaces: 10,
        max_total_snapshot_bytes: 10 * 1024 * 1024,
        snapshot: LIMITS,
    };
    db.with_migration_backup(root.clone(), 102, limits, |_| Ok(()), |_| Ok(()))
        .await
        .unwrap();
    let target = dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    let cap = grant(&db, &a, Some(target.to_str().unwrap().into()), true).await;
    let subject = a.subject().to_owned();
    let id = cap.id.clone();
    let index_path = root.join(migration::INDEX);
    let index = std::fs::read(&index_path).unwrap();
    std::fs::write(&index_path, b"foreign file").unwrap();
    let retry_subject = subject.clone();
    let retry_id = id.clone();
    assert!(
        db.transact(move |ctx| {
            let loc = location_view(ctx.connection(), &retry_subject)?;
            ctx.backup_change(
                &retry_subject,
                &ChangeBackupLocation {
                    folder: retry_id,
                    expected: loc.version,
                },
            )?;
            Ok(())
        })
        .await
        .is_err()
    );
    assert_eq!(std::fs::read(&index_path).unwrap(), b"foreign file");
    db.read(|ctx| {
        assert!(location(ctx.connection())?.current.is_none());
        Ok(())
    })
    .await
    .unwrap();
    std::fs::write(index_path, index).unwrap();
    db.transact(move |ctx| {
        let loc = location_view(ctx.connection(), &subject)?;
        ctx.backup_change(
            &subject,
            &ChangeBackupLocation {
                folder: cap.id,
                expected: loc.version,
            },
        )?;
        Ok(())
    })
    .await
    .unwrap();
    db.cleanup_backup_location().await.unwrap();
    let subject = a.subject().to_owned();
    db.transact(move |ctx| {
        let catalog = ctx.backup_catalog(&subject, &CatalogFolder { folder: id.clone() })?;
        assert!(catalog.copies.iter().any(|c| c.kind == "daily"));
        assert!(catalog.copies.iter().any(|c| c.kind == "recovery"));
        let migration = catalog
            .copies
            .iter()
            .find(|c| c.kind == "migration")
            .unwrap();
        let read = ctx.backup_read(
            &subject,
            &CatalogRead {
                folder: id,
                copy: migration.id.clone(),
                expected: migration.digest,
            },
        )?;
        assert!(read.archive.starts_with("5a45445350414345"));
        let state = ctx.recovery_state(SPACE)?;
        assert!(state.backup.is_some());
        Ok(())
    })
    .await
    .unwrap();
    assert!(target.join("migration-current.json").is_file());
    assert!(root.join("migration.lock").is_file());
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn cleanup_failure_keeps_new_location_verified_files_and_retries_without_switching_back() {
    let (dir, db, a) = setup().await;
    let root = dir.path().join("source.db.backups");
    let name = std::fs::read_dir(&root)
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .file_name();
    let target = dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    let cap = grant(&db, &a, Some(target.to_str().unwrap().into()), true).await;
    let subject = a.subject().to_owned();
    db.transact(move |ctx| {
        let loc = location_view(ctx.connection(), &subject)?;
        ctx.backup_change(
            &subject,
            &ChangeBackupLocation {
                folder: cap.id,
                expected: loc.version,
            },
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let foreign = root.join(&name).join("foreign.txt");
    std::fs::write(&foreign, b"do not delete").unwrap();
    assert!(db.cleanup_backup_location().await.is_err());
    let t = std::fs::canonicalize(&target).unwrap();
    db.read(move |ctx| {
        assert_eq!(folder(ctx.connection())?, t);
        assert!(location(ctx.connection())?.pending.is_some());
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(std::fs::read(&foreign).unwrap(), b"do not delete");
    assert_eq!(
        std::fs::read(root.join(&name).join("space.bin")).unwrap(),
        std::fs::read(target.join(&name).join("space.bin")).unwrap()
    );
    std::fs::remove_file(foreign).unwrap();
    db.cleanup_backup_location().await.unwrap();
    assert!(!root.join(&name).exists());
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn interrupted_file_copy_resumes_only_verified_prefixes_and_preserves_foreign_collisions() {
    let (dir, db, _a) = setup().await;
    let root = dir.path().join("source.db.backups");
    let entry = std::fs::read_dir(&root).unwrap().next().unwrap().unwrap();
    let id = entry.file_name().to_str().unwrap().to_owned();
    let (_, record) = inspect_copy(&root, &id).unwrap();
    let target = dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    let destination = target.join(&id);
    std::fs::create_dir(&destination).unwrap();
    std::fs::write(
        destination.join("transfer.json"),
        serde_json::to_vec(&record).unwrap(),
    )
    .unwrap();
    let original = std::fs::read(entry.path().join("space.bin")).unwrap();
    std::fs::write(destination.join("space.bin"), &original[..23]).unwrap();
    migration::transfer_record(&root, &target, &record).unwrap();
    assert_eq!(
        std::fs::read(destination.join("space.bin")).unwrap(),
        original
    );
    assert!(!destination.join("transfer.json").exists());
    let other = dir.path().join("collision");
    std::fs::create_dir(&other).unwrap();
    std::fs::create_dir(other.join(&id)).unwrap();
    std::fs::write(other.join(&id).join("space.bin"), b"foreign").unwrap();
    assert!(migration::transfer_record(&root, &other, &record).is_err());
    assert_eq!(
        std::fs::read(other.join(&id).join("space.bin")).unwrap(),
        b"foreign"
    );
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn corrupt_location_never_falls_back_and_saved_cleanup_cannot_escape_its_root() {
    let (dir, db, _a) = setup().await;
    db.transact(|ctx| {
        save(ctx.connection(), KEY, &Location::default())?;
        ctx.connection().execute(
            "UPDATE command_results SET request=?2 WHERE key=?1",
            (KEY, b"wrong-service"),
        )?;
        assert!(folder(ctx.connection()).is_err());
        save(ctx.connection(), KEY, &Location::default())?;
        let missing = serde_json::json!({"generation":0,"pending":null,"deletions":[]});
        save(ctx.connection(), KEY, &missing)?;
        assert!(folder(ctx.connection()).is_err());
        save(
            ctx.connection(),
            KEY,
            &Location {
                current: Some("relative".into()),
                ..Default::default()
            },
        )?;
        assert!(folder(ctx.connection()).is_err());
        save(ctx.connection(), KEY, &Location::default())?;
        Ok(())
    })
    .await
    .unwrap();
    let root = dir.path().join("source.db.backups");
    let outside = dir.path().join("keep.txt");
    std::fs::write(&outside, b"keep").unwrap();
    let id = "zedone-restore-candidate-fake";
    std::fs::create_dir(root.join(id)).unwrap();
    let forged = CopyRecord {
        id: id.into(),
        files: vec![FileRecord {
            name: "../../keep.txt".into(),
            size: 4,
            digest: Sha256::digest(b"keep").into(),
        }],
    };
    assert!(remove_record(&root, &forged).is_err());
    assert_eq!(std::fs::read(&outside).unwrap(), b"keep");
    db.shutdown().await.unwrap();
}
