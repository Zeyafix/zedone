use super::*;
use zedone_core::{cards::ResolveCardText, replay::StateVersion, sync::SyncStore};

fn resolution(
    id: u8,
    title: &str,
    description: &str,
    expected: StateVersion,
) -> CommandRequest<ResolveCardText> {
    req(
        id,
        ResolveCardText {
            space: sid(1),
            card: sid(9),
            title: title.into(),
            description: description.into(),
            expected,
        },
    )
}
pub(super) async fn copy(
    db: &Storage,
    path: std::path::PathBuf,
    admin: &Arc<PendingAccess>,
) -> Storage {
    let page = db
        .transact(|tx| crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![]))
        .await
        .unwrap();
    let remote = Storage::open(path).await.unwrap();
    let owner = admin.clone();
    remote
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let selection = tx.read().sync_selection(&sid(1), &sid(99))?;
            tx.set_sync_selected(&sid(1), &sid(99), true, selection.version)?;
            tx.receive_sync_page(&sid(1), &sid(99), owner.subject(), &page)
        })
        .await
        .unwrap();
    remote
}
pub(super) async fn exchange(a: &Storage, b: &Storage) {
    let left = a
        .transact(|tx| crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![]))
        .await
        .unwrap();
    let right = b
        .transact(|tx| crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![]))
        .await
        .unwrap();
    for (db, page) in [(a, right), (b, left)] {
        db.transact(move |tx| {
            tx.push_sync(&sid(1), "test", &vec![], &page.changes)?;
            tx.push_sync(&sid(1), "test", &vec![], &page.changes)?;
            Ok(())
        })
        .await
        .unwrap();
    }
}
async fn conflicted() -> (TempDir, Storage, Storage, Arc<PendingAccess>) {
    conflicted_fields(true, true).await
}

