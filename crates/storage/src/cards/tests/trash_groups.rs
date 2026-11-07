use super::*;
use zedone_core::tags::{CreateTag, TagStore};
use zedone_core::{
    columns::{ColumnSnapshot, RestoreColumn},
    replay::StateVersion,
};

macro_rules! column_run {
    ($s:expr, $a:expr, $handler:ident, $r:expr) => {{
        let a = $a.clone();
        let r = $r;
        $s.transact(move |ctx| {
            Ok(zedone_core::columns::$handler(
                ctx,
                &a.credential(),
                &Now,
                &r,
            )?)
        })
        .await
    }};
}
fn trash(id: u8, expected: StateVersion) -> CommandRequest<TrashColumn> {
    req(
        id,
        TrashColumn {
            space: sid(1),
            column: sid(2),
            expected,
        },
    )
}
fn restore(id: u8, expected: StateVersion) -> CommandRequest<RestoreColumn> {
    req(
        id,
        RestoreColumn {
            space: sid(1),
            column: sid(2),
            expected,
        },
    )
}
async fn column(s: &Storage, id: u8) -> ColumnSnapshot {
    s.transact(move |ctx| Ok(ctx.column_snapshot(&sid(1), &sid(id))?))
        .await
        .unwrap()
}

#[tokio::test]
async fn column_group_restores_only_its_members_preserves_data_and_replays_after_restart() {
    let (dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let old = run!(s, a, create, creation(2, 8)).unwrap().result;
    let other = run!(s, a, create, creation(3, 7)).unwrap().result;
    let old = run!(
        s,
        a,
        trash,
        req(
            4,
            TrashCard {
                space: sid(1),
                card: sid(8),
                expected: old.version,
            }
        )
    )
    .unwrap()
    .result;
    let key = a.clone();
    s.transact(move |ctx| {
        zedone_core::ordering::move_card(
            ctx,
            &key.credential(),
            &Now,
            &req(
                5,
                zedone_core::ordering::MoveCard {
                    space: sid(1),
                    card: sid(7),
                    column: sid(3),
                    before: None,
                    expected: other.version,
                },
            ),
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let other = column(&s, 3).await;
    let before = column(&s, 2).await;
    assert_eq!(before.contents.as_ref().unwrap().len(), 2);
    let history = versions(&s, 9).await;
    let trashed = column_run!(s, a, trash, trash(6, before.version))
        .unwrap()
        .result;
    let members = trashed.contents.as_ref().unwrap();
    let one = members.iter().find(|c| c.card == sid(9)).unwrap();
    assert_eq!(one.state, SpaceState::Trashed);
    assert_eq!(one.deletion_groups, trashed.deletion_groups);
    assert_ne!(one.deletion_groups, old.deletion_groups);
    assert_eq!(fresh(&s, &a, 7, 8).await, old);
    assert_eq!(column(&s, 3).await, other);
    assert_eq!(versions(&s, 9).await, history);
    let back = column_run!(s, a, restore, restore(8, trashed.version))
        .unwrap()
        .result;
    let current = fresh(&s, &a, 9, 9).await;
    assert_eq!(current.state, SpaceState::Active);
    assert_eq!(current.titles, first.titles);
    assert_eq!(current.descriptions, first.descriptions);
    assert_eq!(current.tags, first.tags);
    assert_eq!(current.completed, first.completed);
    assert_eq!(current.positions, first.positions);
    assert_eq!(back.positions, before.positions);
    assert_eq!(fresh(&s, &a, 10, 8).await, old);
    assert_eq!(column(&s, 3).await, other);
    s.shutdown().await.unwrap();
    let s = Storage::open(dir.path().join("cards.db")).await.unwrap();
    let replay = column_run!(s, a, trash, trash(6, before.version)).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, trashed);
    assert_eq!(column(&s, 2).await, back);
    assert_eq!(fresh(&s, &a, 11, 9).await, current);
    let again = column_run!(s, a, trash, trash(12, back.version))
        .unwrap()
        .result;
    assert_ne!(again.deletion_groups, trashed.deletion_groups);
    assert_eq!(fresh(&s, &a, 13, 8).await, old);
    let repeated_restore = column_run!(s, a, restore, restore(8, trashed.version)).unwrap();
    assert!(repeated_restore.replayed);
    assert_eq!(repeated_restore.result, back);
    assert_eq!(column(&s, 2).await, again);
}

#[tokio::test]
async fn group_failure_rolls_back_every_member_pending_history_and_receipt() {
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
                expected: first.version,
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
                description: "last pair".into(),
                expected: first.version,
            }
        )
    )
    .unwrap();
    let before = column(&s, 2).await;
    let history = versions(&s, 9).await;
    assert_eq!(history.len(), 1);
    for checkpoint in [Stage::AfterWork, Stage::AfterChanges, Stage::BeforeCommit] {
        let key = a.clone();
        let expected = before.version;
        assert_eq!(
            s.transact_checked(
                move |ctx| {
                    Ok(zedone_core::columns::trash(
                        ctx,
                        &key.credential(),
                        &Now,
                        &trash(5, expected),
                    )?)
                },
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
        assert_eq!(column(&s, 2).await, before);
        assert_eq!(versions(&s, 9).await, history);
    }
    // Fail between individual members, after a pending text version may have
    // been finalized, rather than only at the outer transaction checkpoints.
    s.transact(|ctx| {
        ctx.connection().execute_batch("CREATE TRIGGER fail_group_member BEFORE DELETE ON card_editing BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;
        Ok(())
    }).await.unwrap();
    rejected(
        column_run!(s, a, trash, trash(5, before.version)),
        ErrorCode::Unavailable,
    );
    assert_eq!(column(&s, 2).await, before);
    assert_eq!(versions(&s, 9).await, history);
    s.transact(|ctx| {
        ctx.connection().execute_batch("DROP TRIGGER fail_group_member; CREATE TRIGGER fail_group_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;
        Ok(())
    }).await.unwrap();
    rejected(
        column_run!(s, a, trash, trash(5, before.version)),
        ErrorCode::Unavailable,
    );
    assert_eq!(column(&s, 2).await, before);
    assert_eq!(versions(&s, 9).await, history);
    s.transact(|ctx| {
        ctx.connection()
            .execute_batch("DROP TRIGGER fail_group_receipt")?;
        Ok(())
    })
    .await
    .unwrap();
    let key = a.clone();
    assert_eq!(
        s.transact_checked(
            move |ctx| {
                Ok(zedone_core::columns::trash(
                    ctx,
                    &key.credential(),
                    &Now,
                    &trash(5, before.version),
                )?)
            },
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
    let committed = column(&s, 2).await;
    let history = versions(&s, 9).await;
    assert_eq!(history.len(), 2);
    assert_eq!(
        (&history[1].title, &history[1].description),
        (&"pending".into(), &"last pair".into())
    );
    let replay = column_run!(s, a, trash, trash(5, before.version)).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, committed);
    for checkpoint in [Stage::AfterWork, Stage::AfterChanges, Stage::BeforeCommit] {
        let key = a.clone();
        let expected = committed.version;
        assert_eq!(
            s.transact_checked(
                move |ctx| {
                    Ok(zedone_core::columns::restore(
                        ctx,
                        &key.credential(),
                        &Now,
                        &restore(6, expected),
                    )?)
                },
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
        assert_eq!(column(&s, 2).await, committed);
        assert_eq!(versions(&s, 9).await, history);
    }
    let restored = column_run!(s, a, restore, restore(6, committed.version))
        .unwrap()
        .result;
    assert_eq!(restored.state, SpaceState::Active);
    let current = fresh(&s, &a, 7, 9).await;
    rejected(
        run!(
            s,
            a,
            autosave,
            req(
                8,
                AutosaveCard {
                    space: sid(1),
                    card: sid(9),
                    session: "03".repeat(16),
                    title: "old session".into(),
                    description: "".into(),
                    expected: current.version,
                }
            )
        ),
        ErrorCode::StaleState,
    );
    assert_eq!(versions(&s, 9).await, history);
}

#[tokio::test]
async fn group_freshness_covers_every_member_and_column_field_including_causal_return() {
    for field in [
        "title",
        "description",
        "completed",
        "position",
        "column",
        "tags",
        "lifecycle",
        "name",
        "column_position",
        "tag_name",
    ] {
        let (_dir, s, a) = setup().await;
        run!(s, a, create, creation(1, 9)).unwrap();
        s.transact(|ctx| {
            ctx.create_tag(&CreateTag {
                space: sid(1),
                tag: sid(5),
                name: "Tag".into(),
            })?;
            let card = object(ctx.read().document(&sid(1))?, &sid(9))?;
            let tags = map(ctx.read().document(&sid(1))?, &card, "tags")?;
            ctx.edit(&sid(1), |tx| {
                tx.put(&tags, sid(5), true)?;
                Ok(())
            })
        })
        .await
        .unwrap();
        let before = column(&s, 2).await;
        s.transact(move |ctx| {
            let graph = ctx.read().document(&sid(1))?;
            let card = object(graph, &sid(9))?;
            let col = crate::columns::object(graph, &sid(2))?;
            let tag = graph.get(ROOT, format!("tag:{}", sid(5)))?.unwrap().1;
            let tags = map(graph, &card, "tags")?;
            ctx.edit(&sid(1), |tx| {
                match field {
                    "completed" => {
                        tx.put(&card, field, true)?;
                    }
                    "position" => {
                        tx.put(&card, field, 7_u64)?;
                    }
                    "column" => {
                        tx.put(&card, field, sid(3))?;
                    }
                    "tags" => {
                        tx.put(&tags, sid(5), false)?;
                    }
                    "lifecycle" => {
                        tx.put(&card, field, format!("trash:{}:{}", sid(8), sid(9)))?;
                    }
                    "name" => {
                        tx.put(&col, "name", "renamed")?;
                    }
                    "column_position" => {
                        tx.put(&col, "position", 7_u64)?;
                    }
                    "tag_name" => {
                        tx.put(&tag, "name", "renamed")?;
                    }
                    _ => {
                        tx.put(&card, field, "changed")?;
                    }
                }
                Ok(())
            })
        })
        .await
        .unwrap();
        let after = column(&s, 2).await;
        assert_ne!(after.version, before.version, "{field}");
        rejected(
            column_run!(s, a, trash, trash(2, before.version)),
            ErrorCode::StaleState,
        );
        assert_eq!(column(&s, 2).await, after);
        // Returning the old value is a new causal action, never a freshening
        // of an old deletion. Include name changes of assigned tags.
        if field == "tag_name" {
            s.transact(|ctx| Ok(ctx.rename_tag(&sid(1), &sid(5), "Tag")?))
                .await
                .unwrap();
            let back = column(&s, 2).await;
            assert_ne!(back.version, before.version);
            rejected(
                column_run!(s, a, trash, trash(2, before.version)),
                ErrorCode::StaleState,
            );
            let removed = column_run!(s, a, trash, trash(3, back.version))
                .unwrap()
                .result;
            s.transact(|ctx| Ok(ctx.rename_tag(&sid(1), &sid(5), "After deletion")?))
                .await
                .unwrap();
            rejected(
                column_run!(s, a, restore, restore(4, removed.version)),
                ErrorCode::StaleState,
            );
            let current = column(&s, 2).await;
            let restored = column_run!(s, a, restore, restore(5, current.version))
                .unwrap()
                .result;
            assert_eq!(restored.contents.as_ref().unwrap()[0].tags, [sid(5)]);
        }
    }
}

#[tokio::test]
async fn group_restore_refuses_member_conflict_without_restoring_other_members() {
    let (_dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    run!(s, a, create, creation(2, 8)).unwrap();
    let before = column(&s, 2).await;
    let deleted = column_run!(s, a, trash, trash(3, before.version))
        .unwrap()
        .result;
    s.transact(|ctx| {
        let graph = ctx.read().document(&sid(1))?;
        let card = object(graph, &sid(9))?;
        let mut changes = Vec::new();
        for n in [21, 22] {
            let mut branch = graph.fork().with_actor(ActorId::from(vec![n]));
            let mut tx = branch.transaction();
            tx.put(&card, "title", format!("variant {n}"))?;
            tx.commit();
            changes.extend(branch.get_changes(&[]));
        }
        ctx.apply_changes(&sid(1), changes)
    })
    .await
    .unwrap();
    let conflicted = column(&s, 2).await;
    rejected(
        column_run!(s, a, restore, restore(4, deleted.version)),
        ErrorCode::StaleState,
    );
    rejected(
        column_run!(s, a, restore, restore(5, conflicted.version)),
        ErrorCode::Conflict,
    );
    assert_eq!(column(&s, 2).await, conflicted);
    assert_eq!(fresh(&s, &a, 6, 8).await.state, SpaceState::Trashed);
    assert_eq!(
        fresh(&s, &a, 7, 9).await.titles,
        ["variant 21", "variant 22"]
    );
}

#[tokio::test]
async fn concurrent_edit_and_group_deletion_serialize_without_losing_text() {
    let (dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let before = column(&s, 2).await;
    let other = Storage::open(dir.path().join("cards.db")).await.unwrap();
    let (k1, k2) = (a.clone(), a.clone());
    let (deleted, edited) = tokio::join!(
        s.transact(move |ctx| Ok(zedone_core::columns::trash(
            ctx,
            &k1.credential(),
            &Now,
            &trash(2, before.version)
        )?)),
        other.transact(move |ctx| Ok(core::edit(
            ctx,
            &k2.credential(),
            &Now,
            &edit_req(3, 9, "concurrent", "saved", first.version)
        )?))
    );
    assert_ne!(deleted.is_ok(), edited.is_ok());
    if let Ok(deleted) = deleted {
        rejected(edited, ErrorCode::StaleState);
        column_run!(s, a, restore, restore(4, deleted.result.version)).unwrap();
        assert_eq!(fresh(&s, &a, 5, 9).await.titles, ["Ёж 空間 😀"]);
    } else {
        rejected(deleted, ErrorCode::StaleState);
        assert_eq!(fresh(&s, &a, 5, 9).await.titles, ["concurrent"]);
    }
}

#[tokio::test]
async fn group_token_excludes_history_progress_reads_and_unrelated_column_data() {
    let (_dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    let mut elsewhere = creation(2, 8);
    elsewhere.parameters.column = sid(3);
    let other = run!(s, a, create, elsewhere).unwrap().result;
    let before = column(&s, 2).await;
    run!(s, a, edit, edit_req(3, 8, "elsewhere", "", other.version)).unwrap();
    fresh(&s, &a, 4, 9).await;
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
                "historical",
                "pair",
            )
        })?;
        ctx.set_peer_state(&sid(1), b"peer", &automerge::sync::State::new())
    })
    .await
    .unwrap();
    assert_eq!(column(&s, 2).await, before);
    let history = versions(&s, 9).await;
    let removed = column_run!(s, a, trash, trash(5, before.version))
        .unwrap()
        .result;
    assert_eq!(removed.state, SpaceState::Trashed);
    assert_eq!(versions(&s, 9).await, history);
}

#[tokio::test]
async fn tag_deletion_against_assignment_blocks_both_group_directions_atomically() {
    for restoring in [false, true] {
        let (_dir, s, a) = setup().await;
        run!(s, a, create, creation(1, 9)).unwrap();
        run!(s, a, create, creation(2, 8)).unwrap();
        s.transact(|ctx| {
            ctx.create_tag(&CreateTag {
                space: sid(1),
                tag: sid(5),
                name: "Tag".into(),
            })?;
            let graph = ctx.read().document(&sid(1))?;
            let card = object(graph, &sid(9))?;
            let tags = map(graph, &card, "tags")?;
            ctx.edit(&sid(1), |tx| {
                tx.put(&tags, sid(5), true)?;
                Ok(())
            })
        })
        .await
        .unwrap();
        if restoring {
            let before = column(&s, 2).await;
            column_run!(s, a, trash, trash(3, before.version)).unwrap();
        }
        s.transact(|ctx| {
            let graph = ctx.read().document(&sid(1))?;
            let tag = graph.get(ROOT, format!("tag:{}", sid(5)))?.unwrap().1;
            let card = object(graph, &sid(9))?;
            let assignments = map(graph, &card, "tags")?;
            let mut deletion = graph.fork().with_actor(ActorId::from(vec![21]));
            let mut tx = deletion.transaction();
            tx.put(&tag, "deleted", true)?;
            tx.commit();
            let mut assignment = graph.fork().with_actor(ActorId::from(vec![22]));
            for value in [false, true] {
                let mut tx = assignment.transaction();
                tx.put(&assignments, sid(5), value)?;
                tx.commit();
            }
            let mut changes = deletion.get_changes(&[]);
            changes.extend(assignment.get_changes(&[]));
            ctx.apply_changes(&sid(1), changes)
        })
        .await
        .unwrap();
        let conflicted = column(&s, 2).await;
        assert!(
            conflicted
                .contents
                .as_ref()
                .unwrap()
                .iter()
                .any(|c| c.state == SpaceState::Conflicted)
        );
        if restoring {
            rejected(
                column_run!(s, a, restore, restore(4, conflicted.version)),
                ErrorCode::Conflict,
            );
        } else {
            rejected(
                column_run!(s, a, trash, trash(4, conflicted.version)),
                ErrorCode::Conflict,
            );
        }
        assert_eq!(column(&s, 2).await, conflicted);
    }
}
