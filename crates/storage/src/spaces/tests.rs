use super::*;
use crate::{
    RecordedResult, Stage, Storage,
    access::{AccessKey, PendingAccess},
};
use automerge::{ActorId, ObjType, sync::State};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;
use zedone_core::{
    access::{Clock, Credential, GrantScope, SpacePermission},
    command::Command,
    replay::StateVersion,
    spaces::{self as core, *},
    wire::CommandRequest,
};

struct Now;
impl Clock for Now {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_millis(100))
    }
}
fn sid(n: u8) -> String {
    format!("{n:032x}")
}
fn req<C: Command>(id: u8, parameters: C) -> CommandRequest<C> {
    CommandRequest {
        id: [id; 16],
        operation: C::NAME.into(),
        parameters,
    }
}
fn creation(id: u8, space: u8, name: &str) -> CommandRequest<CreateSpace> {
    req(
        id,
        CreateSpace {
            space: sid(space),
            name: name.into(),
        },
    )
}
fn rename_req(
    id: u8,
    space: u8,
    name: &str,
    expected: StateVersion,
) -> CommandRequest<RenameSpace> {
    req(
        id,
        RenameSpace {
            space: sid(space),
            name: name.into(),
            expected,
        },
    )
}
fn trash_req(id: u8, space: u8, expected: StateVersion) -> CommandRequest<TrashSpace> {
    req(
        id,
        TrashSpace {
            space: sid(space),
            expected,
        },
    )
}
fn restore_req(id: u8, space: u8, expected: StateVersion) -> CommandRequest<RestoreSpace> {
    req(
        id,
        RestoreSpace {
            space: sid(space),
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
async fn setup() -> (TempDir, Storage, Arc<PendingAccess>) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("spaces.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    storage
        .transact(move |ctx| ctx.bootstrap_access(&a))
        .await
        .unwrap();
    (dir, storage, admin)
}
async fn fresh(storage: &Storage, key: &PendingAccess, id: u8, space: u8) -> SpaceSnapshot {
    run!(storage, key, read, req(id, ReadSpace { space: sid(space) }))
        .unwrap()
        .result
}
fn rejected<T: std::fmt::Debug>(result: StorageResult<T>, code: ErrorCode) {
    assert_eq!(
        result.unwrap_err(),
        StorageError::Rejected(CommandError { code })
    );
}

#[tokio::test]
async fn creation_keeps_existing_modes_and_admin_separate_and_replay_never_regrants_removed_rights()
{
    let (_dir, storage, admin) = setup().await;
    let original = run!(&storage, admin, create, creation(1, 1, "Base")).unwrap();
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
    let created = run!(
        &storage,
        member,
        create,
        creation(2, 2, "Ёж 空間 😀\nnotes")
    )
    .unwrap();
    assert_eq!(created.result.names, ["Ёж 空間 😀\nnotes"]);
    rejected(
        run!(
            &storage,
            member,
            rename,
            rename_req(3, 1, "Forbidden", original.result.version)
        ),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(&storage, admin, read, req(4, ReadSpace { space: sid(2) })),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(&storage, admin, create, creation(5, 2, "Cannot steal")),
        ErrorCode::StaleState,
    );
    assert_eq!(
        fresh(&storage, &admin, 6, 1).await.version,
        original.result.version
    );
    let own = run!(
        &storage,
        member,
        list,
        req(
            7,
            ListSpaces {
                include_trashed: false
            }
        )
    )
    .unwrap();
    assert_eq!(own.result.spaces.len(), 2);
    let admin_list = run!(
        &storage,
        admin,
        list,
        req(
            8,
            ListSpaces {
                include_trashed: true
            }
        )
    )
    .unwrap();
    assert_eq!(admin_list.result.spaces.len(), 1);
    let replacement = Arc::new(AccessKey::generate().unwrap());
    let (a, m, r) = (admin.clone(), member.clone(), replacement.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(
                &a.credential(),
                &Now,
                m.subject(),
                &r,
                &GrantScope::Spaces(BTreeMap::from([
                    (sid(1), SpacePermission::Read),
                    (sid(2), SpacePermission::Read),
                ])),
                None,
            )
        })
        .await
        .unwrap();
    let replay = run!(
        &storage,
        replacement,
        create,
        creation(2, 2, "Ёж 空間 😀\nnotes")
    )
    .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, created.result);
    rejected(
        run!(
            &storage,
            replacement,
            rename,
            rename_req(9, 2, "No regrant", created.result.version)
        ),
        ErrorCode::AccessDenied,
    );
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            assert!(
                ctx.inspect_access(&a.credential(), &Now, m.subject())?
                    .unwrap()
                    .scope
                    .spaces()
                    .unwrap()
                    .values()
                    .all(|p| *p == SpacePermission::Read)
            );
            Ok(())
        })
        .await
        .unwrap();
    let limited = Arc::new(AccessKey::generate().unwrap());
    let (a, m, k) = (admin.clone(), member.clone(), limited.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(
                &a.credential(),
                &Now,
                m.subject(),
                &k,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
                None,
            )
        })
        .await
        .unwrap();
    rejected(
        run!(
            &storage,
            limited,
            list,
            req(
                7,
                ListSpaces {
                    include_trashed: false
                }
            )
        ),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(
            &storage,
            limited,
            read,
            req(10, ReadSpace { space: sid(2) })
        ),
        ErrorCode::AccessDenied,
    );
    assert_eq!(
        run!(
            &storage,
            limited,
            list,
            req(
                11,
                ListSpaces {
                    include_trashed: false
                }
            )
        )
        .unwrap()
        .result
        .spaces
        .len(),
        1
    );
    assert!(
        run!(
            &storage,
            limited,
            create,
            creation(2, 2, "Ёж 空間 😀\nnotes")
        )
        .unwrap()
        .replayed
    );
}