async fn conflicted_fields(
    titles: bool,
    descriptions: bool,
) -> (TempDir, Storage, Storage, Arc<PendingAccess>) {
    let (dir, left, admin) = setup().await;
    let base = run!(&left, admin, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(
        &left,
        admin,
        edit,
        edit_req(
            2,
            9,
            if titles {
                "Left title"
            } else {
                &base.titles[0]
            },
            if descriptions {
                "Left description"
            } else {
                &base.descriptions[0]
            },
            base.version,
        )
    )
    .unwrap();
    run!(
        &right,
        admin,
        edit,
        edit_req(
            3,
            9,
            if titles {
                "Right title"
            } else {
                &base.titles[0]
            },
            if descriptions {
                "Right description"
            } else {
                &base.descriptions[0]
            },
            base.version,
        )
    )
    .unwrap();
    exchange(&left, &right).await;
    (dir, left, right, admin)
}

#[tokio::test]
async fn different_fields_merge_but_same_fields_block_ordinary_writes_until_explicit_resolution() {
    let (dir, left, admin) = setup().await;
    let base = run!(&left, admin, create, creation(1, 9)).unwrap().result;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(
        &left,
        admin,
        edit,
        edit_req(2, 9, "New title", &base.descriptions[0], base.version)
    )
    .unwrap();
    run!(
        &right,
        admin,
        edit,
        edit_req(3, 9, &base.titles[0], "New description", base.version)
    )
    .unwrap();
    exchange(&left, &right).await;
    let merged = fresh(&left, &admin, 4, 9).await;
    assert_eq!(merged, fresh(&right, &admin, 4, 9).await);
    assert_eq!(merged.titles, ["New title"]);
    assert_eq!(merged.descriptions, ["New description"]);
    assert_eq!(merged.state, SpaceState::Active);
    rejected(
        run!(
            &left,
            admin,
            resolve_text,
            resolution(5, "Bypass", "", merged.version)
        ),
        ErrorCode::Conflict,
    );
    run!(
        &left,
        admin,
        edit,
        edit_req(6, 9, "Title A", "New description", merged.version)
    )
    .unwrap();
    run!(
        &right,
        admin,
        edit,
        edit_req(7, 9, "Title B", "New description", merged.version)
    )
    .unwrap();
    exchange(&left, &right).await;
    let before = fresh(&left, &admin, 8, 9).await;
    assert_eq!(before.state, SpaceState::Conflicted);
    assert_eq!(before.descriptions, ["New description"]);
    rejected(
        run!(
            &left,
            admin,
            edit,
            edit_req(9, 9, "Title A", "Overwrite other field", before.version)
        ),
        ErrorCode::Conflict,
    );
    rejected(
        run!(
            &left,
            admin,
            set_completed,
            completion::request(10, true, before.version)
        ),
        ErrorCode::Conflict,
    );
    rejected(
        run!(
            &left,
            admin,
            trash,
            req(
                11,
                TrashCard {
                    space: sid(1),
                    card: sid(9),
                    expected: before.version
                }
            )
        ),
        ErrorCode::Conflict,
    );
    let resolved = run!(
        &left,
        admin,
        resolve_text,
        resolution(12, "Title B", "New description", before.version)
    )
    .unwrap()
    .result;
    assert_eq!(resolved.state, SpaceState::Active);
    assert_eq!(resolved.card, before.card);
    assert_eq!(resolved.columns, before.columns);
    assert_eq!(resolved.positions, before.positions);
    assert_eq!(resolved.completed, before.completed);
    assert_eq!(resolved.tags, before.tags);
    let history = versions(&left, 9).await;
    assert_eq!(history.len(), 6);
    assert_eq!(history.last().unwrap().title, "Title B");
    assert_eq!(history.last().unwrap().description, "New description");
    let changed = run!(
        &left,
        admin,
        edit,
        edit_req(13, 9, "Ordinary edit", "", resolved.version)
    )
    .unwrap()
    .result;
    assert_eq!(changed.state, SpaceState::Active);
}

#[tokio::test]
async fn choices_manual_merge_rights_stale_replay_restart_and_history_are_preserved() {
    let (dir, left, right, admin) = conflicted().await;
    let before = fresh(&left, &admin, 4, 9).await;
    let original_history = versions(&left, 9).await;
    assert_eq!(original_history.len(), 3);
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (owner, user) = (admin.clone(), member.clone());
    left.transact(move |tx| {
        tx.issue_access(
            &owner.credential(),
            &Now,
            &user,
            &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
            None,
        )
    })
    .await
    .unwrap();
    rejected(
        run!(
            &left,
            member,
            resolve_text,
            resolution(5, "Merged", "", before.version)
        ),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(
            &left,
            admin,
            resolve_text,
            resolution(6, " \t\n", "", before.version)
        ),
        ErrorCode::InvalidRequest,
    );
    rejected(
        run!(
            &left,
            admin,
            resolve_text,
            resolution(7, "Merged", "", StateVersion([0; 32]))
        ),
        ErrorCode::StaleState,
    );
    assert_eq!(fresh(&left, &admin, 8, 9).await, before);
    let chosen = run!(
        &left,
        admin,
        resolve_text,
        resolution(
            9,
            "Left title",
            "Left + right 😀\n<script>raw</script>",
            before.version
        )
    )
    .unwrap()
    .result;
    assert_eq!(chosen.state, SpaceState::Active);
    let history = versions(&left, 9).await;
    assert_eq!(&history[..3], &original_history);
    assert_eq!(history[3].title, "Left title");
    assert_eq!(history[3].description, chosen.descriptions[0]);
    assert_eq!(history[3].subject, admin.subject());
    left.shutdown().await.unwrap();
    let left = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert_eq!(fresh(&left, &admin, 10, 9).await, chosen);
    assert_eq!(versions(&left, 9).await, history);
    let late = run!(
        &left,
        admin,
        edit,
        edit_req(11, 9, "Later", "", chosen.version)
    )
    .unwrap()
    .result;
    let replay = run!(
        &left,
        admin,
        resolve_text,
        resolution(
            9,
            "Left title",
            "Left + right 😀\n<script>raw</script>",
            before.version
        )
    )
    .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, chosen);
    assert_eq!(fresh(&left, &admin, 12, 9).await, late);
    exchange(&left, &right).await;
    assert_eq!(fresh(&right, &admin, 12, 9).await, late);
    assert_eq!(versions(&right, 9).await, versions(&left, 9).await);
}

