use super::*;
use zedone_core::{
    cleanup::{
        self, ClearHistory, HistoryPreview, InspectHistory, InspectTrash, PurgeTrash, TrashPreview,
        TrashSelection,
    },
    columns::{ColumnSnapshot, RestoreColumn},
    replay::StateVersion,
    spaces::{RestoreSpace, SpaceSnapshot, TrashSpace},
};
macro_rules! cleanup_run {
    ($s:expr, $a:expr, $handler:ident, $r:expr) => {{
        let a = $a.clone();
        let r = $r;
        $s.transact(move |ctx| Ok(cleanup::$handler(ctx, &a.credential(), &Now, &r)?))
            .await
    }};
}
macro_rules! space_run {
    ($s:expr, $a:expr, $handler:ident, $r:expr) => {{
        let a = $a.clone();
        let r = $r;
        $s.transact(move |ctx| Ok(spaces::$handler(ctx, &a.credential(), &Now, &r)?))
            .await
    }};
}
async fn space(s: &Storage) -> SpaceSnapshot {
    s.transact(|ctx| Ok(ctx.space_snapshot(&sid(1))?))
        .await
        .unwrap()
}
async fn column(s: &Storage, id: u8) -> ColumnSnapshot {
    s.transact(move |ctx| Ok(ctx.column_snapshot(&sid(1), &sid(id))?))
        .await
        .unwrap()
}
async fn trash_column(s: &Storage, a: &Arc<PendingAccess>, id: u8, command: u8) -> ColumnSnapshot {
    let expected = column(s, id).await.version;
    let a = a.clone();
    s.transact(move |ctx| {
        Ok(zedone_core::columns::trash(
            ctx,
            &a.credential(),
            &Now,
            &req(
                command,
                TrashColumn {
                    space: sid(1),
                    column: sid(id),
                    expected,
                },
            ),
        )?)
    })
    .await
    .unwrap()
    .result
}
async fn basket(
    s: &Storage,
    a: &Arc<PendingAccess>,
    id: u8,
    selection: TrashSelection,
) -> TrashPreview {
    cleanup_run!(
        s,
        a,
        inspect_trash,
        req(
            id,
            InspectTrash {
                space: sid(1),
                selection
            }
        )
    )
    .unwrap()
    .result
}
fn purge(id: u8, p: &TrashPreview) -> CommandRequest<PurgeTrash> {
    req(
        id,
        PurgeTrash {
            space: p.space.clone(),
            selection: p.selection.clone(),
            expected: p.version,
        },
    )
}
async fn history_scope(
    s: &Storage,
    a: &Arc<PendingAccess>,
    id: u8,
    card: Option<u8>,
) -> HistoryPreview {
    cleanup_run!(
        s,
        a,
        inspect_history,
        req(
            id,
            InspectHistory {
                space: sid(1),
                card: card.map(sid)
            }
        )
    )
    .unwrap()
    .result
}
fn clear(id: u8, p: &HistoryPreview) -> CommandRequest<ClearHistory> {
    req(
        id,
        ClearHistory {
            space: p.space.clone(),
            card: p.card.clone(),
            expected: p.version,
        },
    )
}
fn remove_space(id: u8, expected: StateVersion) -> CommandRequest<TrashSpace> {
    req(
        id,
        TrashSpace {
            space: sid(1),
            expected,
        },
    )
}

