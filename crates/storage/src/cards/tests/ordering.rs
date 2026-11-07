use super::*;
use std::collections::BTreeMap;
use zedone_core::{
    columns::ListColumns,
    ordering::{self as order, MoveCard, MoveColumn, Rank},
};

fn movement(
    id: u8,
    card: u8,
    column: u8,
    before: Option<u8>,
    expected: zedone_core::replay::StateVersion,
) -> CommandRequest<MoveCard> {
    req(
        id,
        MoveCard {
            space: sid(1),
            card: sid(card),
            column: sid(column),
            before: before.map(sid),
            expected,
        },
    )
}
macro_rules! move_card {
    ($s:expr,$a:expr,$r:expr) => {{
        let a = $a.clone();
        let r = $r;
        $s.transact(move |ctx| Ok(order::move_card(ctx, &a.credential(), &Now, &r)?))
            .await
    }};
}
async fn ordered(storage: &Storage, column: u8) -> Vec<String> {
    storage
        .transact(move |ctx| {
            let mut cards = ctx.cards(&sid(1))?;
            cards.retain(|c| c.columns == [sid(column)] && c.state == SpaceState::Active);
            cards.sort_by_key(|c| c.positions[0].key(&c.card));
            Ok(cards.into_iter().map(|c| c.card).collect())
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn moves_preserve_identity_data_neighbors_history_and_replay_after_undo_and_restart() {
    let (dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let second = run!(s, a, create, creation(2, 8)).unwrap().result;
    let third = run!(s, a, create, creation(3, 7)).unwrap().result;
    let history = versions(&s, 9).await;
    let r = movement(4, 9, 2, Some(7), first.version);
    let moved = move_card!(s, a, r).unwrap().result;
    assert_eq!(ordered(&s, 2).await, [sid(8), sid(9), sid(7)]);
    assert_eq!(fresh(&s, &a, 5, 8).await, second);
    assert_eq!(fresh(&s, &a, 6, 7).await, third);
    assert_eq!(versions(&s, 9).await, history);
    assert_eq!(moved.titles, first.titles);
    assert_eq!(moved.descriptions, first.descriptions);
    assert_eq!(moved.completed, first.completed);
    assert_eq!(moved.tags, first.tags);
    let noop = move_card!(s, a, movement(7, 9, 2, Some(7), moved.version))
        .unwrap()
        .result;
    assert_eq!(noop, moved);
    let there = move_card!(s, a, movement(8, 9, 3, None, moved.version))
        .unwrap()
        .result;
    assert_eq!(ordered(&s, 2).await, [sid(8), sid(7)]);
    assert_eq!(ordered(&s, 3).await, [sid(9)]);
    let back = move_card!(s, a, movement(9, 9, 2, Some(8), there.version))
        .unwrap()
        .result;
    assert_eq!(ordered(&s, 2).await, [sid(9), sid(8), sid(7)]);
    assert_ne!(back.version, first.version);
    rejected(
        move_card!(s, a, movement(10, 9, 2, None, first.version)),
        ErrorCode::StaleState,
    );
    // Old numeric receipt remains decodable, and replay cannot undo a later move.
    let original = run!(s, a, create, creation(1, 9)).unwrap();
    assert!(original.replayed);
    assert_eq!(original.result, first);
    s.shutdown().await.unwrap();
    let s = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert_eq!(fresh(&s, &a, 11, 9).await, back);
    let replay = move_card!(s, a, movement(4, 9, 2, Some(7), first.version)).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, moved);
    assert_eq!(fresh(&s, &a, 12, 9).await, back);
    rejected(
        move_card!(s, a, movement(4, 9, 3, None, first.version)),
        ErrorCode::CommandMismatch,
    );
}

#[tokio::test]
async fn moves_reject_stale_text_self_missing_foreign_and_trashed_targets_without_effects() {
    let (_dir, s, a) = setup().await;
    let card = run!(s, a, create, creation(1, 9)).unwrap().result;
    let edited = run!(
        s,
        a,
        edit,
        edit_req(2, 9, "New", "description", card.version)
    )
    .unwrap()
    .result;
    rejected(
        move_card!(s, a, movement(3, 9, 3, None, card.version)),
        ErrorCode::StaleState,
    );
    for (id, column, before, code) in [
        (4, 3, Some(9), ErrorCode::InvalidRequest),
        (5, 3, Some(7), ErrorCode::InvalidRequest),
        (6, 99, None, ErrorCode::NotFound),
    ] {
        rejected(
            move_card!(s, a, movement(id, 9, column, before, edited.version)),
            code,
        );
    }
    // A real column in another document is not an admissible destination.
    let b = a.clone();
    s.transact(move |ctx| {
        spaces::create(
            ctx,
            &b.credential(),
            &Now,
            &req(
                20,
                CreateSpace {
                    space: sid(11),
                    name: "Other".into(),
                },
            ),
        )?;
        zedone_core::columns::create(
            ctx,
            &b.credential(),
            &Now,
            &req(
                21,
                CreateColumn {
                    space: sid(11),
                    column: sid(99),
                    name: "Other".into(),
                },
            ),
        )?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        move_card!(s, a, movement(7, 9, 99, None, edited.version)),
        ErrorCode::NotFound,
    );
    s.transact(|ctx| {
        Ok(ctx.set_column_lifecycle(&sid(1), &sid(3), &format!("trash:{}:{}", sid(8), sid(8)))?)
    })
    .await
    .unwrap();
    rejected(
        move_card!(s, a, movement(8, 9, 3, None, edited.version)),
        ErrorCode::InvalidRequest,
    );
    assert_eq!(fresh(&s, &a, 9, 9).await, edited);
    let trashed = run!(
        s,
        a,
        trash,
        req(
            10,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: edited.version
            }
        )
    )
    .unwrap()
    .result;
    rejected(
        move_card!(s, a, movement(11, 9, 2, None, trashed.version)),
        ErrorCode::InvalidRequest,
    );
}

#[tokio::test]
async fn unrelated_card_edit_does_not_invalidate_move_and_ranked_append_is_after_everything() {
    let (_dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let second = run!(s, a, create, creation(2, 8)).unwrap().result;
    run!(s, a, edit, edit_req(3, 8, "Other", "", second.version)).unwrap();
    move_card!(s, a, movement(4, 9, 2, None, first.version)).unwrap();
    run!(s, a, create, creation(5, 7)).unwrap();
    assert_eq!(ordered(&s, 2).await, [sid(8), sid(9), sid(7)]);
    for i in 0..70_u8 {
        let c = fresh(&s, &a, 30 + i, 9).await;
        let before = if i % 2 == 0 { Some(8) } else { Some(7) };
        move_card!(s, a, movement(110 + i, 9, 2, before, c.version)).unwrap();
    }
    assert_eq!(ordered(&s, 2).await, [sid(8), sid(9), sid(7)]);
}

#[tokio::test]
async fn every_failure_rolls_back_whole_place_membership_and_receipt_and_unknown_commit_replays() {
    let (_dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let before = s
        .transact(|ctx| {
            Ok((
                ctx.column_snapshot(&sid(1), &sid(2))?,
                ctx.column_snapshot(&sid(1), &sid(3))?,
            ))
        })
        .await
        .unwrap();
    for checkpoint in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
    ] {
        let b = a.clone();
        assert_eq!(
            s.transact_checked(
                move |ctx| Ok(order::move_card(
                    ctx,
                    &b.credential(),
                    &Now,
                    &movement(2, 9, 3, None, first.version)
                )?),
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
        assert_eq!(fresh(&s, &a, 3, 9).await, first);
        assert_eq!(
            s.transact(|ctx| Ok((
                ctx.column_snapshot(&sid(1), &sid(2))?,
                ctx.column_snapshot(&sid(1), &sid(3))?
            )))
            .await
            .unwrap(),
            before
        );
    }
    s.transact(|ctx| {ctx.connection().execute_batch("CREATE TRIGGER fail_move_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;Ok(())}).await.unwrap();
    rejected(
        move_card!(s, a, movement(2, 9, 3, None, first.version)),
        ErrorCode::Unavailable,
    );
    s.transact(|ctx| {
        ctx.connection()
            .execute_batch("DROP TRIGGER fail_move_receipt")?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(fresh(&s, &a, 4, 9).await, first);
    let b = a.clone();
    assert_eq!(
        s.transact_checked(
            move |ctx| Ok(order::move_card(
                ctx,
                &b.credential(),
                &Now,
                &movement(2, 9, 3, None, first.version)
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
    let replay = move_card!(s, a, movement(2, 9, 3, None, first.version)).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result.columns, [sid(3)]);
    let after = s
        .transact(|ctx| {
            Ok((
                ctx.column_snapshot(&sid(1), &sid(2))?,
                ctx.column_snapshot(&sid(1), &sid(3))?,
            ))
        })
        .await
        .unwrap();
    assert!(after.0.empty);
    assert!(!after.1.empty);
    assert_ne!(before.0.version, after.0.version);
    assert_ne!(before.1.version, after.1.version);
}

#[tokio::test]
async fn concurrent_move_and_edit_require_a_new_intention_for_the_loser() {
    let (dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let other = Storage::open(dir.path().join("cards.db")).await.unwrap();
    let (b, c) = (a.clone(), a.clone());
    let (one, two) = tokio::join!(
        s.transact(move |ctx| Ok(order::move_card(
            ctx,
            &b.credential(),
            &Now,
            &movement(2, 9, 3, None, first.version)
        )?)),
        other.transact(move |ctx| Ok(core::edit(
            ctx,
            &c.credential(),
            &Now,
            &edit_req(3, 9, "Concurrent", "", first.version)
        )?))
    );
    assert_ne!(one.is_ok(), two.is_ok());
    rejected(if one.is_ok() { two } else { one }, ErrorCode::StaleState);
}

#[tokio::test]
async fn independent_whole_moves_keep_paired_variants_and_block_normal_edits() {
    let (_dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    s.transact(|ctx| {
        let graph = ctx.read().document(&sid(1))?;
        let object = object(graph, &sid(9))?;
        let mut changes = Vec::new();
        for (actor, column) in [(41, 2), (42, 3)] {
            let mut branch = graph.clone().with_actor(ActorId::from(vec![actor]));
            let place = order::CardPlacement {
                column: sid(column),
                rank: Rank::Legacy(u64::from(actor)),
            };
            let mut tx = branch.transaction();
            tx.put(&object, "placement", serde_json::to_string(&place).unwrap())?;
            tx.commit();
            changes.push(branch.get_last_local_change().unwrap());
        }
        ctx.apply_changes(&sid(1), changes)?;
        Ok(())
    })
    .await
    .unwrap();
    let conflicted = fresh(&s, &a, 2, 9).await;
    assert_eq!(conflicted.state, SpaceState::Conflicted);
    assert_eq!(conflicted.placements.len(), 2);
    assert_eq!(conflicted.placements[0].column, sid(2));
    assert_eq!(conflicted.placements[0].rank, Rank::Legacy(41));
    assert_eq!(conflicted.placements[1].column, sid(3));
    assert_eq!(conflicted.placements[1].rank, Rank::Legacy(42));
    rejected(
        move_card!(s, a, movement(3, 9, 2, None, conflicted.version)),
        ErrorCode::Conflict,
    );
    rejected(
        run!(
            s,
            a,
            edit,
            edit_req(4, 9, "Blocked", "", conflicted.version)
        ),
        ErrorCode::Conflict,
    );
}

#[tokio::test]
async fn columns_reorder_with_contents_and_return_without_changing_card_versions() {
    let (_dir, s, a) = setup().await;
    let card = run!(s, a, create, creation(1, 9)).unwrap().result;
    let before = s
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    let b = a.clone();
    let moved = s
        .transact(move |ctx| {
            Ok(order::move_column(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    2,
                    MoveColumn {
                        space: sid(1),
                        column: sid(2),
                        before: None,
                        expected: before.version,
                    },
                ),
            )?)
        })
        .await
        .unwrap()
        .result;
    assert_eq!(fresh(&s, &a, 3, 9).await, card);
    let b = a.clone();
    let list = s
        .transact(move |ctx| {
            Ok(zedone_core::columns::list(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    4,
                    ListColumns {
                        space: sid(1),
                        include_trashed: false,
                    },
                ),
            )?)
        })
        .await
        .unwrap()
        .result;
    assert_eq!(
        list.columns
            .iter()
            .map(|c| c.column.clone())
            .collect::<Vec<_>>(),
        [sid(3), sid(2)]
    );
    let b = a.clone();
    s.transact(move |ctx| {
        Ok(order::move_column(
            ctx,
            &b.credential(),
            &Now,
            &req(
                5,
                MoveColumn {
                    space: sid(1),
                    column: sid(2),
                    before: Some(sid(3)),
                    expected: moved.version,
                },
            ),
        )?)
    })
    .await
    .unwrap();
    assert_eq!(fresh(&s, &a, 6, 9).await, card);
}

#[tokio::test]
async fn expired_and_revoked_writers_cannot_move_or_replay_even_noops() {
    let (_dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let expired = Arc::new(PendingAccess::generate().unwrap());
    let writer = Arc::new(PendingAccess::generate().unwrap());
    let (b, e, w) = (a.clone(), expired.clone(), writer.clone());
    s.transact(move |ctx| {
        for (key, expiry) in [
            (&e, Some(UNIX_EPOCH + Duration::from_millis(101))),
            (&w, None),
        ] {
            ctx.issue_access(
                &b.credential(),
                &Now,
                key,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::ReadWrite)])),
                expiry,
            )?;
        }
        Ok(())
    })
    .await
    .unwrap();
    let e = expired.clone();
    rejected(
        s.timed_transact(move |ctx, clock| {
            Ok(order::move_card(
                ctx,
                &e.credential(),
                clock,
                &movement(2, 9, 3, None, first.version),
            )?)
        })
        .await,
        ErrorCode::AccessDenied,
    );
    move_card!(s, writer, movement(2, 9, 3, None, first.version)).unwrap();
    let (b, w) = (a.clone(), writer.clone());
    s.transact(move |ctx| {
        ctx.revoke_access(&b.credential(), &Now, w.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        move_card!(s, writer, movement(2, 9, 3, None, first.version)),
        ErrorCode::AccessDenied,
    );
}

#[tokio::test]
async fn v4_upgrade_preserves_graph_tokens_and_receipts_and_rejects_old_writers() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("cards.db");
    let mut startup = crate::startup::Startup::new(&path);
    startup.registry = crate::schema::Registry(&crate::schema::REGISTRY.0[..4]);
    let s = startup.automatic(None).await.unwrap();
    let a = Arc::new(PendingAccess::generate().unwrap());
    let b = a.clone();
    s.transact(move |ctx| {
        ctx.bootstrap_access(&b)?;
        spaces::create(
            ctx,
            &b.credential(),
            &Now,
            &req(
                250,
                CreateSpace {
                    space: sid(1),
                    name: "Legacy".into(),
                },
            ),
        )?;
        for column in [2, 3] {
            zedone_core::columns::create(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    250 + column,
                    CreateColumn {
                        space: sid(1),
                        column: sid(column),
                        name: "Legacy".into(),
                    },
                ),
            )?;
        }
        Ok(())
    })
    .await
    .unwrap();
    let legacy = run!(s, a, create, creation(1, 9)).unwrap().result;
    let bytes = s
        .transact(|ctx| {
            Ok(ctx
                .read()
                .document(&sid(1))?
                .get_changes(&[])
                .iter()
                .map(|c| c.raw_bytes().to_vec())
                .collect::<Vec<_>>())
        })
        .await
        .unwrap();
    s.shutdown().await.unwrap();
    assert_eq!(
        Storage::open(&path).await.err(),
        Some(StorageError::Unavailable)
    );
    let options = crate::startup::MigrationOptions {
        folder: dir.path().to_owned(),
        created_at_unix_ms: 10,
        limits: crate::migration_backup::tests::LIMITS,
    };
    let s = crate::startup::Startup::new(&path)
        .automatic(Some(options))
        .await
        .unwrap();
    assert_eq!(fresh(&s, &a, 2, 9).await, legacy);
    assert_eq!(
        s.transact(|ctx| Ok(ctx
            .read()
            .document(&sid(1))?
            .get_changes(&[])
            .iter()
            .map(|c| c.raw_bytes().to_vec())
            .collect::<Vec<_>>()))
            .await
            .unwrap(),
        bytes
    );
    let replay = run!(s, a, create, creation(1, 9)).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, legacy);
    move_card!(s, a, movement(3, 9, 3, None, legacy.version)).unwrap();
    s.shutdown().await.unwrap();
    let mut old = crate::startup::Startup::new(&path);
    old.registry = crate::schema::Registry(&crate::schema::REGISTRY.0[..4]);
    assert_eq!(
        old.automatic(None).await.err(),
        Some(StorageError::UnsupportedSchema)
    );
}

#[tokio::test]
async fn column_move_rejects_stale_and_bad_anchors_and_rolls_back_at_each_boundary() {
    let (_dir, s, a) = setup().await;
    let first = s
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    for before in [Some(sid(2)), Some(sid(99))] {
        let b = a.clone();
        rejected(
            s.transact(move |ctx| {
                Ok(order::move_column(
                    ctx,
                    &b.credential(),
                    &Now,
                    &req(
                        1,
                        MoveColumn {
                            space: sid(1),
                            column: sid(2),
                            before,
                            expected: first.version,
                        },
                    ),
                )?)
            })
            .await,
            ErrorCode::InvalidRequest,
        );
    }
    for checkpoint in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
    ] {
        let b = a.clone();
        assert_eq!(
            s.transact_checked(
                move |ctx| Ok(order::move_column(
                    ctx,
                    &b.credential(),
                    &Now,
                    &req(
                        2,
                        MoveColumn {
                            space: sid(1),
                            column: sid(2),
                            before: None,
                            expected: first.version
                        }
                    )
                )?),
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
        assert_eq!(
            s.transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
                .await
                .unwrap(),
            first
        );
    }
    let b = a.clone();
    let moved = s
        .transact(move |ctx| {
            Ok(order::move_column(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    2,
                    MoveColumn {
                        space: sid(1),
                        column: sid(2),
                        before: None,
                        expected: first.version,
                    },
                ),
            )?)
        })
        .await
        .unwrap()
        .result;
    let b = a.clone();
    rejected(
        s.transact(move |ctx| {
            Ok(order::move_column(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    3,
                    MoveColumn {
                        space: sid(1),
                        column: sid(2),
                        before: Some(sid(3)),
                        expected: first.version,
                    },
                ),
            )?)
        })
        .await,
        ErrorCode::StaleState,
    );
    let b = a.clone();
    let noop = s
        .transact(move |ctx| {
            Ok(order::move_column(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    4,
                    MoveColumn {
                        space: sid(1),
                        column: sid(2),
                        before: None,
                        expected: moved.version,
                    },
                ),
            )?)
        })
        .await
        .unwrap()
        .result;
    assert_eq!(moved, noop);
}

#[tokio::test]
async fn moved_card_lifecycle_and_edit_keep_the_whole_placement_and_historical_memberships() {
    let (_dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let moved = move_card!(s, a, movement(2, 9, 3, None, first.version))
        .unwrap()
        .result;
    let edited = run!(s, a, edit, edit_req(3, 9, "Edited", "New", moved.version))
        .unwrap()
        .result;
    let done = run!(
        s,
        a,
        set_completed,
        req(
            4,
            zedone_core::cards::SetCardCompleted {
                space: sid(1),
                card: sid(9),
                completed: true,
                expected: edited.version
            }
        )
    )
    .unwrap()
    .result;
    let trashed = run!(
        s,
        a,
        trash,
        req(
            5,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: done.version
            }
        )
    )
    .unwrap()
    .result;
    let restored = run!(
        s,
        a,
        restore,
        req(
            6,
            RestoreCard {
                space: sid(1),
                card: sid(9),
                expected: trashed.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(restored.placements, moved.placements);
    assert_eq!(restored.positions, moved.positions);
    assert_eq!(restored.columns, [sid(3)]);
    assert_eq!(restored.titles, ["Edited"]);
    assert_eq!(restored.completed, [true]);
    let before = s
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    assert!(before.empty);
    let returned = move_card!(s, a, movement(7, 9, 2, None, restored.version))
        .unwrap()
        .result;
    let after = s
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    assert!(!after.empty);
    assert_ne!(before.version, after.version);
    move_card!(s, a, movement(8, 9, 3, None, returned.version)).unwrap();
    let again = s
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    assert!(again.empty);
    assert_ne!(before.version, again.version); // Removed placement remains causal evidence.
}