#[tokio::test]
async fn independent_solutions_and_unseen_late_edits_reconflict_then_converge_without_lost_history()
{
    let (dir, left, right, admin) = conflicted().await;
    let late = copy(&left, dir.path().join("late.db"), &admin).await;
    let before = fresh(&left, &admin, 4, 9).await;
    for (db, id, title, text) in [
        (&left, 5, "Choice A", "Merged A"),
        (&right, 6, "Choice B", "Merged B"),
        (&late, 7, "Late title", "Late text"),
    ] {
        run!(
            db,
            admin,
            resolve_text,
            resolution(id, title, text, before.version)
        )
        .unwrap();
    }
    exchange(&right, &left).await;
    let conflict = fresh(&left, &admin, 8, 9).await;
    assert_eq!(conflict.titles, ["Choice A", "Choice B"]);
    assert_eq!(conflict.descriptions, ["Merged A", "Merged B"]);
    assert_eq!(conflict, fresh(&right, &admin, 8, 9).await);
    assert_eq!(versions(&left, 9).await.len(), 5);
    run!(
        &left,
        admin,
        resolve_text,
        resolution(9, "Final", "Both", conflict.version)
    )
    .unwrap();
    exchange(&left, &late).await;
    let unseen = fresh(&left, &admin, 10, 9).await;
    assert_eq!(unseen.titles, ["Final", "Late title"]);
    assert_eq!(unseen.descriptions, ["Both", "Late text"]);
    exchange(&left, &right).await;
    exchange(&right, &late).await;
    assert_eq!(
        fresh(&left, &admin, 11, 9).await,
        fresh(&right, &admin, 11, 9).await
    );
    assert_eq!(
        fresh(&left, &admin, 12, 9).await,
        fresh(&late, &admin, 12, 9).await
    );
    assert_eq!(versions(&left, 9).await, versions(&right, 9).await);
    assert_eq!(versions(&left, 9).await, versions(&late, 9).await);
    assert_eq!(versions(&left, 9).await.len(), 7);
}

#[tokio::test]
async fn independent_saved_text_choices_survive_both_delivery_orders_and_repeats() {
    for (titles, descriptions) in [(true, true), (true, false), (false, true)] {
        for reverse in [false, true] {
            for (one, two) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
                let (_dir, left, right, admin) = conflicted_fields(titles, descriptions).await;
                let before = fresh(&left, &admin, 4, 9).await;
                let original_history = versions(&left, 9).await;
                for (db, choice, id) in [(&left, one, 5), (&right, two, 6)] {
                    run!(
                        db,
                        admin,
                        resolve_text,
                        resolution(
                            id,
                            &before.titles[if titles { choice } else { 0 }],
                            &before.descriptions[if descriptions { choice } else { 0 }],
                            before.version,
                        )
                    )
                    .unwrap();
                }
                if reverse {
                    exchange(&right, &left).await;
                } else {
                    exchange(&left, &right).await;
                }
                let actual = fresh(&left, &admin, 7, 9).await;
                assert_eq!(actual, fresh(&right, &admin, 7, 9).await);
                for (values, choices, conflicted) in [
                    (&actual.titles, &before.titles, titles),
                    (&actual.descriptions, &before.descriptions, descriptions),
                ] {
                    let mut expected = vec![
                        choices[if conflicted { one } else { 0 }].clone(),
                        choices[if conflicted { two } else { 0 }].clone(),
                    ];
                    expected.sort();
                    expected.dedup();
                    assert_eq!(*values, expected, "choices={one}/{two}, reverse={reverse}");
                }
                let history = versions(&left, 9).await;
                assert_eq!(history, versions(&right, 9).await);
                assert_eq!(history.len(), original_history.len() + 2);
                for version in original_history {
                    assert!(history.contains(&version));
                }
                left.shutdown().await.unwrap();
                right.shutdown().await.unwrap();
            }
        }
    }
}