#[tokio::test]
async fn failed_rename_trash_and_restore_keep_the_original_graph_and_can_retry_the_same_intention()
{
    let (_dir, storage, admin) = setup().await;
    let first = run!(&storage, admin, create, creation(1, 1, "Original"))
        .unwrap()
        .result;
    macro_rules! rollback {
        ($handler:ident, $request:expr, $before:expr, $read_id:expr) => {{
            let secret = admin.credential().0.to_vec();
            let request = $request;
            assert_eq!(
                storage
                    .transact_checked(
                        move |ctx| Ok(core::$handler(ctx, &Credential(&secret), &Now, &request)?),
                        |stage| if stage == Stage::AfterChanges {
                            Err(StorageError::Unavailable)
                        } else {
                            Ok(())
                        }
                    )
                    .await
                    .unwrap_err(),
                StorageError::Unavailable
            );
            assert_eq!(fresh(&storage, &admin, $read_id, 1).await, $before);
        }};
    }
    rollback!(
        rename,
        rename_req(2, 1, "Changed", first.version),
        first,
        20
    );
    let renamed = run!(
        &storage,
        admin,
        rename,
        rename_req(2, 1, "Changed", first.version)
    )
    .unwrap();
    assert!(!renamed.replayed);
    rollback!(
        trash,
        trash_req(3, 1, renamed.result.version),
        renamed.result,
        21
    );
    let trashed = run!(
        &storage,
        admin,
        trash,
        trash_req(3, 1, renamed.result.version)
    )
    .unwrap();
    assert!(!trashed.replayed);
    rollback!(
        restore,
        restore_req(4, 1, trashed.result.version),
        trashed.result,
        22
    );
    assert!(
        !run!(
            &storage,
            admin,
            restore,
            restore_req(4, 1, trashed.result.version)
        )
        .unwrap()
        .replayed
    );
}

