use super::*;
use crate::{Stage, Storage, access::PendingAccess};
use automerge::ActorId;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;
use zedone_core::{
    access::{Clock, Credential, GrantScope, SpacePermission},
    cards::{
        self as core, CreateCard, EditCard, ListCards, ReadCard, ReadCardHistory, RestoreCard,
        TrashCard,
    },
    columns::{ColumnStore, CreateColumn, TrashColumn},
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
fn creation(id: u8, card: u8) -> CommandRequest<CreateCard> {
    req(
        id,
        CreateCard {
            space: sid(1),
            card: sid(card),
            column: sid(2),
            title: "Ёж 空間 😀".into(),
            description: "<script>raw</script>\n**text**".into(),
        },
    )
}
fn edit_req(
    id: u8,
    card: u8,
    title: &str,
    description: &str,
    expected: zedone_core::replay::StateVersion,
) -> CommandRequest<EditCard> {
    req(
        id,
        EditCard {
            space: sid(1),
            card: sid(card),
            title: title.into(),
            description: description.into(),
            expected,
        },
    )
}
macro_rules! run {
    ($storage:expr,$key:expr,$handler:ident,$request:expr) => {{
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
    let storage = Storage::open(dir.path().join("cards.db")).await.unwrap();
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
                        name: "Board".into(),
                    },
                ),
            )?;
            for id in [2, 3] {
                zedone_core::columns::create(
                    ctx,
                    &a.credential(),
                    &Now,
                    &req(
                        250 + id,
                        CreateColumn {
                            space: sid(1),
                            column: sid(id),
                            name: id.to_string(),
                        },
                    ),
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    (dir, storage, admin)
}
async fn fresh(storage: &Storage, admin: &PendingAccess, id: u8, card: u8) -> CardSnapshot {
    run!(
        storage,
        admin,
        read,
        req(
            id,
            ReadCard {
                space: sid(1),
                card: sid(card)
            }
        )
    )
    .unwrap()
    .result
}

async fn versions(storage: &Storage, card: u8) -> Vec<CardTextVersion> {
    storage
        .transact(move |ctx| Ok(ctx.card_history(&sid(1), &sid(card))?.versions))
        .await
        .unwrap()
}

#[tokio::test]
async fn lifecycle_history_replay_groups_and_restart_are_one_atomic_result() {
    let (dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    assert_eq!(first.titles, ["Ёж 空間 😀"]);
    assert_eq!(first.positions, [0]);
    assert_eq!(first.completed, [false]);
    assert!(first.tags.is_empty());
    let initial_versions = versions(&storage, 9).await;
    assert_eq!(initial_versions.len(), 1);
    assert_eq!(initial_versions[0].subject, admin.subject());
    let second = run!(storage, admin, create, creation(2, 8)).unwrap().result;
    assert_eq!(second.positions, [1]);
    let changed = run!(
        storage,
        admin,
        edit,
        edit_req(3, 9, "New", &first.descriptions[0], first.version)
    )
    .unwrap()
    .result;
    let back = run!(
        storage,
        admin,
        edit,
        edit_req(
            4,
            9,
            &first.titles[0],
            &first.descriptions[0],
            changed.version
        )
    )
    .unwrap()
    .result;
    assert_ne!(back.version, first.version);
    let text_versions = versions(&storage, 9).await;
    assert_eq!(text_versions.len(), 3);
    for (v, title) in text_versions
        .iter()
        .zip(["Ёж 空間 😀", "New", "Ёж 空間 😀"])
    {
        assert_eq!(v.title, title);
        assert_eq!(v.description, first.descriptions[0]);
    }
    rejected(
        run!(
            storage,
            admin,
            trash,
            req(
                5,
                TrashCard {
                    space: sid(1),
                    card: sid(9),
                    expected: first.version
                }
            )
        ),
        ErrorCode::StaleState,
    );
    let deleted = run!(
        storage,
        admin,
        trash,
        req(
            6,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: back.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(deleted.state, SpaceState::Trashed);
    assert_eq!(deleted.deletion_groups.len(), 1);
    assert_eq!(versions(&storage, 9).await, text_versions);
    let list = run!(
        storage,
        admin,
        list,
        req(
            7,
            ListCards {
                space: sid(1),
                include_trashed: false
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(list.cards, [second]);
    let restored = run!(
        storage,
        admin,
        restore,
        req(
            8,
            RestoreCard {
                space: sid(1),
                card: sid(9),
                expected: deleted.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(restored.state, SpaceState::Active);
    assert_eq!(restored.positions, first.positions);
    assert_eq!(versions(&storage, 9).await, text_versions);
    let repeated = run!(
        storage,
        admin,
        trash,
        req(
            6,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: back.version
            }
        )
    )
    .unwrap();
    assert!(repeated.replayed);
    assert_eq!(repeated.result, deleted);
    let deleted_again = run!(
        storage,
        admin,
        trash,
        req(
            9,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: restored.version
            }
        )
    )
    .unwrap()
    .result;
    assert_ne!(deleted.deletion_groups, deleted_again.deletion_groups);
    assert!(
        run!(
            storage,
            admin,
            restore,
            req(
                8,
                RestoreCard {
                    space: sid(1),
                    card: sid(9),
                    expected: deleted.version
                }
            )
        )
        .unwrap()
        .replayed
    );
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert!(
        run!(storage, admin, create, creation(1, 9))
            .unwrap()
            .replayed
    );
    assert_eq!(fresh(&storage, &admin, 10, 9).await, deleted_again);
    let mismatch = edit_req(3, 9, "different", "", first.version);
    rejected(
        run!(storage, admin, edit, mismatch),
        ErrorCode::CommandMismatch,
    );
}

#[tokio::test]
async fn empty_title_noop_and_stale_edit_do_not_write_text_or_history() {
    let (_dir, storage, admin) = setup().await;
    for (i, title) in ["", " \t\r\n", "\u{2003}"].iter().enumerate() {
        let mut request = creation(i as u8 + 1, 9);
        request.parameters.title = (*title).into();
        rejected(
            run!(storage, admin, create, request),
            ErrorCode::InvalidRequest,
        );
    }
    let first = run!(storage, admin, create, creation(4, 9)).unwrap().result;
    let noop = run!(
        storage,
        admin,
        edit,
        edit_req(
            5,
            9,
            &first.titles[0],
            &first.descriptions[0],
            first.version
        )
    )
    .unwrap()
    .result;
    assert_eq!(noop, first);
    rejected(
        run!(
            storage,
            admin,
            edit,
            edit_req(6, 9, "", "lost", first.version)
        ),
        ErrorCode::InvalidRequest,
    );
    let changed = run!(
        storage,
        admin,
        edit,
        edit_req(7, 9, "Title only", &first.descriptions[0], first.version)
    )
    .unwrap()
    .result;
    rejected(
        run!(
            storage,
            admin,
            edit,
            edit_req(
                8,
                9,
                &first.titles[0],
                "old description edit",
                first.version
            )
        ),
        ErrorCode::StaleState,
    );
    assert_eq!(fresh(&storage, &admin, 9, 9).await, changed);
    let changed2 = run!(
        storage,
        admin,
        edit,
        edit_req(10, 9, "Title only", "description only", changed.version)
    )
    .unwrap()
    .result;
    rejected(
        run!(
            storage,
            admin,
            edit,
            edit_req(
                11,
                9,
                "old title edit",
                &changed.descriptions[0],
                changed.version
            )
        ),
        ErrorCode::StaleState,
    );
    assert_eq!(fresh(&storage, &admin, 12, 9).await, changed2);
}

#[tokio::test]
async fn card_data_and_removed_membership_change_only_relevant_column_and_card_tokens() {
    let (_dir, storage, admin) = setup().await;
    let (empty, other) = storage
        .transact(|ctx| {
            Ok((
                ctx.column_snapshot(&sid(1), &sid(2))?,
                ctx.column_snapshot(&sid(1), &sid(3))?,
            ))
        })
        .await
        .unwrap();
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let occupied = storage
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    assert!(!occupied.empty);
    assert_ne!(empty.version, occupied.version);
    assert_eq!(
        storage
            .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(3))?))
            .await
            .unwrap(),
        other
    );
    let a = admin.clone();
    rejected(
        storage
            .transact(move |ctx| {
                Ok(zedone_core::columns::trash(
                    ctx,
                    &a.credential(),
                    &Now,
                    &req(
                        2,
                        TrashColumn {
                            space: sid(1),
                            column: sid(2),
                            expected: empty.version,
                        },
                    ),
                )?)
            })
            .await,
        ErrorCode::StaleState,
    );
    let a = admin.clone();
    rejected(
        storage
            .transact(move |ctx| {
                Ok(zedone_core::columns::trash(
                    ctx,
                    &a.credential(),
                    &Now,
                    &req(
                        3,
                        TrashColumn {
                            space: sid(1),
                            column: sid(2),
                            expected: occupied.version,
                        },
                    ),
                )?)
            })
            .await,
        ErrorCode::InvalidRequest,
    );
    run!(storage, admin, create, creation(4, 8)).unwrap();
    assert_eq!(fresh(&storage, &admin, 5, 9).await, first);
    for (i, field) in [
        "title",
        "description",
        "completed",
        "position",
        "column",
        "tags",
        "lifecycle",
    ]
    .iter()
    .enumerate()
    {
        let before = fresh(&storage, &admin, 20 + i as u8, 9).await;
        let field = field.to_string();
        storage
            .transact(move |ctx| {
                let object = object(ctx.read().document(&sid(1))?, &sid(9))?;
                ctx.edit(&sid(1), |doc| {
                    match field.as_str() {
                        "completed" => doc.put(&object, &field, true)?,
                        "position" => doc.put(&object, &field, 7_u64)?,
                        "column" => doc.put(&object, &field, sid(3))?,
                        "tags" => {
                            let tags = doc.get(&object, "tags")?.unwrap().1;
                            doc.put(&tags, sid(5), true)?;
                        }
                        "lifecycle" => {
                            doc.put(&object, &field, format!("trash:{}:{}", sid(5), sid(6)))?
                        }
                        _ => doc.put(&object, &field, "changed")?,
                    };
                    Ok(())
                })?;
                Ok(())
            })
            .await
            .unwrap();
        rejected(
            run!(
                storage,
                admin,
                edit,
                edit_req(40 + i as u8, 9, "stale", "stale", before.version)
            ),
            ErrorCode::StaleState,
        );
    }
    let before = fresh(&storage, &admin, 60, 9).await;
    storage
        .transact(|ctx| {
            let obj = object(ctx.read().document(&sid(1))?, &sid(9))?;
            let history = map(ctx.read().document(&sid(1))?, &obj, "history")?;
            ctx.edit(&sid(1), |doc| {
                append_version(
                    doc,
                    &history,
                    &format!("{}:{}", sid(5), sid(8)),
                    99,
                    "bookkeeping",
                    "",
                )
            })?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(fresh(&storage, &admin, 61, 9).await.version, before.version);
    let moved = storage
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    assert_ne!(moved.version, occupied.version);
}

#[tokio::test]
async fn faults_roll_back_card_history_and_receipt_and_unknown_completion_replays() {
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
                    move |ctx| Ok(core::create(ctx, &a.credential(), &Now, &creation(id, 9))?),
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
                    ReadCard {
                        space: sid(1),
                        card: sid(9)
                    }
                )
            ),
            ErrorCode::NotFound,
        );
    }
    let first = run!(storage, admin, create, creation(4, 9)).unwrap().result;
    storage.transact(|ctx| {ctx.connection().execute_batch("CREATE TRIGGER fail_card_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;Ok(())}).await.unwrap();
    rejected(
        run!(
            storage,
            admin,
            edit,
            edit_req(5, 9, "SQL fail", "", first.version)
        ),
        ErrorCode::Unavailable,
    );
    storage
        .transact(|ctx| {
            ctx.connection()
                .execute_batch("DROP TRIGGER fail_card_receipt")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(fresh(&storage, &admin, 30, 9).await, first);
    let a = admin.clone();
    assert_eq!(
        storage
            .transact_checked(
                move |ctx| Ok(core::edit(
                    ctx,
                    &a.credential(),
                    &Now,
                    &edit_req(5, 9, "SQL fail", "", first.version)
                )?),
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
    let retry = run!(
        storage,
        admin,
        edit,
        edit_req(5, 9, "SQL fail", "", first.version)
    )
    .unwrap();
    assert!(retry.replayed);
    assert_eq!(versions(&storage, 9).await.len(), 2);
}

#[tokio::test]
async fn two_workers_serialize_creation_and_text_edits() {
    let (dir, storage, admin) = setup().await;
    let other = Storage::open(dir.path().join("cards.db")).await.unwrap();
    let a = admin.clone();
    let b = admin.clone();
    let (one, two) = tokio::join!(
        storage.transact(move |ctx| Ok(core::create(ctx, &a.credential(), &Now, &creation(1, 9))?)),
        other.transact(move |ctx| Ok(core::create(ctx, &b.credential(), &Now, &creation(1, 9))?))
    );
    let (one, two) = (one.unwrap(), two.unwrap());
    assert_ne!(one.replayed, two.replayed);
    assert_eq!(one.result, two.result);
    let expected = one.result.version;
    let a = admin.clone();
    let b = admin.clone();
    let (one, two) = tokio::join!(
        storage.transact(move |ctx| Ok(core::edit(
            ctx,
            &a.credential(),
            &Now,
            &edit_req(2, 9, "one", "", expected)
        )?)),
        other.transact(move |ctx| Ok(core::edit(
            ctx,
            &b.credential(),
            &Now,
            &edit_req(3, 9, "two", "", expected)
        )?))
    );
    assert_ne!(one.is_ok(), two.is_ok());
    rejected(if one.is_err() { one } else { two }, ErrorCode::StaleState);
    assert_eq!(versions(&storage, 9).await.len(), 2);
}

#[tokio::test]
async fn rights_expiry_revocation_and_parent_states_apply_even_to_receipts() {
    let (_dir, storage, admin) = setup().await;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let expired = Arc::new(PendingAccess::generate().unwrap());
    let (a, m, e) = (admin.clone(), member.clone(), expired.clone());
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &Now,
                &m,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
                None,
            )?;
            ctx.issue_access(
                &a.credential(),
                &Now,
                &e,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::ReadWrite)])),
                Some(UNIX_EPOCH + Duration::from_millis(101)),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    run!(
        storage,
        member,
        read,
        req(
            2,
            ReadCard {
                space: sid(1),
                card: sid(9)
            }
        )
    )
    .unwrap();
    rejected(
        run!(
            storage,
            member,
            edit,
            edit_req(3, 9, "denied", "", first.version)
        ),
        ErrorCode::AccessDenied,
    );
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            ctx.revoke_access(&a.credential(), &Now, m.subject())?;
            Ok(())
        })
        .await
        .unwrap();
    rejected(
        run!(
            storage,
            member,
            read,
            req(
                2,
                ReadCard {
                    space: sid(1),
                    card: sid(9)
                }
            )
        ),
        ErrorCode::AccessDenied,
    );
    let e = expired.clone();
    rejected(
        storage
            .timed_transact(move |ctx, clock| {
                Ok(core::create(ctx, &e.credential(), clock, &creation(4, 8))?)
            })
            .await,
        ErrorCode::AccessDenied,
    );
    storage
        .transact(|ctx| {
            ctx.set_column_lifecycle(&sid(1), &sid(2), &format!("trash:{}:{}", sid(8), sid(8)))?;
            Ok(())
        })
        .await
        .unwrap();
    rejected(
        run!(
            storage,
            admin,
            edit,
            edit_req(5, 9, "parent trashed", "", first.version)
        ),
        ErrorCode::InvalidRequest,
    );
    assert!(
        run!(storage, admin, create, creation(1, 9))
            .unwrap()
            .replayed
    );
    assert_eq!(fresh(&storage, &admin, 6, 9).await, first);
}

#[tokio::test]
async fn conflicts_preserve_variants_and_block_every_ordinary_mutation() {
    for field in [
        "title",
        "description",
        "column",
        "position",
        "completed",
        "lifecycle",
    ] {
        let (_dir, storage, admin) = setup().await;
        let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
        storage
            .transact(move |ctx| {
                let graph = ctx.read().document(&sid(1))?;
                let object = object(graph, &sid(9))?;
                let mut a = graph.fork().with_actor(ActorId::from(vec![21]));
                let mut b = graph.fork().with_actor(ActorId::from(vec![22]));
                for (graph, n) in [(&mut a, 2_u64), (&mut b, 3_u64)] {
                    // Automerge elides assigning the unchanged base value. Make
                    // that branch a real independent change with a causal return.
                    if (field == "completed" && n == 3) || (field == "column" && n == 2) {
                        let mut tx = graph.transaction();
                        if field == "completed" {
                            tx.put(&object, field, true)?;
                        } else {
                            tx.put(&object, field, sid(3))?;
                        }
                        tx.commit();
                    }
                    let mut tx = graph.transaction();
                    match field {
                        "position" => tx.put(&object, field, n)?,
                        "completed" => tx.put(&object, field, n == 2)?,
                        "column" => tx.put(&object, field, sid(n as u8))?,
                        "lifecycle" => {
                            tx.put(&object, field, format!("trash:{}:{n:032x}", sid(5)))?
                        }
                        _ => tx.put(&object, field, format!("variant {n}"))?,
                    }
                    tx.commit();
                }
                let mut changes = a.get_changes(&[]);
                changes.extend(b.get_changes(&[]));
                ctx.apply_changes(&sid(1), changes)
            })
            .await
            .unwrap();
        let conflicted = fresh(&storage, &admin, 2, 9).await;
        assert_eq!(conflicted.state, SpaceState::Conflicted);
        match field {
            "title" => assert_eq!(conflicted.titles.len(), 2),
            "description" => assert_eq!(conflicted.descriptions.len(), 2),
            "column" => assert_eq!(conflicted.columns, [sid(2), sid(3)]),
            "completed" => assert_eq!(conflicted.completed, [false, true]),
            "position" => assert_eq!(conflicted.positions, [2, 3]),
            _ => assert_eq!(conflicted.deletion_groups.len(), 2),
        }
        rejected(
            run!(
                storage,
                admin,
                edit,
                edit_req(3, 9, "overwrite", "", first.version)
            ),
            ErrorCode::StaleState,
        );
        rejected(
            run!(
                storage,
                admin,
                edit,
                edit_req(4, 9, "overwrite", "", conflicted.version)
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
                    TrashCard {
                        space: sid(1),
                        card: sid(9),
                        expected: conflicted.version
                    }
                )
            ),
            ErrorCode::Conflict,
        );
        rejected(
            run!(
                storage,
                admin,
                restore,
                req(
                    6,
                    RestoreCard {
                        space: sid(1),
                        card: sid(9),
                        expected: conflicted.version
                    }
                )
            ),
            ErrorCode::Conflict,
        );
        assert_eq!(fresh(&storage, &admin, 7, 9).await, conflicted);
    }
}

#[tokio::test]
async fn membership_aba_and_deleted_cards_never_revive_an_old_column_token() {
    let (_dir, storage, admin) = setup().await;
    let empty = storage
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let occupied = storage
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    storage
        .transact(|ctx| {
            let object = object(ctx.read().document(&sid(1))?, &sid(9))?;
            ctx.edit(&sid(1), |doc| {
                doc.put(&object, "column", sid(3))?;
                Ok(())
            })?;
            Ok(())
        })
        .await
        .unwrap();
    let vacated = storage
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    assert!(vacated.empty);
    assert_ne!(vacated.version, empty.version);
    storage
        .transact(|ctx| {
            let object = object(ctx.read().document(&sid(1))?, &sid(9))?;
            ctx.edit(&sid(1), |doc| {
                doc.put(&object, "column", sid(2))?;
                Ok(())
            })?;
            Ok(())
        })
        .await
        .unwrap();
    let returned = storage
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    assert!(!returned.empty);
    assert_ne!(returned.version, occupied.version);
    rejected(
        run!(
            storage,
            admin,
            edit,
            edit_req(2, 9, "old", "", first.version)
        ),
        ErrorCode::StaleState,
    );
    storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |doc| {
                doc.delete(ROOT, key(&sid(9)))?;
                Ok(())
            })?;
            Ok(())
        })
        .await
        .unwrap();
    let deleted = storage
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    assert!(deleted.empty);
    assert_ne!(deleted.version, empty.version);
    assert_ne!(deleted.version, occupied.version);
}

#[tokio::test]
async fn separate_subjects_and_reverting_text_keep_every_paired_version() {
    let (_dir, storage, admin) = setup().await;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &Now,
                &m,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::ReadWrite)])),
                None,
            )
        })
        .await
        .unwrap();
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let by_agent = run!(
        storage,
        member,
        edit,
        edit_req(1, 9, "Agent", "new description", first.version)
    )
    .unwrap()
    .result;
    let back = run!(
        storage,
        admin,
        edit,
        edit_req(
            2,
            9,
            &first.titles[0],
            &first.descriptions[0],
            by_agent.version
        )
    )
    .unwrap()
    .result;
    assert_eq!(back.titles, first.titles);
    let text_versions = versions(&storage, 9).await;
    assert_eq!(text_versions.len(), 3);
    assert_eq!(
        text_versions
            .iter()
            .map(|v| v.subject.as_str())
            .collect::<Vec<_>>(),
        [admin.subject(), member.subject(), admin.subject()]
    );
    assert_eq!(text_versions[2].title, text_versions[0].title);
    assert_eq!(text_versions[2].description, text_versions[0].description);
    assert_ne!(text_versions[2].id, text_versions[0].id);
}

