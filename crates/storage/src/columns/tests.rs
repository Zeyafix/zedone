use super::*;
use crate::{Stage, Storage, access::PendingAccess};
use automerge::{ActorId, sync::State};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;
use zedone_core::ordering::Rank::Legacy;
use zedone_core::{
    access::{Clock, Credential, GrantScope, SpacePermission},
    columns::{self as core, *},
    command::Command,
    spaces::{self, CreateSpace},
    wire::CommandRequest,
};

struct Now;
impl Clock for Now {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_millis(100))
    }
}
fn sid(id: u8) -> String {
    format!("{id:032x}")
}
fn req<C: Command>(id: u8, parameters: C) -> CommandRequest<C> {
    CommandRequest {
        id: [id; 16],
        operation: C::NAME.into(),
        parameters,
    }
}
fn creation(id: u8, column: u8) -> CommandRequest<CreateColumn> {
    req(
        id,
        CreateColumn {
            space: sid(1),
            column: sid(column),
            name: "Ёж 空間 😀\n".into(),
        },
    )
}
fn rename_req(
    id: u8,
    column: u8,
    name: &str,
    expected: zedone_core::replay::StateVersion,
) -> CommandRequest<RenameColumn> {
    req(
        id,
        RenameColumn {
            space: sid(1),
            column: sid(column),
            name: name.into(),
            expected,
        },
    )
}
macro_rules! run {
    ($storage:expr, $key:expr, $handler:ident, $request:expr) => {{
        let secret = $key.credential().0.to_vec();
        let request = $request;
        $storage
            .transact(move |ctx| Ok(core::$handler(ctx, &Credential(&secret), &Now, &request)?))
            .await
    }};
}
fn rejected<T: std::fmt::Debug>(result: StorageResult<T>, code: ErrorCode) {
    assert_eq!(
        result.unwrap_err(),
        StorageError::Rejected(CommandError { code })
    );
}
async fn setup() -> (TempDir, Storage, Arc<PendingAccess>) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("columns.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    storage
        .transact(move |ctx| {
            ctx.bootstrap_access(&a)?;
            spaces::create(
                ctx,
                &a.credential(),
                &Now,
                &req(
                    250,
                    CreateSpace {
                        space: sid(1),
                        name: "Base".into(),
                    },
                ),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    (dir, storage, admin)
}
async fn fresh(storage: &Storage, admin: &PendingAccess, id: u8, column: u8) -> ColumnSnapshot {
    run!(
        storage,
        admin,
        read,
        req(
            id,
            ReadColumn {
                space: sid(1),
                column: sid(column)
            }
        )
    )
    .unwrap()
    .result
}

#[tokio::test]
async fn lifecycle_preserves_position_identity_groups_replays_and_restart() {
    let (dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let second = run!(storage, admin, create, creation(2, 2)).unwrap().result;
    assert_eq!(first.names, ["Ёж 空間 😀\n"]);
    assert_eq!(first.positions, [Legacy(0)]);
    assert_eq!(second.positions, [Legacy(1)]);
    let renamed = run!(storage, admin, rename, rename_req(3, 9, "", first.version))
        .unwrap()
        .result;
    let back = run!(
        storage,
        admin,
        rename,
        rename_req(4, 9, &first.names[0], renamed.version)
    )
    .unwrap()
    .result;
    assert_ne!(first.version, back.version);
    rejected(
        run!(
            storage,
            admin,
            trash,
            req(
                5,
                TrashColumn {
                    space: sid(1),
                    column: sid(9),
                    expected: first.version
                }
            )
        ),
        ErrorCode::StaleState,
    );
    let trash = req(
        6,
        TrashColumn {
            space: sid(1),
            column: sid(9),
            expected: back.version,
        },
    );
    let trashed = run!(storage, admin, trash, trash).unwrap().result;
    assert_eq!(trashed.state, SpaceState::Trashed);
    assert_eq!(trashed.deletion_groups.len(), 1);
    let visible = run!(
        storage,
        admin,
        list,
        req(
            7,
            ListColumns {
                space: sid(1),
                include_trashed: false
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(visible.columns, std::slice::from_ref(&second));
    let restored = run!(
        storage,
        admin,
        restore,
        req(
            8,
            RestoreColumn {
                space: sid(1),
                column: sid(9),
                expected: trashed.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(restored.state, SpaceState::Active);
    assert_eq!(restored.positions, first.positions);
    let replay = run!(
        storage,
        admin,
        trash,
        req(
            6,
            TrashColumn {
                space: sid(1),
                column: sid(9),
                expected: back.version
            }
        )
    )
    .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, trashed);
    assert!(
        run!(storage, admin, create, creation(1, 9))
            .unwrap()
            .replayed
    );
    rejected(
        run!(storage, admin, create, creation(1, 3)),
        ErrorCode::CommandMismatch,
    );
    rejected(
        run!(storage, admin, create, creation(9, 9)),
        ErrorCode::StaleState,
    );
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("columns.db")).await.unwrap();
    assert_eq!(fresh(&storage, &admin, 10, 9).await, restored);
    let listed = run!(
        storage,
        admin,
        list,
        req(
            11,
            ListColumns {
                space: sid(1),
                include_trashed: true
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(listed.columns, [restored.clone(), second]);
    let next_trash = run!(
        storage,
        admin,
        trash,
        req(
            12,
            TrashColumn {
                space: sid(1),
                column: sid(9),
                expected: restored.version
            }
        )
    )
    .unwrap()
    .result;
    assert_ne!(next_trash.deletion_groups, trashed.deletion_groups);
    let stale_restore = req(
        8,
        RestoreColumn {
            space: sid(1),
            column: sid(9),
            expected: trashed.version,
        },
    );
    assert_eq!(
        run!(storage, admin, restore, stale_restore).unwrap().result,
        restored
    );
    assert_eq!(fresh(&storage, &admin, 13, 9).await, next_trash);
}

#[tokio::test]
async fn projection_is_scoped_causal_and_unknown_descendants_fail_closed() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 1)).unwrap().result;
    let other = run!(storage, admin, create, creation(2, 2)).unwrap().result;
    run!(
        storage,
        admin,
        rename,
        rename_req(3, 2, "Other", other.version)
    )
    .unwrap();
    storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                let history = doc.put_object(ROOT, "history", ObjType::Map)?;
                doc.put(history, "flush", 1)?;
                Ok(())
            })?;
            ctx.set_peer_state(&sid(1), b"peer", &State::new())
        })
        .await
        .unwrap();
    assert_eq!(fresh(&storage, &admin, 4, 1).await, first);
    let before_space = storage
        .transact(|ctx| Ok(ctx.space_snapshot(&sid(1))?))
        .await
        .unwrap();
    storage
        .transact(|ctx| {
            let object = object(ctx.read().document(&sid(1))?, &sid(1))?;
            ctx.edit(&sid(1), |doc| {
                let cards = doc.put_object(&object, "cards", ObjType::Map)?;
                let card = doc.put_object(cards, "one", ObjType::Map)?;
                doc.put(card, "description", "Saved later")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let changed = fresh(&storage, &admin, 5, 1).await;
    assert!(!changed.empty);
    rejected(
        run!(
            storage,
            admin,
            trash,
            req(
                6,
                TrashColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: first.version
                }
            )
        ),
        ErrorCode::StaleState,
    );
    rejected(
        run!(
            storage,
            admin,
            trash,
            req(
                7,
                TrashColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: changed.version
                }
            )
        ),
        ErrorCode::InvalidRequest,
    );
    storage
        .transact(|ctx| {
            let object = object(ctx.read().document(&sid(1))?, &sid(1))?;
            ctx.edit(&sid(1), |doc| {
                doc.delete(object, "cards")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let empty = fresh(&storage, &admin, 8, 1).await;
    assert!(empty.empty);
    assert_ne!(empty.version, first.version);
    rejected(
        run!(
            storage,
            admin,
            trash,
            req(
                6,
                TrashColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: first.version
                }
            )
        ),
        ErrorCode::StaleState,
    );
    // Unknown root content may reference a column without being its child.
    storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                let opaque = doc.put_object(ROOT, "future_cards", ObjType::Map)?;
                doc.put(opaque, "assigned_column", sid(1))?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let opaque = fresh(&storage, &admin, 40, 1).await;
    assert!(!opaque.empty);
    assert_ne!(opaque.version, empty.version);
    rejected(
        run!(
            storage,
            admin,
            trash,
            req(
                41,
                TrashColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: empty.version
                }
            )
        ),
        ErrorCode::StaleState,
    );
    rejected(
        run!(
            storage,
            admin,
            trash,
            req(
                42,
                TrashColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: opaque.version
                }
            )
        ),
        ErrorCode::InvalidRequest,
    );
    storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                doc.delete(ROOT, "future_cards")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let clean = fresh(&storage, &admin, 43, 1).await;
    assert!(clean.empty);
    assert_ne!(clean.version, empty.version);
    rejected(
        run!(
            storage,
            admin,
            trash,
            req(
                44,
                TrashColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: empty.version
                }
            )
        ),
        ErrorCode::StaleState,
    );
    let trashed = run!(
        storage,
        admin,
        trash,
        req(
            45,
            TrashColumn {
                space: sid(1),
                column: sid(1),
                expected: clean.version
            }
        )
    )
    .unwrap()
    .result;
    storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                doc.put(ROOT, "future_cards", true)?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let opaque = fresh(&storage, &admin, 46, 1).await;
    assert_eq!(opaque.state, SpaceState::Trashed);
    assert!(!opaque.empty);
    rejected(
        run!(
            storage,
            admin,
            restore,
            req(
                47,
                RestoreColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: opaque.version
                }
            )
        ),
        ErrorCode::InvalidRequest,
    );
    storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                doc.delete(ROOT, "future_cards")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let clean = fresh(&storage, &admin, 48, 1).await;
    assert_ne!(clean.version, trashed.version);
    assert_eq!(
        run!(
            storage,
            admin,
            restore,
            req(
                49,
                RestoreColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: clean.version
                }
            )
        )
        .unwrap()
        .result
        .state,
        SpaceState::Active
    );
    let current_space = storage
        .transact(|ctx| Ok(ctx.space_snapshot(&sid(1))?))
        .await
        .unwrap();
    assert_ne!(before_space.version, current_space.version);
    let secret = admin.credential().0.to_vec();
    rejected(
        storage
            .transact(move |ctx| {
                Ok(spaces::trash(
                    ctx,
                    &Credential(&secret),
                    &Now,
                    &req(
                        9,
                        spaces::TrashSpace {
                            space: sid(1),
                            expected: before_space.version,
                        },
                    ),
                )?)
            })
            .await,
        ErrorCode::StaleState,
    );
    let secret = admin.credential().0.to_vec();
    rejected(
        storage
            .transact(move |ctx| {
                Ok(spaces::trash(
                    ctx,
                    &Credential(&secret),
                    &Now,
                    &req(
                        10,
                        spaces::TrashSpace {
                            space: sid(1),
                            expected: current_space.version,
                        },
                    ),
                )?)
            })
            .await,
        ErrorCode::InvalidRequest,
    );
}

#[tokio::test]
async fn conflicts_preserve_all_names_positions_and_lifecycle_variants() {
    for field in ["name", "position", "lifecycle"] {
        let (_dir, storage, admin) = setup().await;
        run!(storage, admin, create, creation(1, 1)).unwrap();
        storage
            .transact(move |ctx| {
                let graph = ctx.read().document(&sid(1))?;
                let object = object(graph, &sid(1))?;
                let mut a = graph.fork().with_actor(ActorId::from(vec![21]));
                let mut b = graph.fork().with_actor(ActorId::from(vec![22]));
                for (graph, n) in [(&mut a, 1u64), (&mut b, 2u64)] {
                    let mut tx = graph.transaction();
                    if field == "position" {
                        tx.put(&object, field, n)?;
                    } else if field == "lifecycle" {
                        tx.put(&object, field, format!("trash:{}:{n:032x}", sid(1)))?;
                    } else {
                        tx.put(&object, field, format!("Name {n}"))?;
                    }
                    tx.commit();
                }
                let mut all = a.get_changes(&[]);
                all.extend(b.get_changes(&[]));
                ctx.apply_changes(&sid(1), all)
            })
            .await
            .unwrap();
        let view = fresh(&storage, &admin, 2, 1).await;
        match field {
            "name" => assert_eq!(view.names.len(), 2),
            "position" => assert_eq!(view.positions, [Legacy(1), Legacy(2)]),
            _ => assert_eq!(view.state, SpaceState::Conflicted),
        }
        let listed = run!(
            storage,
            admin,
            list,
            req(
                3,
                ListColumns {
                    space: sid(1),
                    include_trashed: false
                }
            )
        )
        .unwrap()
        .result;
        assert_eq!(listed.columns, std::slice::from_ref(&view));
        rejected(
            run!(
                storage,
                admin,
                rename,
                rename_req(4, 1, "Destroy variants", view.version)
            ),
            ErrorCode::Conflict,
        );
        rejected(
            run!(
                storage,
                admin,
                trash,
                req(
                    5,
                    TrashColumn {
                        space: sid(1),
                        column: sid(1),
                        expected: view.version
                    }
                )
            ),
            ErrorCode::Conflict,
        );
        assert_eq!(fresh(&storage, &admin, 6, 1).await, view);
    }
}

#[tokio::test]
async fn readonly_revoked_expired_and_parent_lifecycle_are_checked_before_effect_or_replay() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 1)).unwrap().result;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &Now,
                &m,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
                None,
            )
        })
        .await
        .unwrap();
    assert_eq!(fresh(&storage, &member, 2, 1).await, first);
    rejected(
        run!(storage, member, create, creation(3, 2)),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(
            storage,
            member,
            rename,
            rename_req(4, 1, "No", first.version)
        ),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(
            storage,
            member,
            trash,
            req(
                5,
                TrashColumn {
                    space: sid(1),
                    column: sid(1),
                    expected: first.version
                }
            )
        ),
        ErrorCode::AccessDenied,
    );
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| ctx.revoke_access(&a.credential(), &Now, m.subject()))
        .await
        .unwrap();
    rejected(
        run!(
            storage,
            member,
            read,
            req(
                2,
                ReadColumn {
                    space: sid(1),
                    column: sid(1)
                }
            )
        ),
        ErrorCode::AccessDenied,
    );
    let expiring = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), expiring.clone());
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &Now,
                &m,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::ReadWrite)])),
                Some(UNIX_EPOCH + Duration::from_millis(101)),
            )
        })
        .await
        .unwrap();
    struct Later;
    impl Clock for Later {
        fn now(&self) -> CommandResult<SystemTime> {
            Ok(UNIX_EPOCH + Duration::from_millis(101))
        }
    }
    let m = expiring.clone();
    rejected(
        storage
            .transact(move |ctx| Ok(core::create(ctx, &m.credential(), &Later, &creation(6, 2))?))
            .await,
        ErrorCode::AccessDenied,
    );
    storage
        .transact(|ctx| {
            Ok(ctx.set_space_lifecycle(&sid(1), &format!("trash:{}:{}", sid(1), sid(20)))?)
        })
        .await
        .unwrap();
    rejected(
        run!(storage, admin, create, creation(7, 2)),
        ErrorCode::InvalidRequest,
    );
    rejected(
        run!(
            storage,
            admin,
            rename,
            rename_req(8, 1, "No", first.version)
        ),
        ErrorCode::InvalidRequest,
    );
    assert_eq!(fresh(&storage, &admin, 9, 1).await, first);
}

