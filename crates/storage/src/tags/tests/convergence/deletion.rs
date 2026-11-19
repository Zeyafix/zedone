use super::*;
use zedone_core::tag_deletion::{self, ResolveTagDeletion};

async fn conflict() -> (TempDir, Storage, Storage, Arc<PendingAccess>) {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    run!(right, admin, core::create, create(2, 6, "РАБОТА")).unwrap();
    assign(&left, &admin, 3, 3, 5, true).await;
    exchange(&left, &right, false).await;
    let t = tag(&left, 6).await;
    run!(left, admin, core::delete, delete(4, 6, t.version)).unwrap();
    assert!(card(&left, 3).await.tags.is_empty());
    assign(&right, &admin, 5, 4, 6, true).await;
    exchange(&left, &right, true).await;
    (dir, left, right, admin)
}

async fn decision(db: &Storage, id: u8, keep: bool) -> CommandRequest<ResolveTagDeletion> {
    req(
        id,
        ResolveTagDeletion {
            space: sid(1),
            tag: sid(6),
            keep,
            expected: tag(db, 6).await.version,
        },
    )
}

#[tokio::test]
async fn tag_deletion_both_choices_preserve_old_new_links_other_fields_replay_and_restart() {
    for keep in [false, true] {
        let (dir, left, right, admin) = conflict().await;
        let before = [card(&left, 3).await, card(&left, 4).await];
        for id in [3, 4] {
            let c = card(&left, id).await;
            assert_eq!(c.tags, [sid(5)]);
            assert_eq!(c.state, SpaceState::Conflicted);
            let t = tag(&left, 6).await;
            assert!(t.deletion_conflict);
            rejected(
                run!(
                    left,
                    admin,
                    core::set_card_tag,
                    set(10, id, 5, false, c.version, t.version)
                ),
                ErrorCode::Conflict,
            );
        }
        let request = decision(&left, 11, keep).await;
        let saved = serde_json::to_vec(&request).unwrap();
        let stale = decision(&left, 12, !keep).await;
        run!(left, admin, tag_deletion::resolve, request).unwrap();
        rejected(
            run!(left, admin, tag_deletion::resolve, stale),
            ErrorCode::StaleState,
        );
        exchange(&left, &right, keep).await;
        for db in [&left, &right] {
            let t = tag(db, 5).await;
            assert_eq!(
                t.state,
                if keep {
                    TagState::Active
                } else {
                    TagState::Deleted
                }
            );
            assert!(!t.deletion_conflict);
            for id in [3, 4] {
                let c = card(db, id).await;
                let original = &before[usize::from(id - 3)];
                assert_eq!(c.state, SpaceState::Active);
                assert_eq!(c.tags, if keep { vec![sid(5)] } else { vec![] });
                assert_eq!(c.titles, original.titles);
                assert_eq!(c.descriptions, original.descriptions);
                assert_eq!(c.positions, original.positions);
                assert_eq!(c.completed, original.completed);
            }
        }
        if keep {
            assign(&left, &admin, 13, 3, 5, false).await;
            assign(&left, &admin, 14, 3, 6, true).await;
            let t = tag(&left, 5).await;
            run!(left, admin, core::delete, delete(15, 5, t.version)).unwrap();
            assert_eq!(tag(&left, 5).await.state, TagState::Deleted);
        }
        let after = listing(&left).await;
        assert!(
            run!(
                left,
                admin,
                tag_deletion::resolve,
                serde_json::from_slice::<CommandRequest<ResolveTagDeletion>>(&saved).unwrap()
            )
            .unwrap()
            .replayed
        );
        assert_eq!(listing(&left).await, after);
        run!(left, admin, core::create, create(16, 7, "Работа")).unwrap();
        assert!(card(&left, 3).await.tags.is_empty());
        exchange(&left, &right, !keep).await;
        right.shutdown().await.unwrap();
        let reopened = Storage::open(dir.path().join("right.db")).await.unwrap();
        assert_eq!(listing(&left).await, listing(&reopened).await);
        assert_eq!(card(&left, 4).await, card(&reopened, 4).await);
    }
}