#[tokio::test]
async fn lifecycle_replays_preserve_late_renames_restore_groups_and_restart_identity() {
    let (dir, storage, admin) = setup().await;
    let first = run!(&storage, admin, create, creation(1, 1, "Alpha")).unwrap();
    let beta = run!(
        &storage,
        admin,
        rename,
        rename_req(2, 1, "Beta", first.result.version)
    )
    .unwrap();
    let alpha = run!(
        &storage,
        admin,
        rename,
        rename_req(3, 1, "Alpha", beta.result.version)
    )
    .unwrap();
    assert_ne!(first.result.version, alpha.result.version);
    rejected(
        run!(
            &storage,
            admin,
            trash,
            trash_req(4, 1, first.result.version)
        ),
        ErrorCode::StaleState,
    );
    let trashed = run!(
        &storage,
        admin,
        trash,
        trash_req(5, 1, alpha.result.version)
    )
    .unwrap();
    assert_eq!(trashed.result.state, SpaceState::Trashed);
    assert_eq!(trashed.result.deletion_groups.len(), 1);
    assert!(
        run!(
            &storage,
            admin,
            list,
            req(
                6,
                ListSpaces {
                    include_trashed: false
                }
            )
        )
        .unwrap()
        .result
        .spaces
        .is_empty()
    );
    assert_eq!(
        run!(
            &storage,
            admin,
            list,
            req(
                7,
                ListSpaces {
                    include_trashed: true
                }
            )
        )
        .unwrap()
        .result
        .spaces,
        std::slice::from_ref(&trashed.result)
    );
    let restored = run!(
        &storage,
        admin,
        restore,
        restore_req(8, 1, trashed.result.version)
    )
    .unwrap();
    assert_eq!(restored.result.state, SpaceState::Active);
    let old = run!(
        &storage,
        admin,
        trash,
        trash_req(5, 1, alpha.result.version)
    )
    .unwrap();
    assert!(old.replayed);
    assert_eq!(old.result, trashed.result);
    assert_eq!(fresh(&storage, &admin, 9, 1).await, restored.result);
    let second = run!(
        &storage,
        admin,
        trash,
        trash_req(10, 1, restored.result.version)
    )
    .unwrap();
    assert_ne!(
        second.result.deletion_groups,
        trashed.result.deletion_groups
    );
    assert!(
        run!(
            &storage,
            admin,
            restore,
            restore_req(8, 1, trashed.result.version)
        )
        .unwrap()
        .replayed
    );
    assert_eq!(fresh(&storage, &admin, 11, 1).await, second.result);
    rejected(
        run!(
            &storage,
            admin,
            trash,
            trash_req(5, 1, second.result.version)
        ),
        ErrorCode::CommandMismatch,
    );
    rejected(
        run!(
            &storage,
            admin,
            rename,
            rename_req(5, 1, "other operation", second.result.version)
        ),
        ErrorCode::CommandMismatch,
    );
    let active = run!(
        &storage,
        admin,
        restore,
        restore_req(12, 1, second.result.version)
    )
    .unwrap();
    assert!(
        run!(
            &storage,
            admin,
            rename,
            rename_req(2, 1, "Beta", first.result.version)
        )
        .unwrap()
        .replayed
    );
    let actor = storage
        .read(|ctx| Ok(ctx.document(&sid(1))?.get_actor().to_bytes().to_vec()))
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("spaces.db")).await.unwrap();
    let replay = run!(&storage, admin, create, creation(1, 1, "Alpha")).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, first.result);
    assert_eq!(fresh(&storage, &admin, 13, 1).await, active.result);
    storage
        .read(move |ctx| {
            assert_eq!(ctx.document(&sid(1))?.get_actor().to_bytes(), actor);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn semantic_version_retains_removed_membership_but_ignores_technical_history_and_other_scopes()
 {
    let (_dir, storage, admin) = setup().await;
    let first = run!(&storage, admin, create, creation(1, 1, "Stable"))
        .unwrap()
        .result;
    storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                let h = doc.put_object(ROOT, "history", ObjType::Map)?;
                let nested = doc.put_object(h, "session", ObjType::Map)?;
                doc.put(nested, "flush", true)?;
                Ok(())
            })?;
            ctx.set_peer_state(&sid(1), b"peer", &State::new())?;
            ctx.record(
                b"technical-read",
                &RecordedResult {
                    request: vec![1],
                    result: vec![2],
                },
            )
        })
        .await
        .unwrap();
    run!(&storage, admin, create, creation(2, 2, "Unrelated")).unwrap();
    assert_eq!(fresh(&storage, &admin, 3, 1).await.version, first.version);
    let changed = storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                let c = doc.put_object(ROOT, "columns", ObjType::Map)?;
                let child = doc.put_object(c, "one", ObjType::Map)?;
                doc.put(child, "name", "Content")?;
                Ok(())
            })?;
            Ok(ctx.space_snapshot(&sid(1))?)
        })
        .await
        .unwrap();
    assert!(!changed.empty);
    rejected(
        run!(&storage, admin, trash, trash_req(4, 1, first.version)),
        ErrorCode::StaleState,
    );
    rejected(
        run!(&storage, admin, trash, trash_req(5, 1, changed.version)),
        ErrorCode::InvalidRequest,
    );
    let empty = storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                doc.delete(ROOT, "columns")?;
                Ok(())
            })?;
            Ok(ctx.space_snapshot(&sid(1))?)
        })
        .await
        .unwrap();
    assert!(empty.empty);
    assert_ne!(empty.version, first.version);
    assert_ne!(empty.version, changed.version);
    rejected(
        run!(&storage, admin, trash, trash_req(4, 1, first.version)),
        ErrorCode::StaleState,
    );
    let trashed = run!(&storage, admin, trash, trash_req(6, 1, empty.version)).unwrap();
    assert_eq!(trashed.result.state, SpaceState::Trashed);
}