#[tokio::test]
async fn sql_and_commit_failures_roll_back_graph_and_receipt_and_unknown_completion_replays() {
    let (_dir, storage, admin) = setup().await;
    for (id, checkpoint) in [
        (1, Stage::AfterWork),
        (2, Stage::AfterChanges),
        (3, Stage::BeforeCommit),
    ] {
        let a = admin.clone();
        assert_eq!(
            storage
                .transact_checked(
                    move |ctx| Ok(core::create(ctx, &a.credential(), &Now, &creation(id, id))?),
                    move |stage| if stage == checkpoint {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                )
                .await
                .unwrap_err(),
            StorageError::Unavailable
        );
        rejected(
            run!(
                storage,
                admin,
                read,
                req(
                    20 + id,
                    ReadColumn {
                        space: sid(1),
                        column: sid(id)
                    }
                )
            ),
            ErrorCode::NotFound,
        );
        assert!(
            !run!(storage, admin, create, creation(id, id))
                .unwrap()
                .replayed
        );
    }
    let before = fresh(&storage, &admin, 30, 1).await;
    storage.transact(|ctx| { ctx.connection().execute_batch("CREATE TRIGGER fail_column_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?; Ok(()) }).await.unwrap();
    rejected(
        run!(
            storage,
            admin,
            rename,
            rename_req(4, 1, "SQL rollback", before.version)
        ),
        ErrorCode::Unavailable,
    );
    storage
        .transact(|ctx| {
            ctx.connection()
                .execute_batch("DROP TRIGGER fail_column_receipt")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(fresh(&storage, &admin, 31, 1).await, before);
    assert!(
        !run!(
            storage,
            admin,
            rename,
            rename_req(4, 1, "SQL rollback", before.version)
        )
        .unwrap()
        .replayed
    );
    let a = admin.clone();
    assert_eq!(
        storage
            .transact_checked(
                move |ctx| Ok(core::create(ctx, &a.credential(), &Now, &creation(5, 5))?),
                |stage| if stage == Stage::AfterCommit {
                    Err(StorageError::Unavailable)
                } else {
                    Ok(())
                }
            )
            .await
            .unwrap_err(),
        StorageError::OutcomeUnknown
    );
    assert!(
        run!(storage, admin, create, creation(5, 5))
            .unwrap()
            .replayed
    );
}

#[tokio::test]
async fn two_workers_do_not_duplicate_creation_and_stale_mutations_cannot_both_commit() {
    let (dir, storage, admin) = setup().await;
    let other = Storage::open(dir.path().join("columns.db")).await.unwrap();
    let a = admin.clone();
    let b = admin.clone();
    let (one, two) = tokio::join!(
        storage.transact(move |ctx| Ok(core::create(ctx, &a.credential(), &Now, &creation(1, 1))?)),
        other.transact(move |ctx| Ok(core::create(ctx, &b.credential(), &Now, &creation(1, 1))?))
    );
    let (one, two) = (one.unwrap(), two.unwrap());
    assert_ne!(one.replayed, two.replayed);
    assert_eq!(one.result, two.result);
    let expected = one.result.version;
    let a = admin.clone();
    let b = admin.clone();
    let (one, two) = tokio::join!(
        storage.transact(move |ctx| Ok(core::rename(
            ctx,
            &a.credential(),
            &Now,
            &rename_req(2, 1, "New name", expected)
        )?)),
        other.transact(move |ctx| Ok(core::trash(
            ctx,
            &b.credential(),
            &Now,
            &req(
                3,
                TrashColumn {
                    space: sid(1),
                    column: sid(1),
                    expected
                }
            )
        )?))
    );
    match (one, two) {
        (Ok(_), Err(e)) | (Err(e), Ok(_)) => assert_eq!(
            e,
            StorageError::Rejected(CommandError {
                code: ErrorCode::StaleState
            })
        ),
        result => panic!("Unexpected concurrent results: {result:?}"),
    }
}
