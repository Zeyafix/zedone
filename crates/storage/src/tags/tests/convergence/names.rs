use super::*;
use zedone_core::tag_names::{self, ResolveTagNames, TagNameChoice};

async fn bridge() -> (TempDir, Storage, Storage, Arc<PendingAccess>) {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    assign(&left, &admin, 2, 3, 5, true).await;
    let t = tag(&left, 5).await;
    run!(left, admin, core::rename, rename(3, 5, "Личное", t.version)).unwrap();
    run!(left, admin, core::create, create(4, 6, "Работа")).unwrap();
    assign(&left, &admin, 5, 3, 6, true).await;
    run!(right, admin, core::create, create(6, 7, "РАБОТА")).unwrap();
    assign(&right, &admin, 7, 3, 7, true).await;
    exchange(&left, &right, true).await;
    (dir, left, right, admin)
}
async fn choice(db: &Storage, id: u8, names: [&str; 3]) -> CommandRequest<ResolveTagNames> {
    let mut choices = Vec::new();
    for (tag_id, name) in [5, 6, 7].into_iter().zip(names) {
        choices.push(TagNameChoice {
            tag: sid(tag_id),
            name: name.into(),
            expected: tag(db, tag_id).await.version,
        });
    }
    req(
        id,
        ResolveTagNames {
            space: sid(1),
            tag: sid(7),
            names: choices,
        },
    )
}

#[tokio::test]
async fn bridge_requires_complete_unique_current_names_preserves_ids_links_and_replay() {
    for reverse in [false, true] {
        let (dir, left, right, admin) = bridge().await;
        let before = listing(&left).await;
        let c = card(&left, 3).await;
        for t in &before {
            assert_eq!(t.name_group, [sid(5), sid(6), sid(7)]);
        }
        for names in [["", "Все", "Всё"], ["Work", " WORK ", "Всё"]] {
            rejected(
                run!(
                    left,
                    admin,
                    tag_names::resolve,
                    choice(&left, 20, names).await
                ),
                ErrorCode::InvalidRequest,
            );
            assert_eq!(listing(&left).await, before);
        }
        let mut incomplete = choice(&left, 21, ["Личное", "Все", "Всё"]).await;
        incomplete.parameters.names.pop();
        rejected(
            run!(left, admin, tag_names::resolve, incomplete),
            ErrorCode::InvalidRequest,
        );
        run!(left, admin, core::create, create(22, 8, "Занято")).unwrap();
        let fresh = listing(&left).await;
        rejected(
            run!(
                left,
                admin,
                tag_names::resolve,
                choice(&left, 23, ["Занято", "Все", "Всё"]).await
            ),
            ErrorCode::Conflict,
        );
        assert_eq!(listing(&left).await, fresh);
        let stale = choice(&left, 24, ["One", "Two", "Three"]).await;
        let request = choice(&left, 25, ["Личное", "Все", "Всё"]).await;
        let saved = serde_json::to_vec(&request).unwrap();
        run!(left, admin, tag_names::resolve, request).unwrap();
        rejected(
            run!(left, admin, tag_names::resolve, stale),
            ErrorCode::StaleState,
        );
        exchange(&left, &right, reverse).await;
        for db in [&left, &right] {
            let current = listing(db).await;
            assert!(current.iter().all(|t| t.state == TagState::Active));
            assert_eq!(current.len(), 4);
            let after = card(db, 3).await;
            assert_eq!(after.tags, c.tags);
            assert_eq!(after.titles, c.titles);
            assert_eq!(after.completed, c.completed);
            assert_eq!(after.positions, c.positions);
        }
        let t = tag(&left, 5).await;
        run!(
            left,
            admin,
            core::rename,
            rename(26, 5, "Другое", t.version)
        )
        .unwrap();
        assign(&left, &admin, 27, 3, 5, false).await;
        assign(&left, &admin, 28, 3, 5, true).await;
        let after = listing(&left).await;
        let replay = run!(
            left,
            admin,
            tag_names::resolve,
            serde_json::from_slice::<CommandRequest<ResolveTagNames>>(&saved).unwrap()
        )
        .unwrap();
        assert!(replay.replayed);
        assert_eq!(listing(&left).await, after);
        exchange(&left, &right, !reverse).await;
        right.shutdown().await.unwrap();
        let reopened = Storage::open(dir.path().join("right.db")).await.unwrap();
        assert_eq!(listing(&reopened).await, after);
    }
}

#[tokio::test]
async fn bridge_same_different_and_late_decisions_keep_all_names_and_group_blocks() {
    for same in [false, true] {
        for reverse in [false, true] {
            let (_dir, left, right, admin) = bridge().await;
            run!(
                left,
                admin,
                tag_names::resolve,
                choice(&left, 30, ["Личное", "Все", "Всё"]).await
            )
            .unwrap();
            let choices = if same {
                ["Личное", "Все", "Всё"]
            } else {
                ["Дом", "Дела", "Планы"]
            };
            run!(
                right,
                admin,
                tag_names::resolve,
                choice(&right, 31, choices).await
            )
            .unwrap();
            if !same {
                let t = tag(&left, 5).await;
                run!(
                    left,
                    admin,
                    core::rename,
                    rename(33, 5, "Позднее личное", t.version)
                )
                .unwrap();
            }
            exchange(&left, &right, reverse).await;
            assert_eq!(listing(&left).await, listing(&right).await);
            assert_eq!(
                tag(&left, 5).await.state,
                if same {
                    TagState::Active
                } else {
                    TagState::Conflicted
                }
            );
            if !same {
                assert_eq!(tag(&left, 5).await.names, ["Дом", "Позднее личное"]);
                assert_eq!(tag(&left, 6).await.names, ["Все", "Дела"]);
                assert_eq!(tag(&left, 7).await.names, ["Всё", "Планы"]);
                run!(
                    left,
                    admin,
                    tag_names::resolve,
                    choice(&left, 32, ["Итог 1", "Итог 2", "Итог 3"]).await
                )
                .unwrap();
                exchange(&left, &right, !reverse).await;
                assert!(
                    listing(&right)
                        .await
                        .iter()
                        .all(|t| t.state == TagState::Active)
                );
            }
        }
    }
}

