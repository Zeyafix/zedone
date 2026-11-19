use super::{
    text_conflicts::{copy, exchange},
    *,
};
use zedone_core::{
    cards::ResolveCardDeletion,
    cleanup::{self, InspectTrash, PurgeTrash, TrashSelection},
    ordering::{self, CardPlacement, MoveCard, OrderStore},
    placements::{self, ResolveCardPlacement},
    replay::StateVersion,
    tag_assignments::{self, ResolveCardTag},
    tags::{self, CreateTag, SetCardTag, TagStore},
};

fn trash_card(id: u8, expected: StateVersion) -> CommandRequest<TrashCard> {
    req(
        id,
        TrashCard {
            space: sid(1),
            card: sid(9),
            expected,
        },
    )
}

fn decision(id: u8, expected: StateVersion, keep: bool) -> CommandRequest<ResolveCardDeletion> {
    req(
        id,
        ResolveCardDeletion {
            space: sid(1),
            card: sid(9),
            expected,
            keep,
        },
    )
}

#[tokio::test]
async fn mixed_conflicts_resolve_in_either_order_without_changing_the_deletion_group() {
    for fields_first in [false, true] {
        let (dir, left, admin) = setup().await;
        run!(&left, admin, create, creation(1, 9)).unwrap();
        left.transact(|tx| {
            tx.create_tag(&CreateTag {
                space: sid(1),
                tag: sid(12),
                name: "Mixed".into(),
            })?;
            Ok(())
        })
        .await
        .unwrap();
        let right = copy(&left, dir.path().join("right.db"), &admin).await;
        for (db, column, present) in [(&left, 2, false), (&right, 3, true)] {
            db.transact(move |tx| {
                tx.place_card(
                    &sid(1),
                    &sid(9),
                    &CardPlacement {
                        column: sid(column),
                        rank: Legacy(10),
                    },
                )?;
                let card = tx.card_snapshot(&sid(1), &sid(9))?;
                let tag = tx.tag_snapshot(&sid(1), &sid(12))?;
                tx.set_card_tag(&SetCardTag {
                    space: card.space,
                    card: card.card,
                    tag: tag.tag,
                    present,
                    expected: card.version,
                    expected_tag: tag.version,
                })?;
                Ok(())
            })
            .await
            .unwrap();
        }
        let active = fresh(&left, &admin, 2, 9).await;
        let trash = run!(&left, admin, trash, trash_card(3, active.version))
            .unwrap()
            .result;
        exchange(&left, &right).await;
        let merged = fresh(&left, &admin, 4, 9).await;
        assert!(merged.deletion_conflict);
        assert_eq!(merged.placements.len(), 2);
        if !fields_first {
            let result = run!(
                &left,
                admin,
                resolve_deletion,
                decision(5, merged.version, false)
            )
            .unwrap()
            .result;
            assert!(!result.deletion_conflict);
            assert_eq!(result.state, SpaceState::Conflicted);
        }
        let owner = admin.clone();
        let chosen = left
            .transact(move |tx| {
                let card = tx.card_snapshot(&sid(1), &sid(9))?;
                let place = card
                    .placements
                    .iter()
                    .find(|place| place.column == sid(3))
                    .unwrap()
                    .clone();
                let result = placements::resolve(
                    tx,
                    &owner.credential(),
                    &Now,
                    &req(
                        6,
                        ResolveCardPlacement {
                            space: card.space,
                            card: card.card,
                            expected: card.version,
                            placement: place,
                        },
                    ),
                )?
                .result;
                let tag = tx.tag_snapshot(&sid(1), &sid(12))?;
                Ok(tag_assignments::resolve(
                    tx,
                    &owner.credential(),
                    &Now,
                    &req(
                        7,
                        ResolveCardTag {
                            space: result.space,
                            card: result.card,
                            expected: result.version,
                            tag: tag.tag,
                            expected_tag: tag.version,
                            present: fields_first,
                        },
                    ),
                )?
                .result)
            })
            .await
            .unwrap();
        assert_eq!(chosen.deletion_groups, trash.deletion_groups);
        assert_eq!(chosen.deletion_conflict, fields_first);
        if fields_first {
            run!(
                &left,
                admin,
                resolve_deletion,
                decision(8, chosen.version, false)
            )
            .unwrap();
        }
        exchange(&left, &right).await;
        let final_card = fresh(&right, &admin, 9, 9).await;
        assert_eq!(final_card.state, SpaceState::Trashed);
        assert!(!final_card.deletion_conflict);
        assert_eq!(final_card.deletion_groups, trash.deletion_groups);
        assert_eq!(final_card.columns, [sid(3)]);
        assert_eq!(final_card.tags.contains(&sid(12)), fields_first);
        assert_eq!(final_card, fresh(&left, &admin, 9, 9).await);
        let restored = run!(
            &right,
            admin,
            restore,
            req(
                10,
                RestoreCard {
                    space: sid(1),
                    card: sid(9),
                    expected: final_card.version,
                    column: None,
                    expected_column: None,
                }
            )
        )
        .unwrap()
        .result;
        assert_eq!(restored.state, SpaceState::Active);
        assert_eq!(restored.columns, final_card.columns);
        assert_eq!(restored.tags, final_card.tags);
    }
}