#[tokio::test]
async fn history_pages_have_causal_versions_and_do_not_inflate_mutation_receipts() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let second = run!(
        storage,
        admin,
        edit,
        edit_req(2, 9, "Two", "", first.version)
    )
    .unwrap()
    .result;
    let third = run!(
        storage,
        admin,
        edit,
        edit_req(3, 9, "Three", "", second.version)
    )
    .unwrap()
    .result;
    assert!(
        serde_json::to_value(&third)
            .unwrap()
            .get("text_versions")
            .is_none()
    );
    let page = run!(
        storage,
        admin,
        history,
        req(
            10,
            ReadCardHistory {
                space: sid(1),
                card: sid(9),
                after: None,
                limit: 1,
                expected: None
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(page.versions.len(), 1);
    assert_eq!(page.versions[0].title, first.titles[0]);
    assert!(page.next.is_some());
    let after = page.next.clone();
    let expected = page.version;
    let next = run!(
        storage,
        admin,
        history,
        req(
            11,
            ReadCardHistory {
                space: sid(1),
                card: sid(9),
                after: after.clone(),
                limit: 1,
                expected: Some(expected)
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(next.versions[0].title, "Two");
    let last = run!(
        storage,
        admin,
        history,
        req(
            12,
            ReadCardHistory {
                space: sid(1),
                card: sid(9),
                after: next.next,
                limit: 1,
                expected: Some(expected)
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(last.versions[0].title, "Three");
    assert_eq!(last.next, None);
    run!(storage, admin, create, creation(4, 8)).unwrap();
    let unchanged = run!(
        storage,
        admin,
        history,
        req(
            13,
            ReadCardHistory {
                space: sid(1),
                card: sid(9),
                after: None,
                limit: 100,
                expected: Some(expected)
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(unchanged.versions.len(), 3);
    run!(
        storage,
        admin,
        edit,
        edit_req(5, 9, "Four", "", third.version)
    )
    .unwrap();
    rejected(
        run!(
            storage,
            admin,
            history,
            req(
                14,
                ReadCardHistory {
                    space: sid(1),
                    card: sid(9),
                    after: after.clone(),
                    limit: 1,
                    expected: Some(expected)
                }
            )
        ),
        ErrorCode::StaleState,
    );
    assert!(
        run!(
            storage,
            admin,
            history,
            req(
                11,
                ReadCardHistory {
                    space: sid(1),
                    card: sid(9),
                    after,
                    limit: 1,
                    expected: Some(expected)
                }
            )
        )
        .unwrap()
        .replayed
    );
    for (id, limit) in [(20, 0), (21, 101)] {
        rejected(
            run!(
                storage,
                admin,
                history,
                req(
                    id,
                    ReadCardHistory {
                        space: sid(1),
                        card: sid(9),
                        after: None,
                        limit,
                        expected: None
                    }
                )
            ),
            ErrorCode::InvalidRequest,
        );
    }
    rejected(
        run!(
            storage,
            admin,
            history,
            req(
                22,
                ReadCardHistory {
                    space: sid(1),
                    card: sid(9),
                    after: Some("missing".into()),
                    limit: 1,
                    expected: None
                }
            )
        ),
        ErrorCode::InvalidRequest,
    );
}