#[tokio::test]
async fn both_resolutions_are_atomic_at_every_commit_boundary() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        for names in [false, true] {
            let (_dir, db, _right, admin) = if names {
                bridge().await
            } else {
                super::resolution::conflict().await
            };
            let before = listing(&db).await;
            let card_before = card(&db, 3).await;
            let request = if names {
                serde_json::to_value(choice(&db, 40, ["Личное", "Все", "Всё"]).await).unwrap()
            } else {
                serde_json::to_value(super::resolution::decision(&db, 40, false).await).unwrap()
            };
            let saved = request.clone();
            let owner = admin.clone();
            let result = db
                .transact_checked(
                    move |tx| {
                        if names {
                            tag_names::resolve(
                                tx,
                                &owner.credential(),
                                &Now,
                                &serde_json::from_value(request).unwrap(),
                            )?;
                        } else {
                            zedone_core::tag_assignments::resolve(
                                tx,
                                &owner.credential(),
                                &Now,
                                &serde_json::from_value(request).unwrap(),
                            )?;
                        }
                        Ok(())
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
            if stage != Stage::AfterCommit {
                assert_eq!(listing(&db).await, before);
                assert_eq!(card(&db, 3).await, card_before);
            }
            let owner = admin.clone();
            let replayed = db
                .transact(move |tx| {
                    Ok(if names {
                        tag_names::resolve(
                            tx,
                            &owner.credential(),
                            &Now,
                            &serde_json::from_value(saved).unwrap(),
                        )?
                        .replayed
                    } else {
                        zedone_core::tag_assignments::resolve(
                            tx,
                            &owner.credential(),
                            &Now,
                            &serde_json::from_value(saved).unwrap(),
                        )?
                        .replayed
                    })
                })
                .await
                .unwrap();
            assert_eq!(replayed, stage == Stage::AfterCommit);
        }
    }
}

#[tokio::test]
async fn bridge_resolution_guards_permissions_and_revoked_receipts() {
    let (_dir, db, _right, admin) = bridge().await;
    let reader = Arc::new(PendingAccess::generate().unwrap());
    let writer = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    let r = reader.clone();
    let w = writer.clone();
    db.transact(move |tx| {
        for (user, permission) in [
            (&r, SpacePermission::Read),
            (&w, SpacePermission::ReadWrite),
        ] {
            tx.issue_access(
                &a.credential(),
                &Now,
                user,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), permission)])),
                None,
            )?;
        }
        Ok(())
    })
    .await
    .unwrap();
    let before = listing(&db).await;
    rejected(
        run!(
            db,
            reader,
            tag_names::resolve,
            choice(&db, 60, ["Личное", "Все", "Всё"]).await
        ),
        ErrorCode::AccessDenied,
    );
    assert_eq!(listing(&db).await, before);
    let request = choice(&db, 61, ["Личное", "Все", "Всё"]).await;
    let saved = serde_json::to_vec(&request).unwrap();
    run!(db, writer, tag_names::resolve, request).unwrap();
    let after = listing(&db).await;
    let a = admin.clone();
    let w = writer.clone();
    db.transact(move |tx| {
        tx.revoke_access(&a.credential(), &Now, w.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        run!(
            db,
            writer,
            tag_names::resolve,
            serde_json::from_slice::<CommandRequest<ResolveTagNames>>(&saved).unwrap()
        ),
        ErrorCode::AccessDenied,
    );
    assert_eq!(listing(&db).await, after);
}

#[tokio::test]
async fn one_disputed_bridge_name_blocks_every_member_without_losing_links() {
    let (_dir, left, right, admin) = bridge().await;
    run!(
        left,
        admin,
        tag_names::resolve,
        choice(&left, 70, ["Личное", "Все", "Всё"]).await
    )
    .unwrap();
    run!(
        right,
        admin,
        tag_names::resolve,
        choice(&right, 71, ["Личное", "Дела", "Всё"]).await
    )
    .unwrap();
    exchange(&left, &right, true).await;
    assert!(
        listing(&left)
            .await
            .iter()
            .all(|t| t.state == TagState::Conflicted)
    );
    let c = card(&left, 3).await;
    assert_eq!(c.state, SpaceState::Active);
    for tag_id in [5, 6, 7] {
        let t = tag(&left, tag_id).await;
        rejected(
            run!(
                left,
                admin,
                core::set_card_tag,
                set(72, 3, tag_id, false, c.version, t.version)
            ),
            ErrorCode::Conflict,
        );
        assert_eq!(card(&left, 3).await, c);
    }
}