#[tokio::test]
async fn placement_resolution_cannot_move_a_member_out_of_a_container_deletion_group() {
    let (dir, left, admin) = setup().await;
    let base = run!(&left, admin, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    let group = format!("trash:{}:{}", admin.subject(), sid(2));
    left.transact(move |tx| {
        tx.place_card(
            &sid(1),
            &sid(9),
            &CardPlacement {
                column: sid(2),
                rank: Legacy(8),
            },
        )?;
        tx.set_card_lifecycle(&sid(1), &sid(9), &group)?;
        tx.set_column_lifecycle(&sid(1), &sid(2), &group)?;
        Ok(())
    })
    .await
    .unwrap();
    run!(
        &right,
        admin,
        edit,
        edit_req(2, 9, "Independent", "", base.version)
    )
    .unwrap();
    right
        .transact(|tx| {
            tx.place_card(
                &sid(1),
                &sid(9),
                &CardPlacement {
                    column: sid(3),
                    rank: Legacy(9),
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    exchange(&left, &right).await;
    let before = fresh(&left, &admin, 3, 9).await;
    assert!(!before.deletion_conflict);
    assert_eq!(before.placements.len(), 2);
    let owner = admin.clone();
    let unchanged = before.clone();
    rejected(
        left.transact(move |tx| {
            let placement = before
                .placements
                .into_iter()
                .find(|place| place.column == sid(3))
                .unwrap();
            Ok(placements::resolve(
                tx,
                &owner.credential(),
                &Now,
                &req(
                    4,
                    ResolveCardPlacement {
                        space: before.space,
                        card: before.card,
                        expected: before.version,
                        placement,
                    },
                ),
            )?)
        })
        .await,
        ErrorCode::Conflict,
    );
    assert_eq!(fresh(&left, &admin, 5, 9).await, unchanged);
}

#[tokio::test]
async fn independent_edit_and_trash_are_visible_as_a_conflict_on_both_replicas() {
    let (dir, left, admin) = setup().await;
    let base = run!(&left, admin, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    let trashed = run!(
        &left,
        admin,
        trash,
        req(
            2,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: base.version
            }
        )
    )
    .unwrap()
    .result;
    run!(
        &right,
        admin,
        edit,
        edit_req(3, 9, "Independent edit", "Preserve me", base.version)
    )
    .unwrap();
    exchange(&left, &right).await;
    let merged = fresh(&left, &admin, 4, 9).await;
    assert_eq!(merged, fresh(&right, &admin, 4, 9).await);
    assert_eq!(merged.state, SpaceState::Conflicted);
    assert!(merged.deletion_conflict);
    assert_eq!(merged.deletion_groups, trashed.deletion_groups);
    assert_eq!(merged.titles, ["Independent edit"]);
    assert_eq!(merged.descriptions, ["Preserve me"]);

    let sibling = run!(&left, admin, create, creation(7, 10)).unwrap().result;
    let sibling = run!(
        &left,
        admin,
        trash,
        req(
            8,
            TrashCard {
                space: sibling.space,
                card: sibling.card,
                expected: sibling.version,
            }
        )
    )
    .unwrap()
    .result;
    let owner = admin.clone();
    let preview = left
        .transact(move |tx| {
            Ok(cleanup::inspect_trash(
                tx,
                &owner.credential(),
                &Now,
                &req(
                    9,
                    InspectTrash {
                        space: sid(1),
                        selection: TrashSelection::All,
                    },
                ),
            )?
            .result)
        })
        .await
        .unwrap();
    assert_eq!(preview.conflicts, [format!("card:{}", sid(9))]);
    let owner = admin.clone();
    rejected(
        left.transact(move |tx| {
            Ok(cleanup::purge_trash(
                tx,
                &owner.credential(),
                &Now,
                &req(
                    10,
                    PurgeTrash {
                        space: sid(1),
                        selection: preview.selection,
                        expected: preview.version,
                    },
                ),
            )?)
        })
        .await,
        ErrorCode::Conflict,
    );
    assert_eq!(fresh(&left, &admin, 11, 10).await, sibling);
    assert_eq!(fresh(&left, &admin, 12, 9).await, merged);
}

#[tokio::test]
async fn both_lifecycle_choices_preserve_every_kind_of_independent_edit() {
    for (field, keep) in (0..5).flat_map(|field| [false, true].map(move |keep| (field, keep))) {
        let (dir, left, admin) = setup().await;
        let base = run!(&left, admin, create, creation(1, 9)).unwrap().result;
        let tag_owner = admin.clone();
        let tag = left
            .transact(move |tx| {
                Ok(tags::create(
                    tx,
                    &tag_owner.credential(),
                    &Now,
                    &req(
                        2,
                        CreateTag {
                            space: sid(1),
                            tag: sid(12),
                            name: "Independent tag".into(),
                        },
                    ),
                )?
                .result)
            })
            .await
            .unwrap();
        let right = copy(&left, dir.path().join("right.db"), &admin).await;
        let trashed = run!(&left, admin, trash, trash_card(3, base.version))
            .unwrap()
            .result;
        match field {
            0 | 1 => {
                let title = if field == 0 {
                    "Changed title"
                } else {
                    &base.titles[0]
                };
                let description = if field == 1 {
                    "Changed description"
                } else {
                    &base.descriptions[0]
                };
                run!(
                    &right,
                    admin,
                    edit,
                    edit_req(4, 9, title, description, base.version)
                )
                .unwrap();
            }
            2 => {
                run!(
                    &right,
                    admin,
                    set_completed,
                    completion::request(4, true, base.version)
                )
                .unwrap();
            }
            3 => {
                let owner = admin.clone();
                right
                    .transact(move |tx| {
                        ordering::move_card(
                            tx,
                            &owner.credential(),
                            &Now,
                            &req(
                                4,
                                MoveCard {
                                    space: sid(1),
                                    card: sid(9),
                                    column: sid(3),
                                    before: None,
                                    expected: base.version,
                                },
                            ),
                        )?;
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
            _ => {
                let owner = admin.clone();
                right
                    .transact(move |tx| {
                        tags::set_card_tag(
                            tx,
                            &owner.credential(),
                            &Now,
                            &req(
                                4,
                                SetCardTag {
                                    space: sid(1),
                                    card: sid(9),
                                    tag: sid(12),
                                    present: true,
                                    expected: base.version,
                                    expected_tag: tag.version,
                                },
                            ),
                        )?;
                        Ok(())
                    })
                    .await
                    .unwrap();
            }
        }
        let edited = fresh(&right, &admin, 5, 9).await;
        exchange(&left, &right).await;
        let conflict = fresh(&left, &admin, 5, 9).await;
        assert!(conflict.deletion_conflict, "{field}");
        rejected(
            run!(
                &left,
                admin,
                resolve_deletion,
                decision(6, trashed.version, keep)
            ),
            ErrorCode::StaleState,
        );
        rejected(
            run!(&left, admin, trash, trash_card(7, conflict.version)),
            ErrorCode::Conflict,
        );
        let resolved = run!(
            &left,
            admin,
            resolve_deletion,
            decision(8, conflict.version, keep)
        )
        .unwrap()
        .result;
        let mut expected = edited;
        expected.version = resolved.version;
        if !keep {
            expected.state = SpaceState::Trashed;
            expected.deletion_groups = trashed.deletion_groups;
        }
        assert_eq!(resolved, expected, "{field}, keep={keep}");
        exchange(&left, &right).await;
        assert_eq!(fresh(&right, &admin, 9, 9).await, resolved);
        left.shutdown().await.unwrap();
        let reopened = Storage::open(dir.path().join("cards.db")).await.unwrap();
        assert_eq!(fresh(&reopened, &admin, 9, 9).await, resolved);
        assert!(
            run!(
                &reopened,
                admin,
                resolve_deletion,
                decision(8, conflict.version, keep)
            )
            .unwrap()
            .replayed
        );
    }
}

#[tokio::test]
async fn a_late_unseen_edit_reopens_a_confirmed_trash_choice() {
    let (dir, left, admin) = setup().await;
    let base = run!(&left, admin, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    let late = copy(&left, dir.path().join("late.db"), &admin).await;
    run!(&left, admin, trash, trash_card(2, base.version)).unwrap();
    run!(
        &right,
        admin,
        edit,
        edit_req(3, 9, "First edit", &base.descriptions[0], base.version)
    )
    .unwrap();
    exchange(&left, &right).await;
    let conflict = fresh(&left, &admin, 4, 9).await;
    run!(
        &left,
        admin,
        resolve_deletion,
        decision(5, conflict.version, false)
    )
    .unwrap();
    run!(
        &late,
        admin,
        edit,
        edit_req(6, 9, &base.titles[0], "Late edit", base.version)
    )
    .unwrap();
    exchange(&left, &late).await;
    let reopened = fresh(&left, &admin, 7, 9).await;
    assert!(reopened.deletion_conflict);
    assert_eq!(reopened.titles, ["First edit"]);
    assert_eq!(reopened.descriptions, ["Late edit"]);
    assert_eq!(reopened, fresh(&late, &admin, 7, 9).await);
}

#[tokio::test]
async fn deletion_resolution_requires_write_access_and_rolls_back_with_its_receipt() {
    let (dir, left, admin) = setup().await;
    let base = run!(&left, admin, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(&left, admin, trash, trash_card(2, base.version)).unwrap();
    run!(
        &right,
        admin,
        edit,
        edit_req(3, 9, "Edited", "", base.version)
    )
    .unwrap();
    exchange(&left, &right).await;
    let conflict = fresh(&left, &admin, 4, 9).await;
    let reader = Arc::new(PendingAccess::generate().unwrap());
    let credential = admin.clone();
    let grant = reader.clone();
    left.transact(move |tx| {
        tx.issue_access(
            &credential.credential(),
            &Now,
            &grant,
            &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
            None,
        )
    })
    .await
    .unwrap();
    rejected(
        run!(
            &left,
            reader,
            resolve_deletion,
            decision(5, conflict.version, true)
        ),
        ErrorCode::AccessDenied,
    );
    let history = versions(&left, 9).await;
    for (index, stage) in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
    ]
    .into_iter()
    .enumerate()
    {
        let credential = admin.clone();
        let choice = decision(6, conflict.version, index % 2 == 0);
        let result = left
            .transact_checked(
                move |tx| {
                    Ok(core::resolve_deletion(
                        tx,
                        &credential.credential(),
                        &Now,
                        &choice,
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
        assert_eq!(result.unwrap_err(), StorageError::Unavailable);
        assert_eq!(fresh(&left, &admin, 7, 9).await, conflict);
        assert_eq!(versions(&left, 9).await, history);
    }
    let chosen = run!(
        &left,
        admin,
        resolve_deletion,
        decision(6, conflict.version, false)
    )
    .unwrap();
    assert!(!chosen.replayed);
    assert_eq!(chosen.result.state, SpaceState::Trashed);
    assert_eq!(versions(&left, 9).await, history);
}

#[tokio::test]
async fn equal_concurrent_choices_converge_and_different_choices_reopen_the_conflict() {
    for same in [false, true] {
        let (dir, left, admin) = setup().await;
        let base = run!(&left, admin, create, creation(1, 9)).unwrap().result;
        let right = copy(&left, dir.path().join("right.db"), &admin).await;
        run!(&left, admin, trash, trash_card(2, base.version)).unwrap();
        run!(
            &right,
            admin,
            edit,
            edit_req(3, 9, "Edited", "", base.version)
        )
        .unwrap();
        exchange(&left, &right).await;
        let conflict = fresh(&left, &admin, 4, 9).await;
        run!(
            &left,
            admin,
            resolve_deletion,
            decision(5, conflict.version, false)
        )
        .unwrap();
        run!(
            &right,
            admin,
            resolve_deletion,
            decision(6, conflict.version, !same)
        )
        .unwrap();
        exchange(&left, &right).await;
        let merged = fresh(&left, &admin, 7, 9).await;
        assert_eq!(merged, fresh(&right, &admin, 7, 9).await);
        assert_eq!(merged.deletion_conflict, !same);
        assert_eq!(
            merged.state,
            if same {
                SpaceState::Trashed
            } else {
                SpaceState::Conflicted
            }
        );
        assert_eq!(merged.titles, ["Edited"]);
    }
}