#[tokio::test]
async fn conflicts_are_visible_once_sorted_first_and_cannot_be_overwritten_by_regular_mutations() {
    let (_dir, storage, admin) = setup().await;
    run!(&storage, admin, create, creation(1, 1, "Common")).unwrap();
    run!(&storage, admin, create, creation(2, 2, "alpha")).unwrap();
    run!(&storage, admin, create, creation(3, 3, "Alpha")).unwrap();
    storage
        .transact(|ctx| {
            let graph = ctx.read().document(&sid(1))?;
            let mut a = graph.fork().with_actor(ActorId::from(vec![21]));
            let mut b = graph.fork().with_actor(ActorId::from(vec![22]));
            for (graph, name) in [(&mut a, "Zulu"), (&mut b, "Beta")] {
                let mut tx = graph.transaction();
                tx.put(ROOT, "name", name)?;
                tx.commit();
            }
            let mut all = a.get_changes(&[]);
            all.extend(b.get_changes(&[]));
            ctx.apply_changes(&sid(1), all)
        })
        .await
        .unwrap();
    let view = fresh(&storage, &admin, 4, 1).await;
    assert_eq!(view.names, ["Beta", "Zulu"]);
    let list = run!(
        &storage,
        admin,
        list,
        req(
            5,
            ListSpaces {
                include_trashed: false
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(
        list.spaces
            .iter()
            .map(|s| s.space.clone())
            .collect::<Vec<_>>(),
        [sid(1), sid(3), sid(2)]
    );
    rejected(
        run!(
            &storage,
            admin,
            rename,
            rename_req(6, 1, "Overwrite", view.version)
        ),
        ErrorCode::Conflict,
    );
    rejected(
        run!(&storage, admin, trash, trash_req(7, 1, view.version)),
        ErrorCode::Conflict,
    );
    assert_eq!(fresh(&storage, &admin, 8, 1).await, view);
}

#[tokio::test]
async fn sql_failures_and_every_commit_boundary_preserve_atomic_creation_and_unknown_replay() {
    let (_dir, storage, admin) = setup().await;
    for (n, stage) in [
        (1, Stage::AfterWork),
        (2, Stage::AfterChanges),
        (3, Stage::BeforeCommit),
    ] {
        let secret = admin.credential().0.to_vec();
        assert_eq!(
            storage
                .transact_checked(
                    move |ctx| Ok(core::create(
                        ctx,
                        &Credential(&secret),
                        &Now,
                        &creation(n, n, "Rollback")
                    )?),
                    move |s| if s == stage {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                )
                .await
                .unwrap_err(),
            StorageError::Unavailable
        );
        storage
            .transact(|ctx| {
                for table in ["documents", "changes", "access_spaces", "command_results"] {
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
    }
    // A real SQL abort after document/grant creation but before saving receipt.
    storage.transact(|ctx| {ctx.connection().execute_batch("CREATE TEMP TRIGGER fail_result BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT,'test receipt write failure'); END;")?; Ok(())}).await.unwrap();
    assert!(run!(&storage, admin, create, creation(4, 4, "SQL failure")).is_err());
    storage
        .transact(|ctx| {
            ctx.connection().execute_batch("DROP TRIGGER fail_result")?;
            assert_eq!(
                ctx.connection()
                    .query_row("SELECT count(*) FROM documents", [], |r| r.get::<_, i64>(0))?,
                0
            );
            assert_eq!(
                ctx.connection()
                    .query_row("SELECT count(*) FROM access_spaces", [], |r| r
                        .get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
    let secret = admin.credential().0.to_vec();
    let unknown = storage
        .transact_checked(
            move |ctx| {
                Ok(core::create(
                    ctx,
                    &Credential(&secret),
                    &Now,
                    &creation(5, 5, "Lost acknowledgement"),
                )?)
            },
            |stage| {
                if stage == Stage::AfterCommit {
                    Err(StorageError::Unavailable)
                } else {
                    Ok(())
                }
            },
        )
        .await;
    assert_eq!(unknown.unwrap_err(), StorageError::OutcomeUnknown);
    let retry = run!(
        &storage,
        admin,
        create,
        creation(5, 5, "Lost acknowledgement")
    )
    .unwrap();
    assert!(retry.replayed);
    storage
        .transact(|ctx| {
            for table in ["documents", "changes", "access_spaces", "command_results"] {
                assert_eq!(
                    ctx.connection().query_row(
                        &format!("SELECT count(*) FROM {table}"),
                        [],
                        |r| r.get::<_, i64>(0)
                    )?,
                    1
                );
            }
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn concurrent_workers_execute_one_creation_and_reject_a_stale_competing_rename() {
    let (dir, storage, admin) = setup().await;
    let other = Storage::open(dir.path().join("spaces.db")).await.unwrap();
    let (a, b) = tokio::join!(
        async { run!(&storage, admin, create, creation(1, 1, "Once")) },
        async { run!(&other, admin, create, creation(1, 1, "Once")) }
    );
    let (a, b) = (a.unwrap(), b.unwrap());
    assert_ne!(a.replayed, b.replayed);
    assert_eq!(a.result, b.result);
    let version = a.result.version;
    let (a, b) = tokio::join!(
        async { run!(&storage, admin, rename, rename_req(2, 1, "First", version)) },
        async { run!(&other, admin, rename, rename_req(3, 1, "Second", version)) }
    );
    assert_ne!(a.is_ok(), b.is_ok());
    rejected(if a.is_err() { a } else { b }, ErrorCode::StaleState);
    assert_eq!(fresh(&storage, &admin, 4, 1).await.names.len(), 1);
}

#[tokio::test]
async fn expiry_after_creation_work_rolls_back_the_space_and_its_new_permission() {
    let (_dir, storage, admin) = setup().await;
    run!(&storage, admin, create, creation(1, 1, "Existing")).unwrap();
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &Now,
                &m,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
                Some(UNIX_EPOCH + Duration::from_millis(200)),
            )
        })
        .await
        .unwrap();
    struct Expiring(std::cell::Cell<u64>);
    impl Clock for Expiring {
        fn now(&self) -> CommandResult<SystemTime> {
            let time = self.0.get();
            self.0.set(200);
            Ok(UNIX_EPOCH + Duration::from_millis(time))
        }
    }
    let m = member.clone();
    rejected(
        storage
            .transact(move |ctx| {
                Ok(core::create(
                    ctx,
                    &m.credential(),
                    &Expiring(std::cell::Cell::new(100)),
                    &creation(2, 2, "Expired"),
                )?)
            })
            .await,
        ErrorCode::AccessDenied,
    );
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            assert_eq!(
                ctx.inspect_access(&a.credential(), &Now, m.subject())?
                    .unwrap()
                    .scope
                    .spaces()
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                ctx.connection()
                    .query_row("SELECT count(*) FROM documents", [], |r| r.get::<_, i64>(0))?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[test]
fn projection_converges_across_delivery_order_and_rejects_unsupported_or_invalid_headers() {
    let mut base = Automerge::new();
    let mut tx = base.transaction();
    tx.put(ROOT, "format", "zedone.space.v1").unwrap();
    tx.put(ROOT, "name", "Base").unwrap();
    tx.put(ROOT, "lifecycle", "active").unwrap();
    tx.commit();
    let mut a = base.fork();
    let mut b = base.fork();
    for (graph, name) in [(&mut a, "One"), (&mut b, "Two")] {
        let mut tx = graph.transaction();
        tx.put(ROOT, "name", name).unwrap();
        tx.commit();
    }
    let mut left = base.clone();
    let mut right = base.clone();
    left.apply_changes(a.get_changes(&[])).unwrap();
    left.apply_changes(b.get_changes(&[])).unwrap();
    right.apply_changes(b.get_changes(&[])).unwrap();
    right.apply_changes(a.get_changes(&[])).unwrap();
    assert_eq!(
        snapshot(&left, &sid(1)).unwrap(),
        snapshot(&right, &sid(1)).unwrap()
    );
    for (key, value) in [
        ("format", "future.space.v2"),
        ("lifecycle", "not a deletion group"),
    ] {
        let mut graph = base.clone();
        let mut tx = graph.transaction();
        tx.put(ROOT, key, value).unwrap();
        tx.commit();
        assert_eq!(
            snapshot(&graph, &sid(1)),
            Err(StorageError::InvalidDatabase)
        );
    }
}
