use super::*;
use crate::{Stage, access::PendingAccess};
use std::sync::Arc;
use zedone_core::{cards::CardStore, spaces::SpaceStore};

fn id(n: u8) -> String {
    format!("{n:032x}")
}

async fn archive() -> String {
    let (_root, source, _) = crate::restore_backup::tests::fixture().await;
    let saved = super::super::tests::archive(&source).await;
    source.shutdown().await.unwrap();
    saved
}

async fn target() -> (tempfile::TempDir, Storage, Arc<PendingAccess>) {
    let root = tempfile::tempdir().unwrap();
    let storage = Storage::open(root.path().join("new.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    storage
        .transact(move |ctx| {
            ctx.bootstrap_access(&a)?;
            ctx.create_space(&id(99), "Keep", a.subject())?;
            Ok(())
        })
        .await
        .unwrap();
    (root, storage, admin)
}

#[tokio::test]
async fn missing_recovery_preserves_full_graph_orphans_identity_and_isolated_local_state_after_restart()
 {
    let archive = archive().await;
    let (root, storage, admin) = target().await;
    let a = admin.clone();
    let source = source(&archive).unwrap();
    let count = source.changes().len();
    storage
        .transact(move |ctx| {
            let state = ctx.missing_recovery_state(&archive)?;
            assert_eq!(state.names, ["ПК", "Телефон"]);
            ctx.restore_missing_snapshot(
                &RestoreMissingSnapshot {
                    space: id(1),
                    archive,
                    expected: state.version,
                },
                a.subject(),
            )?;
            let restored = ctx.space_snapshot(&id(1))?;
            assert_eq!(restored.names, state.names);
            let card = ctx.card_snapshot(&id(1), &id(4))?;
            assert_eq!(card.descriptions, ["Незавершённая история"]);
            assert_eq!(card.tags, [id(7)]);
            assert_eq!(
                ctx.card_snapshot(&id(1), &id(5))?.state,
                zedone_core::spaces::SpaceState::Trashed
            );
            assert_eq!(ctx.space_snapshot(&id(99))?.names, ["Keep"]);
            let policy = ctx.recovery_state(&id(1))?;
            assert!(policy.paused);
            assert!(policy.backup.is_none());
            assert!(!policy.cleanup_pending);
            let graph = ctx.read().document(&id(1))?;
            assert_ne!(graph.get_heads(), graph::source(&source)?.get_heads());
            ctx.flush_document(&id(1))?;
            for change in source.changes() {
                let saved: Vec<u8> = ctx.connection().query_row(
                    "SELECT bytes FROM changes WHERE space=?1 AND hash=?2",
                    (id(1), change.hash().as_ref()),
                    |r| r.get(0),
                )?;
                assert_eq!(saved, change.raw_bytes());
            }
            assert_eq!(
                ctx.connection()
                    .query_row("SELECT count(*) FROM access_principals", [], |r| r
                        .get::<_, i64>(0))?,
                1
            );
            for table in [
                "peers",
                "card_editing",
                "pairing_requests",
                "sync_selections",
            ] {
                assert_eq!(
                    ctx.connection().query_row(
                        &format!("SELECT count(*) FROM {table}"),
                        [],
                        |r| r.get::<_, i64>(0)
                    )?,
                    0
                );
            }
            Ok(())
        })
        .await
        .unwrap();
    assert!(!root.path().join("new.db.backups").exists());
    storage.shutdown().await.unwrap();
    let storage = Storage::open(root.path().join("new.db")).await.unwrap();
    storage
        .transact(move |ctx| {
            assert!(ctx.recovery_state(&id(1))?.paused);
            let count_restored: i64 = ctx.connection().query_row(
                "SELECT count(*) FROM changes WHERE space=?1",
                [id(1)],
                |r| r.get(0),
            )?;
            assert!(count_restored > count as i64);
            assert!(
                !ctx.read()
                    .document(&id(1))?
                    .get_actor()
                    .to_bytes()
                    .is_empty()
            );
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn appearing_active_trashed_purged_empty_or_unreadable_document_never_uses_the_exception() {
    let archive = archive().await;
    for lifecycle in [
        "active",
        "trash:0000000000000000000000000000000a:0000000000000000000000000000000b",
        "purged",
        "empty",
        "unreadable",
    ] {
        let (_root, storage, admin) = target().await;
        let saved = archive.clone();
        let expected = storage
            .transact(move |ctx| Ok(ctx.missing_recovery_state(&saved)?.version))
            .await
            .unwrap();
        let a = admin.clone();
        storage
            .transact(move |ctx| {
                if lifecycle == "empty" {
                    ctx.create_document(&id(1))?;
                } else {
                    ctx.create_space(&id(1), "Appeared", a.subject())?;
                    if lifecycle == "unreadable" {
                        ctx.flush_document(&id(1))?;
                        ctx.connection()
                            .execute("UPDATE changes SET bytes=x'ff' WHERE space=?1", [id(1)])?;
                    } else {
                        ctx.set_space_lifecycle(&id(1), lifecycle)?;
                    }
                }
                Ok(())
            })
            .await
            .unwrap();
        let saved = archive.clone();
        let a = admin.clone();
        assert_eq!(
            storage
                .transact(move |ctx| Ok(ctx.restore_missing_snapshot(
                    &RestoreMissingSnapshot {
                        space: id(1),
                        archive: saved,
                        expected
                    },
                    a.subject()
                )?))
                .await
                .unwrap_err(),
            StorageError::Rejected(CommandError {
                code: ErrorCode::StaleState
            })
        );
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn read_failure_archive_tampering_and_wrong_confirmation_leave_no_partial_document() {
    let archive = archive().await;
    let (_root, storage, admin) = target().await;
    for mode in [
        "read_failure",
        "bad_archive",
        "wrong_version",
        "wrong_space",
        "grant_failure",
    ] {
        let saved = archive.clone();
        let a = admin.clone();
        let result = storage
            .transact(move |ctx| {
                let expected = ctx.missing_recovery_state(&saved)?.version;
                if mode == "read_failure" {
                    ctx.connection()
                        .execute("ALTER TABLE documents RENAME TO hidden_documents", [])?;
                }
                let mut request = RestoreMissingSnapshot {
                    space: id(1),
                    archive: saved,
                    expected,
                };
                if mode == "bad_archive" {
                    request.archive.replace_range(0..2, "ff");
                }
                if mode == "wrong_version" {
                    request.expected.0[0] ^= 1;
                }
                if mode == "wrong_space" {
                    request.space = id(2);
                }
                Ok(ctx.restore_missing_snapshot(
                    &request,
                    if mode == "grant_failure" {
                        "not-a-principal"
                    } else {
                        a.subject()
                    },
                )?)
            })
            .await;
        assert!(result.is_err(), "{mode}");
        storage
            .transact(|ctx| {
                absent(ctx.connection(), &id(1))?;
                assert_eq!(ctx.space_snapshot(&id(99))?.names, ["Keep"]);
                Ok(())
            })
            .await
            .unwrap();
    }
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn every_transaction_boundary_keeps_graph_grant_and_pause_together_after_restart() {
    let archive = archive().await;
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (root, storage, admin) = target().await;
        let saved = archive.clone();
        let a = admin.clone();
        let result = storage
            .transact_checked(
                move |ctx| {
                    let expected = ctx.missing_recovery_state(&saved)?.version;
                    Ok(ctx.restore_missing_snapshot(
                        &RestoreMissingSnapshot {
                            space: id(1),
                            archive: saved,
                            expected,
                        },
                        a.subject(),
                    )?)
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
        assert_eq!(
            result.unwrap_err(),
            if stage == Stage::AfterCommit {
                StorageError::OutcomeUnknown
            } else {
                StorageError::Unavailable
            }
        );
        storage.shutdown().await.unwrap();
        let storage = Storage::open(root.path().join("new.db")).await.unwrap();
        storage
            .transact(move |ctx| {
                if stage == Stage::AfterCommit {
                    assert!(ctx.recovery_state(&id(1))?.paused);
                    assert_eq!(ctx.space_snapshot(&id(1))?.names, ["ПК", "Телефон"]);
                } else {
                    absent(ctx.connection(), &id(1))?;
                    assert_eq!(
                        ctx.connection().query_row(
                            "SELECT count(*) FROM access_spaces WHERE space=?1",
                            [id(1)],
                            |r| r.get::<_, i64>(0)
                        )?,
                        0
                    );
                    assert!(!ctx.read().space_sync_control(&id(1))?.paused);
                }
                Ok(())
            })
            .await
            .unwrap();
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
#[ignore]
async fn missing_restore_crash_child() {
    let root = std::path::PathBuf::from(std::env::var_os("ZEDONE_MISSING_CRASH_ROOT").unwrap());
    let at = std::env::var("ZEDONE_MISSING_CRASH_STAGE").unwrap();
    let storage = Storage::open(root.join("new.db")).await.unwrap();
    let archive = std::fs::read_to_string(root.join("archive.hex")).unwrap();
    let credential = std::fs::read(root.join("credential.bin")).unwrap();
    storage
        .transact_checked(
            move |ctx| {
                let expected = ctx.missing_recovery_state(&archive)?.version;
                let request = zedone_core::wire::CommandRequest {
                    id: [42; 16],
                    operation: "space.recovery.restore_missing".into(),
                    parameters: RestoreMissingSnapshot {
                        space: id(1),
                        archive,
                        expected,
                    },
                };
                struct Now;
                impl zedone_core::access::Clock for Now {
                    fn now(&self) -> zedone_core::command::CommandResult<std::time::SystemTime> {
                        Ok(std::time::SystemTime::now())
                    }
                }
                Ok(zedone_core::recovery::missing::restore(
                    ctx,
                    &zedone_core::access::Credential(&credential),
                    &Now,
                    &request,
                )?)
            },
            move |stage| {
                if matches!(
                    (at.as_str(), stage),
                    ("after_work", Stage::AfterWork)
                        | ("after_changes", Stage::AfterChanges)
                        | ("before_commit", Stage::BeforeCommit)
                        | ("after_commit", Stage::AfterCommit)
                ) {
                    std::process::exit(86);
                }
                Ok(())
            },
        )
        .await
        .unwrap();
    panic!("crash checkpoint was not reached");
}

#[tokio::test]
async fn actual_process_exit_never_leaves_a_partial_restored_space_or_grant() {
    let archive = archive().await;
    for stage in [
        "after_work",
        "after_changes",
        "before_commit",
        "after_commit",
    ] {
        let (root, storage, admin) = target().await;
        storage.shutdown().await.unwrap();
        std::fs::write(root.path().join("archive.hex"), &archive).unwrap();
        std::fs::write(root.path().join("credential.bin"), admin.credential().0).unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--ignored",
                "--exact",
                "recovery::missing::tests::missing_restore_crash_child",
                "--nocapture",
            ])
            .env("ZEDONE_MISSING_CRASH_ROOT", root.path())
            .env("ZEDONE_MISSING_CRASH_STAGE", stage)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(86));
        let storage = Storage::open(root.path().join("new.db")).await.unwrap();
        storage
            .transact(move |ctx| {
                if stage == "after_commit" {
                    assert!(ctx.recovery_state(&id(1))?.paused);
                    assert_eq!(ctx.space_snapshot(&id(1))?.names, ["ПК", "Телефон"]);
                    assert_eq!(
                        ctx.connection().query_row(
                            "SELECT count(*) FROM access_spaces WHERE space=?1",
                            [id(1)],
                            |r| r.get::<_, i64>(0)
                        )?,
                        1
                    );
                } else {
                    absent(ctx.connection(), &id(1))?;
                    assert!(!ctx.read().space_sync_control(&id(1))?.paused);
                    assert_eq!(
                        ctx.connection().query_row(
                            "SELECT count(*) FROM access_spaces WHERE space=?1",
                            [id(1)],
                            |r| r.get::<_, i64>(0)
                        )?,
                        0
                    );
                }
                Ok(())
            })
            .await
            .unwrap();
        storage.shutdown().await.unwrap();
    }
}