#[tokio::test]
async fn tag_deletion_equal_different_and_late_decisions_preserve_conflicting_variants() {
    for a in [false, true] {
        for b in [false, true] {
            let (_dir, left, right, admin) = conflict().await;
            run!(
                left,
                admin,
                tag_deletion::resolve,
                decision(&left, 20, a).await
            )
            .unwrap();
            run!(
                right,
                admin,
                tag_deletion::resolve,
                decision(&right, 21, b).await
            )
            .unwrap();
            if a {
                let c = card(&left, 3).await;
                run!(
                    left,
                    admin,
                    cards::edit,
                    req(
                        22,
                        EditCard {
                            space: sid(1),
                            card: sid(3),
                            title: "Поздняя правка".into(),
                            description: "Сохранить".into(),
                            expected: c.version
                        }
                    )
                )
                .unwrap();
            }
            exchange(&left, &right, a).await;
            assert_eq!(listing(&left).await, listing(&right).await);
            assert_eq!(tag(&left, 5).await.deletion_conflict, a != b);
            if a != b {
                for id in [3, 4] {
                    assert_eq!(card(&left, id).await.state, SpaceState::Conflicted);
                }
                run!(
                    left,
                    admin,
                    tag_deletion::resolve,
                    decision(&left, 23, true).await
                )
                .unwrap();
                exchange(&left, &right, !a).await;
                assert_eq!(tag(&left, 5).await.state, TagState::Active);
            }
            if a {
                assert_eq!(card(&right, 3).await.titles, ["Поздняя правка"]);
            }
        }
    }
}

#[tokio::test]
async fn tag_deletion_decisions_roll_back_or_replay_at_every_commit_boundary() {
    for keep in [false, true] {
        for stage in [
            Stage::AfterWork,
            Stage::AfterChanges,
            Stage::AfterResult,
            Stage::BeforeCommit,
            Stage::AfterCommit,
        ] {
            let (_dir, db, _right, admin) = conflict().await;
            let before = listing(&db).await;
            let c = card(&db, 3).await;
            let request = decision(&db, 30, keep).await;
            let saved = serde_json::to_vec(&request).unwrap();
            let owner = admin.clone();
            let result = db
                .transact_checked(
                    move |tx| {
                        Ok(tag_deletion::resolve(
                            tx,
                            &owner.credential(),
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
                assert_eq!(card(&db, 3).await, c);
            }
            assert_eq!(
                run!(
                    db,
                    admin,
                    tag_deletion::resolve,
                    serde_json::from_slice::<CommandRequest<ResolveTagDeletion>>(&saved).unwrap()
                )
                .unwrap()
                .replayed,
                stage == Stage::AfterCommit
            );
        }
    }
}

#[tokio::test]
async fn tag_deletion_keeps_unrelated_conflicts_and_checks_permissions_before_replay() {
    let (_dir, left, right, admin) = conflict().await;
    for (db, title, command) in [(&left, "Left", 40), (&right, "Right", 41)] {
        let owner = admin.clone();
        db.transact(move |tx| {
            let c = tx.card_snapshot(&sid(1), &sid(3))?;
            tx.edit_card(
                &EditCard {
                    space: sid(1),
                    card: sid(3),
                    title: title.into(),
                    description: "Body".into(),
                    expected: c.version,
                },
                owner.subject(),
                &sid(command),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    }
    exchange(&left, &right, false).await;
    let before = card(&left, 3).await;
    let reader = Arc::new(PendingAccess::generate().unwrap());
    let writer = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    let r = reader.clone();
    let w = writer.clone();
    left.transact(move |tx| {
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
    rejected(
        run!(
            left,
            reader,
            tag_deletion::resolve,
            decision(&left, 42, true).await
        ),
        ErrorCode::AccessDenied,
    );
    let request = decision(&left, 43, true).await;
    let saved = serde_json::to_vec(&request).unwrap();
    run!(left, writer, tag_deletion::resolve, request).unwrap();
    let after = card(&left, 3).await;
    assert_eq!(after.state, SpaceState::Conflicted);
    assert_eq!(after.titles, before.titles);
    assert_eq!(after.tags, before.tags);
    let a = admin.clone();
    let w = writer.clone();
    left.transact(move |tx| {
        tx.revoke_access(&a.credential(), &Now, w.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        run!(
            left,
            writer,
            tag_deletion::resolve,
            serde_json::from_slice::<CommandRequest<ResolveTagDeletion>>(&saved).unwrap()
        ),
        ErrorCode::AccessDenied,
    );
    assert_eq!(card(&left, 3).await, after);
}
