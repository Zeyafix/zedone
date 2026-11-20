use super::*;
use zedone_core::{
    card_history::{AutosaveCard, BeginCardEditing, FinishCardEditing, RestoreCardText},
    cards::ResolveCardText,
    sync::SyncStore,
};

#[tokio::test]
async fn exchanged_ancestor_conflicts_keep_the_complete_history_in_both_cleanup_scopes() {
    for space_conflict in [false, true] {
        let (dir, left, a) = setup().await;
        run!(&left, a, create, creation(1, 9)).unwrap();
        let right = copy(&left, dir.path().join("history-peer.db"), &a).await;
        for (index, db) in [&left, &right].into_iter().enumerate() {
            let expected = if space_conflict {
                space(db).await.version
            } else {
                column(db, 2).await.version
            };
            let owner = a.clone();
            db.transact(move |tx| {
                if space_conflict {
                    spaces::rename(
                        tx,
                        &owner.credential(),
                        &Now,
                        &req(
                            2 + index as u8,
                            spaces::RenameSpace {
                                space: sid(1),
                                name: format!("Variant {index}"),
                                expected,
                            },
                        ),
                    )?;
                } else {
                    zedone_core::columns::rename(
                        tx,
                        &owner.credential(),
                        &Now,
                        &req(
                            2 + index as u8,
                            zedone_core::columns::RenameColumn {
                                space: sid(1),
                                column: sid(2),
                                name: format!("Variant {index}"),
                                expected,
                            },
                        ),
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        }
        exchange(&left, &right).await;
        let before = versions(&left, 9).await;
        for scope in [Some(9), None] {
            let preview =
                history_scope(&left, &a, if scope.is_some() { 4 } else { 6 }, scope).await;
            assert!(!preview.entries.is_empty());
            assert!(preview.entries.iter().all(|entry| entry.retained));
            cleanup_run!(
                &left,
                a,
                clear_history,
                clear(if scope.is_some() { 5 } else { 7 }, &preview)
            )
            .unwrap();
        }
        exchange(&left, &right).await;
        assert_eq!(versions(&left, 9).await, before);
        assert_eq!(versions(&right, 9).await, before);
    }
}

#[tokio::test]
async fn delivered_history_invalidates_preview_and_failed_clear_preserves_both_replicas() {
    let (dir, left, a) = setup().await;
    let base = run!(&left, a, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("history-peer.db"), &a).await;
    let preview = history_scope(&left, &a, 2, None).await;
    run!(
        &right,
        a,
        edit,
        edit_req(3, 9, "Remote version", "Remote pair", base.version)
    )
    .unwrap();
    exchange(&left, &right).await;
    let before = versions(&left, 9).await;
    rejected(
        cleanup_run!(&left, a, clear_history, clear(4, &preview)),
        ErrorCode::StaleState,
    );
    let preview = history_scope(&left, &a, 5, None).await;
    left.transact(|tx| {
        tx.connection().execute_batch("CREATE TRIGGER fail_history_clear BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;
        Ok(())
    }).await.unwrap();
    rejected(
        cleanup_run!(&left, a, clear_history, clear(6, &preview)),
        ErrorCode::Unavailable,
    );
    exchange(&left, &right).await;
    assert_eq!(versions(&left, 9).await, before);
    assert_eq!(versions(&right, 9).await, before);
    left.transact(|tx| {
        tx.connection()
            .execute_batch("DROP TRIGGER fail_history_clear")?;
        Ok(())
    })
    .await
    .unwrap();
    assert!(
        !cleanup_run!(&left, a, clear_history, clear(6, &preview))
            .unwrap()
            .replayed
    );
    exchange(&left, &right).await;
    assert!(versions(&right, 9).await.is_empty());
}

#[tokio::test]
async fn cleanup_requires_write_access_to_the_exact_space_after_exchange() {
    let (dir, left, a) = setup().await;
    run!(&left, a, create, creation(1, 9)).unwrap();
    let right = copy(&left, dir.path().join("history-peer.db"), &a).await;
    let reader = Arc::new(PendingAccess::generate().unwrap());
    let other_writer = Arc::new(PendingAccess::generate().unwrap());
    let (owner, read_key, write_key) = (a.clone(), reader.clone(), other_writer.clone());
    left.transact(move |tx| {
        spaces::create(
            tx,
            &owner.credential(),
            &Now,
            &req(
                70,
                spaces::CreateSpace {
                    space: sid(99),
                    name: "Other space".into(),
                },
            ),
        )?;
        tx.issue_access(
            &owner.credential(),
            &Now,
            &read_key,
            &GrantScope::Spaces([(sid(1), SpacePermission::Read)].into()),
            None,
        )?;
        tx.issue_access(
            &owner.credential(),
            &Now,
            &write_key,
            &GrantScope::Spaces([(sid(99), SpacePermission::ReadWrite)].into()),
            None,
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let before = versions(&left, 9).await;
    for (index, scope) in [Some(9), None].into_iter().enumerate() {
        let preview = history_scope(&left, &reader, 2 + index as u8, scope).await;
        for key in [&reader, &other_writer] {
            rejected(
                cleanup_run!(&left, key, clear_history, clear(3, &preview)),
                ErrorCode::AccessDenied,
            );
        }
    }
    exchange(&left, &right).await;
    assert_eq!(versions(&left, 9).await, before);
    assert_eq!(versions(&right, 9).await, before);
}

#[tokio::test]
async fn offline_cleanup_converges_in_both_scopes_without_losing_independent_cards_or_versions() {
    for whole_space in [false, true] {
        for clear_on_right in [false, true] {
            let (dir, left, a) = setup().await;
            let base = run!(&left, a, create, creation(1, 9)).unwrap().result;
            let trash = run!(&left, a, create, creation(2, 8)).unwrap().result;
            run!(
                &left,
                a,
                trash,
                req(
                    3,
                    TrashCard {
                        space: sid(1),
                        card: sid(8),
                        expected: trash.version,
                    }
                )
            )
            .unwrap();
            let right = copy(&left, dir.path().join("history-peer.db"), &a).await;
            let (cleaner, writer) = if clear_on_right {
                (&right, &left)
            } else {
                (&left, &right)
            };
            let before = history_scope(cleaner, &a, 4, (!whole_space).then_some(9)).await;
            let old_page = writer
                .transact(|tx| {
                    crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
                })
                .await
                .unwrap();
            cleanup_run!(cleaner, a, clear_history, clear(5, &before)).unwrap();
            let edited = run!(
                writer,
                a,
                edit,
                edit_req(6, 9, "Independent 😀", "Paired description", base.version)
            )
            .unwrap()
            .result;
            run!(writer, a, create, creation(7, 7)).unwrap();
            let independent = versions(writer, 9).await.last().unwrap().clone();
            let new_card_history = versions(writer, 7).await;
            exchange(cleaner, writer).await;
            exchange(writer, cleaner).await;
            for db in [cleaner, writer] {
                assert_eq!(
                    versions(db, 9).await.as_slice(),
                    std::slice::from_ref(&independent)
                );
                assert_eq!(versions(db, 7).await, new_card_history);
                assert_eq!(versions(db, 8).await.is_empty(), whole_space);
                assert_eq!(fresh(db, &a, 8, 9).await, edited);
                let page = old_page.clone();
                db.transact(move |tx| {
                    tx.push_sync(&sid(1), "old-peer", &vec![], &page.changes)?;
                    Ok(())
                })
                .await
                .unwrap();
                assert_eq!(
                    versions(db, 9).await.as_slice(),
                    std::slice::from_ref(&independent)
                );
            }
            assert!(
                cleanup_run!(cleaner, a, clear_history, clear(5, &before))
                    .unwrap()
                    .replayed
            );
            assert_eq!(
                versions(cleaner, 9).await.as_slice(),
                std::slice::from_ref(&independent)
            );
            rejected(
                cleanup_run!(cleaner, a, clear_history, clear(9, &before)),
                ErrorCode::StaleState,
            );
            let restored = run!(
                cleaner,
                a,
                restore_text,
                req(
                    10,
                    RestoreCardText {
                        space: sid(1),
                        card: sid(9),
                        text_version: independent.id.clone(),
                        expected: edited.version,
                    }
                )
            )
            .unwrap()
            .result;
            assert_eq!(restored.titles, edited.titles);
            exchange(cleaner, writer).await;
            let history = versions(cleaner, 9).await;
            assert_eq!(history.len(), 2);
            assert!(history[1].sequence > independent.sequence);
            assert_eq!(history, versions(writer, 9).await);
            left.shutdown().await.unwrap();
            right.shutdown().await.unwrap();
            let reopened = Storage::open(dir.path().join(if clear_on_right {
                "history-peer.db"
            } else {
                "cards.db"
            }))
            .await
            .unwrap();
            reopened.recover_editing_sessions().await.unwrap();
            assert!(
                cleanup_run!(&reopened, a, clear_history, clear(5, &before))
                    .unwrap()
                    .replayed
            );
            assert_eq!(versions(&reopened, 9).await, history);
        }
    }
}

#[tokio::test]
async fn concurrent_field_edits_keep_saved_pairs_and_conflicts_protect_history_until_resolution() {
    let (dir, left, a) = setup().await;
    let base = run!(&left, a, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("history-peer.db"), &a).await;
    run!(
        &left,
        a,
        edit,
        edit_req(2, 9, "Left title", &base.descriptions[0], base.version)
    )
    .unwrap();
    run!(
        &right,
        a,
        edit,
        edit_req(3, 9, &base.titles[0], "Right description", base.version)
    )
    .unwrap();
    exchange(&left, &right).await;
    let merged = fresh(&left, &a, 4, 9).await;
    assert_eq!(merged.titles, ["Left title"]);
    assert_eq!(merged.descriptions, ["Right description"]);
    let history = versions(&left, 9).await;
    assert_eq!(history, versions(&right, 9).await);
    assert_eq!(history.len(), 3);
    assert!(
        history
            .iter()
            .any(|v| v.title == "Left title" && v.description == base.descriptions[0])
    );
    assert!(
        history
            .iter()
            .any(|v| v.title == base.titles[0] && v.description == "Right description")
    );
    run!(
        &left,
        a,
        edit,
        edit_req(5, 9, "Conflict A", "Right description", merged.version)
    )
    .unwrap();
    run!(
        &right,
        a,
        edit,
        edit_req(6, 9, "Conflict B", "Right description", merged.version)
    )
    .unwrap();
    exchange(&left, &right).await;
    let conflicted = fresh(&left, &a, 7, 9).await;
    assert_eq!(conflicted.state, SpaceState::Conflicted);
    let protected = versions(&left, 9).await;
    for (db, scope) in [(&left, None), (&right, Some(9))] {
        let preview = history_scope(db, &a, 8, scope).await;
        assert!(preview.entries.iter().all(|e| e.retained));
        assert_eq!(preview.removable_bytes, 0);
        cleanup_run!(db, a, clear_history, clear(9, &preview)).unwrap();
        assert_eq!(versions(db, 9).await, protected);
        assert_eq!(fresh(db, &a, 10, 9).await, conflicted);
    }
    run!(
        &left,
        a,
        resolve_text,
        req(
            11,
            ResolveCardText {
                space: sid(1),
                card: sid(9),
                title: "Resolved".into(),
                description: "Both kept in history".into(),
                expected: conflicted.version,
            }
        )
    )
    .unwrap();
    exchange(&left, &right).await;
    assert_eq!(versions(&left, 9).await.len(), protected.len() + 1);
    assert_eq!(versions(&left, 9).await, versions(&right, 9).await);
}

#[tokio::test]
async fn independent_pending_series_survives_remote_cleanup_and_finishes_once_after_restart() {
    let (dir, left, a) = setup().await;
    let base = run!(&left, a, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("history-peer.db"), &a).await;
    run!(
        &right,
        a,
        begin_editing,
        req(
            2,
            BeginCardEditing {
                space: sid(1),
                card: sid(9),
                expected: base.version
            }
        )
    )
    .unwrap();
    let pending = run!(
        &right,
        a,
        autosave,
        req(
            3,
            AutosaveCard {
                space: sid(1),
                card: sid(9),
                session: "02".repeat(16),
                title: "Pending pair".into(),
                description: "Saved before cleanup".into(),
                expected: base.version,
            }
        )
    )
    .unwrap()
    .result;
    let preview = history_scope(&left, &a, 4, None).await;
    cleanup_run!(&left, a, clear_history, clear(5, &preview)).unwrap();
    exchange(&left, &right).await;
    assert!(versions(&right, 9).await.is_empty());
    right.shutdown().await.unwrap();
    let right = Storage::open(dir.path().join("history-peer.db"))
        .await
        .unwrap();
    right.recover_editing_sessions().await.unwrap();
    right.recover_editing_sessions().await.unwrap();
    let history = versions(&right, 9).await;
    assert_eq!(history.len(), 1);
    assert_eq!(history[0].title, "Pending pair");
    assert_eq!(history[0].description, "Saved before cleanup");
    rejected(
        run!(
            &right,
            a,
            finish_editing,
            req(
                6,
                FinishCardEditing {
                    space: sid(1),
                    card: sid(9),
                    session: "02".repeat(16),
                    expected: pending.version,
                }
            )
        ),
        ErrorCode::StaleState,
    );
    exchange(&left, &right).await;
    assert_eq!(versions(&left, 9).await, history);
    assert_eq!(fresh(&left, &a, 7, 9).await, pending);
}