#[tokio::test]
async fn text_resolution_preserves_other_conflicts_and_deletion_without_unlocking_ordinary_edits() {
    let (_dir, db, _right, admin) = conflicted().await;
    db.transact(|tx| {
        let graph = tx.read().document(&sid(1))?;
        let object = object(graph, &sid(9))?;
        let mut a = graph.fork().with_actor(ActorId::from(vec![81]));
        let mut b = graph.fork().with_actor(ActorId::from(vec![82]));
        for (graph, column) in [(&mut a, 2), (&mut b, 3)] {
            let mut edit = graph.transaction();
            edit.put(
                &object,
                "placement",
                serde_json::to_string(&zedone_core::ordering::CardPlacement {
                    column: sid(column),
                    rank: Legacy(0),
                })
                .unwrap(),
            )?;
            edit.put(
                &object,
                "lifecycle",
                format!("trash:{}:{}", sid(90), sid(91)),
            )?;
            edit.commit();
        }
        tx.apply_changes(&sid(1), a.get_changes(&[]))?;
        tx.apply_changes(&sid(1), b.get_changes(&[]))
    })
    .await
    .unwrap();
    let before = fresh(&db, &admin, 4, 9).await;
    assert_eq!(before.columns.len(), 2);
    let resolved = run!(
        &db,
        admin,
        resolve_text,
        resolution(5, "Chosen", "", before.version)
    )
    .unwrap()
    .result;
    assert_eq!(resolved.state, SpaceState::Conflicted);
    assert_eq!(resolved.columns, before.columns);
    assert_eq!(resolved.deletion_groups, before.deletion_groups);
    assert_eq!(resolved.positions, before.positions);
    assert_eq!(resolved.titles, ["Chosen"]);
    assert_eq!(resolved.descriptions, [""]);
    rejected(
        run!(
            &db,
            admin,
            edit,
            edit_req(6, 9, "Bypass", "", resolved.version)
        ),
        ErrorCode::Conflict,
    );
    rejected(
        run!(
            &db,
            admin,
            resolve_text,
            resolution(7, "Bypass", "", resolved.version)
        ),
        ErrorCode::Conflict,
    );
}

#[tokio::test]
async fn resolution_rolls_back_text_history_and_receipt_and_serializes_competitors() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
    ] {
        let (_dir, db, _right, admin) = conflicted().await;
        let before = fresh(&db, &admin, 4, 9).await;
        let history = versions(&db, 9).await;
        let secret = admin.credential().0.to_vec();
        let request = resolution(5, "Chosen", "", before.version);
        let failure = db
            .transact_checked(
                move |tx| {
                    Ok(core::resolve_text(
                        tx,
                        &Credential(&secret),
                        &Now,
                        &request,
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
        assert_eq!(failure.unwrap_err(), StorageError::Unavailable);
        assert_eq!(fresh(&db, &admin, 6, 9).await, before);
        assert_eq!(versions(&db, 9).await, history);
        let result = run!(
            &db,
            admin,
            resolve_text,
            resolution(5, "Chosen", "", before.version)
        )
        .unwrap();
        assert!(!result.replayed);
        assert_eq!(versions(&db, 9).await.len(), history.len() + 1);
    }
    let (_dir, db, _right, admin) = conflicted().await;
    let before = fresh(&db, &admin, 4, 9).await;
    let (a, b) = tokio::join!(
        async {
            run!(
                &db,
                admin,
                resolve_text,
                resolution(5, "A", "", before.version)
            )
        },
        async {
            run!(
                &db,
                admin,
                resolve_text,
                resolution(6, "B", "", before.version)
            )
        }
    );
    assert_eq!([a.is_ok(), b.is_ok()].into_iter().filter(|v| *v).count(), 1);
    assert!([a, b].into_iter().any(|v| matches!(
        v,
        Err(StorageError::Rejected(CommandError {
            code: ErrorCode::StaleState
        }))
    )));
    assert_eq!(versions(&db, 9).await.len(), 4);
}
