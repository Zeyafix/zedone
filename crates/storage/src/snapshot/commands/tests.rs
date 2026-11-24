use super::*;
use crate::{Stage, Storage, access::PendingAccess};
use std::{
    cell::Cell,
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;
use zedone_core::{
    access::{Clock, GrantScope, SpacePermission},
    command::Command,
    snapshots::{self, ExportSpace, ImportSpace},
    spaces::{CreateSpace, SpaceStore},
    wire::CommandRequest,
};

const SOURCE: &str = "00000000000000000000000000000001";
struct At(u64);
impl Clock for At {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_millis(self.0))
    }
}
fn req<C: Command>(id: u8, parameters: C) -> CommandRequest<C> {
    CommandRequest {
        id: [id; 16],
        operation: C::NAME.into(),
        parameters,
    }
}

async fn fixture() -> (TempDir, Storage, Arc<PendingAccess>, String) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("transfer.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    let archive = storage
        .transact(move |ctx| {
            ctx.bootstrap_access(&a)?;
            zedone_core::spaces::create(
                ctx,
                &a.credential(),
                &At(10),
                &req(
                    1,
                    CreateSpace {
                        space: SOURCE.into(),
                        name: "Original".into(),
                    },
                ),
            )?;
            Ok(snapshots::export(
                ctx,
                &a.credential(),
                &At(10),
                &req(
                    2,
                    ExportSpace {
                        space: SOURCE.into(),
                    },
                ),
            )?
            .result
            .archive)
        })
        .await
        .unwrap();
    (dir, storage, admin, archive)
}

async fn counts(storage: &Storage) -> Vec<i64> {
    storage
        .read(|ctx| {
            [
                "documents",
                "changes",
                "access_spaces",
                "command_results",
                "peers",
                "card_editing",
            ]
            .iter()
            .map(|table| {
                ctx.connection()
                    .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                    .map_err(StorageError::from)
            })
            .collect()
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn public_import_receipt_and_access_share_every_commit_boundary_and_unknown_replays() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (dir, storage, admin, archive) = fixture().await;
        let initial = counts(&storage).await;
        let (a, bytes) = (admin.clone(), archive.clone());
        let result = storage
            .transact_checked(
                move |ctx| {
                    Ok(snapshots::import(
                        ctx,
                        &a.credential(),
                        &At(10),
                        &req(3, ImportSpace { archive: bytes }),
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
        let storage = Storage::open(dir.path().join("transfer.db")).await.unwrap();
        let after = counts(&storage).await;
        if stage == Stage::AfterCommit {
            assert_eq!(after[0], initial[0] + 1);
            assert_eq!(after[2], initial[2] + 1);
            assert_eq!(after[3], initial[3] + 1);
        } else {
            assert_eq!(after, initial);
        }
        let a = admin.clone();
        let imported = storage
            .transact(move |ctx| {
                Ok(snapshots::import(
                    ctx,
                    &a.credential(),
                    &At(10),
                    &req(3, ImportSpace { archive }),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(imported.replayed, stage == Stage::AfterCommit);
        let end = counts(&storage).await;
        assert_eq!(end[0], initial[0] + 1);
        assert_eq!(end[2], initial[2] + 1);
        assert_eq!(end[3], initial[3] + 1);
        assert_eq!(&end[4..], &initial[4..]);
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn failed_grant_or_receipt_rolls_back_the_whole_public_import() {
    for table in ["access_spaces", "command_results"] {
        let (_dir, storage, admin, archive) = fixture().await;
        let initial = counts(&storage).await;
        storage.run(move |connection| {
            connection.execute_batch(&format!("CREATE TRIGGER fail_transfer BEFORE INSERT ON {table} BEGIN SELECT RAISE(ABORT, 'transfer failure'); END;"))?;
            Ok(())
        }).await.unwrap();
        let a = admin.clone();
        let failed = storage
            .transact(move |ctx| {
                assert!(
                    snapshots::import(
                        ctx,
                        &a.credential(),
                        &At(10),
                        &req(3, ImportSpace { archive })
                    )
                    .is_err()
                );
                Ok(())
            })
            .await;
        assert_eq!(failed.unwrap_err(), StorageError::Unavailable);
        assert_eq!(counts(&storage).await, initial);
        storage.shutdown().await.unwrap();
    }
}

struct Expiring(Cell<u64>);
impl Clock for Expiring {
    fn now(&self) -> CommandResult<SystemTime> {
        let value = self.0.get();
        self.0.set(value + 50);
        At(value).now()
    }
}

#[tokio::test]
async fn expiry_during_capture_or_publication_rolls_back_results_and_access() {
    for exporting in [true, false] {
        let (_dir, storage, admin, archive) = fixture().await;
        let member = Arc::new(PendingAccess::generate().unwrap());
        let (a, m) = (admin.clone(), member.clone());
        storage
            .transact(move |ctx| {
                ctx.issue_access(
                    &a.credential(),
                    &At(10),
                    &m,
                    &GrantScope::Spaces(BTreeMap::from([(SOURCE.into(), SpacePermission::Read)])),
                    Some(UNIX_EPOCH + Duration::from_millis(150)),
                )
            })
            .await
            .unwrap();
        let initial = counts(&storage).await;
        let result = storage
            .transact(move |ctx| {
                let clock = Expiring(Cell::new(100));
                if exporting {
                    snapshots::export(
                        ctx,
                        &member.credential(),
                        &clock,
                        &req(
                            3,
                            ExportSpace {
                                space: SOURCE.into(),
                            },
                        ),
                    )?;
                } else {
                    snapshots::import(
                        ctx,
                        &member.credential(),
                        &clock,
                        &req(3, ImportSpace { archive }),
                    )?;
                }
                Ok(())
            })
            .await;
        assert_eq!(
            result.unwrap_err(),
            StorageError::Rejected(CommandError {
                code: ErrorCode::AccessDenied
            })
        );
        assert_eq!(counts(&storage).await, initial);
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn export_budget_refuses_full_archive_without_a_partial_receipt_or_source_changes() {
    let (_dir, storage, admin, _archive) = fixture().await;
    storage
        .transact(|ctx| {
            ctx.rename_space(SOURCE, &"x".repeat(MAX_ARCHIVE_BYTES))?;
            Ok(())
        })
        .await
        .unwrap();
    let initial = counts(&storage).await;
    let result = storage
        .transact(move |ctx| {
            Ok(snapshots::export(
                ctx,
                &admin.credential(),
                &At(10),
                &req(
                    3,
                    ExportSpace {
                        space: SOURCE.into(),
                    },
                ),
            )?)
        })
        .await;
    assert_eq!(
        result.unwrap_err(),
        StorageError::Rejected(CommandError {
            code: ErrorCode::Unavailable
        })
    );
    assert_eq!(counts(&storage).await, initial);
    storage.shutdown().await.unwrap();
}