#[tokio::test]
async fn space_group_preserves_prior_trash_and_can_restore_one_card_elsewhere() {
    let (dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    run!(s, a, create, creation(2, 8)).unwrap();
    let old = trash_column(&s, &a, 2, 3).await;
    let mut other = creation(4, 7);
    other.parameters.column = sid(3);
    let active = run!(s, a, create, other).unwrap().result;
    let before = space(&s).await;
    let deleted = space_run!(s, a, trash, remove_space(5, before.version))
        .unwrap()
        .result;
    assert_eq!(deleted.state, SpaceState::Trashed);
    assert_eq!(column(&s, 2).await, old);
    assert_eq!(
        fresh(&s, &a, 6, 7).await.deletion_groups,
        deleted.deletion_groups
    );
    rejected(
        run!(
            s,
            a,
            restore,
            req(
                7,
                RestoreCard {
                    space: sid(1),
                    card: sid(9),
                    expected: fresh(&s, &a, 8, 9).await.version,
                    column: Some(sid(3)),
                    expected_column: Some(column(&s, 3).await.version)
                }
            )
        ),
        ErrorCode::InvalidRequest,
    );
    let back = space_run!(
        s,
        a,
        restore,
        req(
            9,
            RestoreSpace {
                space: sid(1),
                expected: deleted.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(back.state, SpaceState::Active);
    assert_eq!(column(&s, 2).await, old);
    assert_eq!(fresh(&s, &a, 10, 7).await.titles, active.titles);
    let target = column(&s, 3).await;
    let card = fresh(&s, &a, 11, 9).await;
    let command = req(
        12,
        RestoreCard {
            space: sid(1),
            card: sid(9),
            expected: card.version,
            column: Some(sid(3)),
            expected_column: Some(target.version),
        },
    );
    let moved = run!(s, a, restore, command).unwrap().result;
    assert_eq!(moved.columns, [sid(3)]);
    assert_eq!(moved.state, SpaceState::Active);
    assert_eq!(fresh(&s, &a, 13, 8).await.state, SpaceState::Trashed);
    let key = a.clone();
    let expected = column(&s, 2).await.version;
    s.transact(move |ctx| {
        Ok(zedone_core::columns::restore(
            ctx,
            &key.credential(),
            &Now,
            &req(
                14,
                RestoreColumn {
                    space: sid(1),
                    column: sid(2),
                    expected,
                },
            ),
        )?)
    })
    .await
    .unwrap();
    assert_eq!(fresh(&s, &a, 15, 9).await, moved);
    s.shutdown().await.unwrap();
    let s = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert!(
        space_run!(s, a, trash, remove_space(5, before.version))
            .unwrap()
            .replayed
    );
    assert_eq!(space(&s).await.state, SpaceState::Active);
    assert_eq!(fresh(&s, &a, 16, 9).await, moved);
}

#[tokio::test]
async fn restore_destination_is_checked_causally_and_does_not_revive_old_sessions() {
    let (_dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    trash_column(&s, &a, 2, 2).await;
    let card = fresh(&s, &a, 3, 9).await;
    let target = column(&s, 3).await;
    s.transact(|ctx| {
        ctx.rename_column(&sid(1), &sid(3), "new")?;
        ctx.rename_column(&sid(1), &sid(3), "3")?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        run!(
            s,
            a,
            restore,
            req(
                4,
                RestoreCard {
                    space: sid(1),
                    card: sid(9),
                    expected: card.version,
                    column: Some(sid(3)),
                    expected_column: Some(target.version)
                }
            )
        ),
        ErrorCode::StaleState,
    );
    rejected(
        run!(
            s,
            a,
            restore,
            req(
                5,
                RestoreCard {
                    space: sid(1),
                    card: sid(9),
                    expected: card.version,
                    column: Some(sid(3)),
                    expected_column: None
                }
            )
        ),
        ErrorCode::InvalidRequest,
    );
    assert_eq!(fresh(&s, &a, 6, 9).await, card);
}

#[tokio::test]
async fn space_token_covers_tags_content_aba_but_excludes_user_history_and_peer_progress() {
    let (_dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    let before = space(&s).await;
    s.transact(|ctx| {
        let graph = ctx.read().document(&sid(1))?;
        let card = object(graph, &sid(9))?;
        let history = map(graph, &card, "history")?;
        ctx.edit(&sid(1), |tx| {
            append_version(
                tx,
                &history,
                &format!("{}:{}", sid(6), sid(7)),
                1,
                "old",
                "text",
            )
        })?;
        ctx.set_peer_state(&sid(1), b"peer", &automerge::sync::State::new())
    })
    .await
    .unwrap();
    assert_eq!(space(&s).await.version, before.version);
    s.transact(|ctx| {
        ctx.rename_column(&sid(1), &sid(3), "changed")?;
        ctx.rename_column(&sid(1), &sid(3), "3")?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        space_run!(s, a, trash, remove_space(2, before.version)),
        ErrorCode::StaleState,
    );
    let current = space(&s).await;
    use zedone_core::tags::{CreateTag, TagStore};
    s.transact(|ctx| {
        Ok(ctx.create_tag(&CreateTag {
            space: sid(1),
            tag: sid(5),
            name: "Tag".into(),
        })?)
    })
    .await
    .unwrap();
    rejected(
        space_run!(s, a, trash, remove_space(3, current.version)),
        ErrorCode::StaleState,
    );
    assert_eq!(fresh(&s, &a, 4, 9).await.state, SpaceState::Active);
}

#[tokio::test]
async fn purge_column_includes_earlier_trash_but_preserves_restored_card_and_technical_journal() {
    let (dir, s, a) = setup().await;
    let one = run!(s, a, create, creation(1, 9)).unwrap().result;
    run!(s, a, create, creation(2, 8)).unwrap();
    run!(
        s,
        a,
        trash,
        req(
            3,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: one.version
            }
        )
    )
    .unwrap();
    trash_column(&s, &a, 2, 4).await;
    let restored = fresh(&s, &a, 5, 8).await;
    let target = column(&s, 3).await;
    let restored = run!(
        s,
        a,
        restore,
        req(
            6,
            RestoreCard {
                space: sid(1),
                card: sid(8),
                expected: restored.version,
                column: Some(sid(3)),
                expected_column: Some(target.version)
            }
        )
    )
    .unwrap()
    .result;
    let p = basket(
        &s,
        &a,
        7,
        TrashSelection::Objects {
            columns: vec![sid(2)],
            cards: vec![],
        },
    )
    .await;
    assert_eq!(
        p.cards.iter().map(|c| c.card.clone()).collect::<Vec<_>>(),
        [sid(9)]
    );
    let changes = s
        .transact(|ctx| Ok(ctx.read().document(&sid(1))?.get_changes(&[]).len()))
        .await
        .unwrap();
    cleanup_run!(s, a, purge_trash, purge(8, &p)).unwrap();
    rejected(
        run!(
            s,
            a,
            read,
            req(
                9,
                ReadCard {
                    space: sid(1),
                    card: sid(9)
                }
            )
        ),
        ErrorCode::NotFound,
    );
    assert_eq!(fresh(&s, &a, 10, 8).await, restored);
    assert_eq!(space(&s).await.contents.unwrap().columns.len(), 1);
    assert!(
        s.transact(move |ctx| Ok(ctx.read().document(&sid(1))?.get_changes(&[]).len() > changes))
            .await
            .unwrap()
    );
    rejected(run!(s, a, create, creation(11, 9)), ErrorCode::NotFound); // original column is gone
    let mut reused = creation(12, 9);
    reused.parameters.column = sid(3);
    rejected(run!(s, a, create, reused), ErrorCode::StaleState);
    s.shutdown().await.unwrap();
    let s = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert!(
        cleanup_run!(s, a, purge_trash, purge(8, &p))
            .unwrap()
            .replayed
    );
    assert_eq!(fresh(&s, &a, 13, 8).await, restored);
}

#[tokio::test]
async fn purge_all_and_space_remove_only_confirmed_scope_and_space_is_not_resurrected() {
    for whole_space in [false, true] {
        let (_dir, s, a) = setup().await;
        let card = run!(s, a, create, creation(1, 9)).unwrap().result;
        if whole_space {
            let before = space(&s).await;
            space_run!(s, a, trash, remove_space(2, before.version)).unwrap();
        } else {
            run!(
                s,
                a,
                trash,
                req(
                    2,
                    TrashCard {
                        space: sid(1),
                        card: sid(9),
                        expected: card.version
                    }
                )
            )
            .unwrap();
        }
        let p = basket(
            &s,
            &a,
            3,
            if whole_space {
                TrashSelection::Space
            } else {
                TrashSelection::All
            },
        )
        .await;
        assert_eq!(p.includes_space, whole_space);
        cleanup_run!(s, a, purge_trash, purge(4, &p)).unwrap();
        if whole_space {
            rejected(
                space_run!(s, a, read, req(5, spaces::ReadSpace { space: sid(1) })),
                ErrorCode::NotFound,
            );
            let list = space_run!(
                s,
                a,
                list,
                req(
                    6,
                    spaces::ListSpaces {
                        include_trashed: true
                    }
                )
            )
            .unwrap();
            assert!(list.result.spaces.is_empty());
            rejected(
                space_run!(
                    s,
                    a,
                    create,
                    req(
                        7,
                        CreateSpace {
                            space: sid(1),
                            name: "Reuse".into()
                        }
                    )
                ),
                ErrorCode::StaleState,
            );
        } else {
            assert_eq!(space(&s).await.contents.unwrap().columns.len(), 2);
        }
    }
}

#[tokio::test]
async fn purge_refuses_changed_scope_and_every_conflict_without_partial_effects() {
    let (_dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    run!(s, a, create, creation(2, 8)).unwrap();
    trash_column(&s, &a, 2, 3).await;
    let p = basket(&s, &a, 4, TrashSelection::All).await;
    s.transact(|ctx| {
        ctx.rename_column(&sid(1), &sid(2), "changed")?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        cleanup_run!(s, a, purge_trash, purge(5, &p)),
        ErrorCode::StaleState,
    );
    s.transact(|ctx| {
        let graph = ctx.read().document(&sid(1))?;
        let card = object(graph, &sid(9))?;
        let mut changes = Vec::new();
        for actor in [21, 22] {
            let mut fork = graph.fork();
            fork.set_actor(ActorId::from(vec![actor]));
            let mut tx = fork.transaction();
            tx.put(&card, "title", format!("variant {actor}"))?;
            let (hash, _) = tx.commit();
            changes.push(fork.get_change_by_hash(&hash.unwrap()).unwrap());
        }
        ctx.apply_changes(&sid(1), changes)
    })
    .await
    .unwrap();
    let p = basket(&s, &a, 6, TrashSelection::All).await;
    assert!(p.conflicts.contains(&format!("card:{}", sid(9))));
    let before = space(&s).await;
    rejected(
        cleanup_run!(s, a, purge_trash, purge(7, &p)),
        ErrorCode::Conflict,
    );
    assert_eq!(space(&s).await, before);
    rejected(
        space_run!(s, a, trash, remove_space(8, before.version)),
        ErrorCode::Conflict,
    );
}

#[tokio::test]
async fn history_clear_preserves_current_fields_empty_history_remains_editable_and_retry_is_frozen()
{
    let (dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let current = run!(s, a, edit, edit_req(2, 9, "second", "text", first.version))
        .unwrap()
        .result;
    let p = history_scope(&s, &a, 3, Some(9)).await;
    assert_eq!(p.entries.len(), 2);
    assert!(p.text_bytes > 0);
    assert_eq!(p.text_bytes, p.removable_bytes);
    cleanup_run!(s, a, clear_history, clear(4, &p)).unwrap();
    assert_eq!(fresh(&s, &a, 5, 9).await, current);
    assert!(versions(&s, 9).await.is_empty());
    let edited = run!(s, a, edit, edit_req(6, 9, "third", "new", current.version))
        .unwrap()
        .result;
    assert_eq!(versions(&s, 9).await.len(), 1);
    s.shutdown().await.unwrap();
    let s = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert!(
        cleanup_run!(s, a, clear_history, clear(4, &p))
            .unwrap()
            .replayed
    );
    assert_eq!(versions(&s, 9).await.len(), 1);
    assert_eq!(fresh(&s, &a, 7, 9).await, edited);
}

#[tokio::test]
async fn history_space_scope_is_stale_after_new_version_and_conflicted_history_is_retained() {
    let (_dir, s, a) = setup().await;
    let one = run!(s, a, create, creation(1, 9)).unwrap().result;
    let other = run!(s, a, create, creation(2, 8)).unwrap().result;
    let p = history_scope(&s, &a, 3, None).await;
    run!(s, a, edit, edit_req(4, 8, "later", "new", other.version)).unwrap();
    rejected(
        cleanup_run!(s, a, clear_history, clear(5, &p)),
        ErrorCode::StaleState,
    );
    s.transact(|ctx| {
        let graph = ctx.read().document(&sid(1))?;
        let card = object(graph, &sid(9))?;
        let mut changes = Vec::new();
        for actor in [21, 22] {
            let mut fork = graph.fork();
            fork.set_actor(ActorId::from(vec![actor]));
            let mut tx = fork.transaction();
            tx.put(&card, "title", format!("variant {actor}"))?;
            let (hash, _) = tx.commit();
            changes.push(fork.get_change_by_hash(&hash.unwrap()).unwrap());
        }
        ctx.apply_changes(&sid(1), changes)
    })
    .await
    .unwrap();
    let p = history_scope(&s, &a, 6, None).await;
    assert!(
        p.entries
            .iter()
            .find(|e| e.card == one.card)
            .unwrap()
            .retained
    );
    cleanup_run!(s, a, clear_history, clear(7, &p)).unwrap();
    assert_eq!(versions(&s, 9).await.len(), 1);
    assert!(versions(&s, 8).await.is_empty());
    assert_eq!(fresh(&s, &a, 8, 9).await.state, SpaceState::Conflicted);
}

#[tokio::test]
async fn destructive_commands_rollback_at_every_boundary_and_unknown_results_replay() {
    for history in [false, true] {
        for stage in [
            Stage::AfterWork,
            Stage::AfterChanges,
            Stage::BeforeCommit,
            Stage::AfterCommit,
        ] {
            let (_dir, s, a) = setup().await;
            let card = run!(s, a, create, creation(1, 9)).unwrap().result;
            if !history {
                run!(
                    s,
                    a,
                    trash,
                    req(
                        2,
                        TrashCard {
                            space: sid(1),
                            card: sid(9),
                            expected: card.version
                        }
                    )
                )
                .unwrap();
            }
            let t = basket(&s, &a, 3, TrashSelection::All).await;
            let h = history_scope(&s, &a, 4, None).await;
            let before = space(&s).await;
            let versions_before = versions(&s, 9).await;
            let key = a.clone();
            let command = if history {
                serde_json::to_value(clear(5, &h)).unwrap()
            } else {
                serde_json::to_value(purge(5, &t)).unwrap()
            };
            let result = s
                .transact_checked(
                    move |ctx| {
                        if history {
                            Ok(cleanup::clear_history(
                                ctx,
                                &key.credential(),
                                &Now,
                                &serde_json::from_value(command).unwrap(),
                            )?
                            .replayed)
                        } else {
                            Ok(cleanup::purge_trash(
                                ctx,
                                &key.credential(),
                                &Now,
                                &serde_json::from_value(command).unwrap(),
                            )?
                            .replayed)
                        }
                    },
                    move |point| {
                        if point == stage {
                            Err(StorageError::Unavailable)
                        } else {
                            Ok(())
                        }
                    },
                )
                .await;
            if stage == Stage::AfterCommit {
                assert_eq!(result.unwrap_err(), StorageError::OutcomeUnknown);
            } else {
                assert_eq!(result.unwrap_err(), StorageError::Unavailable);
                assert_eq!(space(&s).await, before);
                assert_eq!(versions(&s, 9).await, versions_before);
            }
            let replayed = if history {
                cleanup_run!(s, a, clear_history, clear(5, &h))
                    .unwrap()
                    .replayed
            } else {
                cleanup_run!(s, a, purge_trash, purge(5, &t))
                    .unwrap()
                    .replayed
            };
            assert_eq!(replayed, stage == Stage::AfterCommit);
        }
    }
}

#[tokio::test]
async fn independent_versions_survive_causal_cleanup_and_new_sequence_does_not_reset() {
    let (_dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let current = run!(s, a, edit, edit_req(2, 9, "second", "text", first.version))
        .unwrap()
        .result;
    let changes = s
        .transact(|ctx| {
            let graph = ctx.read().document(&sid(1))?;
            let card = object(graph, &sid(9))?;
            let history = map(graph, &card, "history")?;
            let mut fork = graph.fork();
            fork.set_actor(ActorId::from(vec![21]));
            let mut tx = fork.transaction();
            append_version(
                &mut tx,
                &history,
                &format!("{}:{}", sid(21), sid(22)),
                2,
                "independent",
                "saved",
            )?;
            let (hash, _) = tx.commit();
            Ok(vec![fork.get_change_by_hash(&hash.unwrap()).unwrap()])
        })
        .await
        .unwrap();
    let p = history_scope(&s, &a, 3, None).await;
    cleanup_run!(s, a, clear_history, clear(4, &p)).unwrap();
    run!(s, a, edit, edit_req(5, 9, "third", "text", current.version)).unwrap();
    assert_eq!(versions(&s, 9).await[0].sequence, 2);
    s.transact(move |ctx| ctx.apply_changes(&sid(1), changes))
        .await
        .unwrap();
    let versions = versions(&s, 9).await;
    assert_eq!(versions.len(), 2);
    assert!(versions.iter().any(|v| v.title == "independent"));
}

#[tokio::test]
async fn clearing_known_history_does_not_end_or_delete_a_pending_text_series() {
    use zedone_core::card_history::{AutosaveCard, BeginCardEditing};
    let (_dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    run!(
        s,
        a,
        begin_editing,
        req(
            2,
            BeginCardEditing {
                space: sid(1),
                card: sid(9),
                expected: first.version
            }
        )
    )
    .unwrap();
    let saved = run!(
        s,
        a,
        autosave,
        req(
            3,
            AutosaveCard {
                space: sid(1),
                card: sid(9),
                session: "02".repeat(16),
                title: "pending".into(),
                description: "saved pair".into(),
                expected: first.version
            }
        )
    )
    .unwrap()
    .result;
    let p = history_scope(&s, &a, 4, None).await;
    assert_eq!(p.entries.len(), 1);
    cleanup_run!(s, a, clear_history, clear(5, &p)).unwrap();
    assert!(versions(&s, 9).await.is_empty());
    assert_eq!(fresh(&s, &a, 6, 9).await, saved);
    let saved = run!(
        s,
        a,
        autosave,
        req(
            7,
            AutosaveCard {
                space: sid(1),
                card: sid(9),
                session: "02".repeat(16),
                title: "pending later".into(),
                description: "saved pair".into(),
                expected: saved.version
            }
        )
    )
    .unwrap()
    .result;
    run!(
        s,
        a,
        finish_editing,
        req(
            8,
            zedone_core::card_history::FinishCardEditing {
                space: sid(1),
                card: sid(9),
                session: "02".repeat(16),
                expected: saved.version
            }
        )
    )
    .unwrap();
    assert_eq!(versions(&s, 9).await[0].title, "pending later");
}

#[tokio::test]
async fn sql_failure_after_cleanup_work_rolls_back_data_history_and_receipt() {
    for history in [false, true] {
        let (_dir, s, a) = setup().await;
        let card = run!(s, a, create, creation(1, 9)).unwrap().result;
        if !history {
            run!(
                s,
                a,
                trash,
                req(
                    2,
                    TrashCard {
                        space: sid(1),
                        card: sid(9),
                        expected: card.version
                    }
                )
            )
            .unwrap();
        }
        let t = basket(&s, &a, 3, TrashSelection::All).await;
        let h = history_scope(&s, &a, 4, None).await;
        let before = space(&s).await;
        let old = versions(&s, 9).await;
        s.transact(|ctx| {ctx.connection().execute_batch("CREATE TRIGGER fail_cleanup BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;Ok(())}).await.unwrap();
        if history {
            rejected(
                cleanup_run!(s, a, clear_history, clear(5, &h)),
                ErrorCode::Unavailable,
            );
        } else {
            rejected(
                cleanup_run!(s, a, purge_trash, purge(5, &t)),
                ErrorCode::Unavailable,
            );
        }
        assert_eq!(space(&s).await, before);
        assert_eq!(versions(&s, 9).await, old);
        s.transact(|ctx| {
            ctx.connection()
                .execute_batch("DROP TRIGGER fail_cleanup")?;
            Ok(())
        })
        .await
        .unwrap();
        if history {
            assert!(
                !cleanup_run!(s, a, clear_history, clear(5, &h))
                    .unwrap()
                    .replayed
            );
        } else {
            assert!(
                !cleanup_run!(s, a, purge_trash, purge(5, &t))
                    .unwrap()
                    .replayed
            );
        }
    }
}

#[tokio::test]
async fn concurrent_purge_and_restore_have_only_one_effect() {
    let (dir, s, a) = setup().await;
    let card = run!(s, a, create, creation(1, 9)).unwrap().result;
    let card = run!(
        s,
        a,
        trash,
        req(
            2,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: card.version
            }
        )
    )
    .unwrap()
    .result;
    let t = basket(&s, &a, 3, TrashSelection::All).await;
    let other = Storage::open(dir.path().join("cards.db")).await.unwrap();
    let (k1, k2) = (a.clone(), a.clone());
    let (purged, restored) = tokio::join!(
        s.transact(move |ctx| Ok(cleanup::purge_trash(
            ctx,
            &k1.credential(),
            &Now,
            &purge(4, &t)
        )?)),
        other.transact(move |ctx| Ok(core::restore(
            ctx,
            &k2.credential(),
            &Now,
            &req(
                5,
                RestoreCard {
                    space: sid(1),
                    card: sid(9),
                    expected: card.version,
                    column: None,
                    expected_column: None
                }
            )
        )?))
    );
    assert_ne!(purged.is_ok(), restored.is_ok());
    if purged.is_ok() {
        rejected(restored, ErrorCode::NotFound);
    } else {
        rejected(purged, ErrorCode::StaleState);
        assert_eq!(fresh(&s, &a, 6, 9).await.state, SpaceState::Active);
    }
}

#[tokio::test]
async fn space_group_sql_failure_rolls_back_every_member_and_pending_history() {
    use zedone_core::card_history::{AutosaveCard, BeginCardEditing};
    let (_dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    run!(s, a, create, creation(2, 8)).unwrap();
    run!(
        s,
        a,
        begin_editing,
        req(
            3,
            BeginCardEditing {
                space: sid(1),
                card: sid(9),
                expected: first.version
            }
        )
    )
    .unwrap();
    run!(
        s,
        a,
        autosave,
        req(
            4,
            AutosaveCard {
                space: sid(1),
                card: sid(9),
                session: "03".repeat(16),
                title: "pending".into(),
                description: "saved pair".into(),
                expected: first.version
            }
        )
    )
    .unwrap();
    let before = space(&s).await;
    let old = versions(&s, 9).await;
    // card 8 is processed before card 9; fail after one member already changed.
    s.transact(|ctx| {ctx.connection().execute_batch("CREATE TRIGGER fail_space BEFORE DELETE ON card_editing BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;Ok(())}).await.unwrap();
    rejected(
        space_run!(s, a, trash, remove_space(5, before.version)),
        ErrorCode::Unavailable,
    );
    assert_eq!(space(&s).await, before);
    assert_eq!(versions(&s, 9).await, old);
    s.transact(|ctx| {
        ctx.connection().execute_batch("DROP TRIGGER fail_space")?;
        Ok(())
    })
    .await
    .unwrap();
    let deleted = space_run!(s, a, trash, remove_space(5, before.version))
        .unwrap()
        .result;
    assert_eq!(deleted.state, SpaceState::Trashed);
    assert_eq!(versions(&s, 9).await.len(), 2);
}

#[tokio::test]
async fn purge_all_of_trashed_space_never_hides_a_new_active_descendant() {
    let (_dir, s, a) = setup().await;
    let before = space(&s).await;
    space_run!(s, a, trash, remove_space(1, before.version)).unwrap();
    // Synthetic concurrent/new descendant: ordinary commands cannot create it
    // below a trashed parent. Cleanup must still fail closed on this graph.
    s.transact(|ctx| {
        ctx.create_column(&sid(1), &sid(4), "independent")?;
        Ok(())
    })
    .await
    .unwrap();
    let before = space(&s).await;
    let p = basket(&s, &a, 2, TrashSelection::All).await;
    assert!(p.includes_space);
    assert!(p.conflicts.contains(&format!("column:{}", sid(4))));
    rejected(
        cleanup_run!(s, a, purge_trash, purge(3, &p)),
        ErrorCode::Conflict,
    );
    assert_eq!(space(&s).await, before);
}
