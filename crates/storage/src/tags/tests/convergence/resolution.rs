use super::*;
use zedone_core::tag_assignments::{self, InspectCardTags, ResolveCardTag};

pub(super) async fn conflict() -> (TempDir, Storage, Storage, Arc<PendingAccess>) {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    run!(right, admin, core::create, create(2, 6, "РАБОТА")).unwrap();
    assign(&left, &admin, 3, 3, 5, true).await;
    exchange(&left, &right, false).await;
    assign(&left, &admin, 4, 3, 5, false).await;
    assign(&right, &admin, 5, 3, 6, false).await;
    assign(&right, &admin, 6, 3, 6, true).await;
    exchange(&left, &right, true).await;
    (dir, left, right, admin)
}
pub(super) async fn decision(
    db: &Storage,
    id: u8,
    present: bool,
) -> CommandRequest<ResolveCardTag> {
    req(
        id,
        ResolveCardTag {
            space: sid(1),
            card: sid(3),
            tag: sid(6),
            present,
            expected: card(db, 3).await.version,
            expected_tag: tag(db, 6).await.version,
        },
    )
}

#[tokio::test]
async fn both_assignment_decisions_preserve_other_data_replay_reverse_and_restart() {
    for present in [false, true] {
        let (dir, left, right, admin) = conflict().await;
        let before = card(&left, 3).await;
        let inspected = run!(
            left,
            admin,
            tag_assignments::inspect,
            req(
                10,
                InspectCardTags {
                    space: sid(1),
                    card: sid(3)
                }
            )
        )
        .unwrap()
        .result;
        assert_eq!(inspected.card, before);
        assert_eq!(inspected.assignments[0].values, [false, true]);
        let stale = decision(&left, 12, !present).await;
        let request = decision(&left, 11, present).await;
        let saved = serde_json::to_vec(&request).unwrap();
        let receipt = run!(left, admin, tag_assignments::resolve, request)
            .unwrap()
            .result;
        assert_eq!(receipt.state, SpaceState::Active);
        assert_eq!(receipt.tags.contains(&sid(5)), present);
        for (a, b) in [
            (&receipt.titles, &before.titles),
            (&receipt.descriptions, &before.descriptions),
            (&receipt.columns, &before.columns),
        ] {
            assert_eq!(a, b);
        }
        assert_eq!(receipt.completed, before.completed);
        assert_eq!(receipt.positions, before.positions);
        rejected(
            run!(left, admin, tag_assignments::resolve, stale),
            ErrorCode::StaleState,
        );
        assign(&left, &admin, 13, 3, 6, !present).await;
        let after = card(&left, 3).await;
        let replay = run!(
            left,
            admin,
            tag_assignments::resolve,
            serde_json::from_slice::<CommandRequest<ResolveCardTag>>(&saved).unwrap()
        )
        .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.result, receipt);
        assert_eq!(card(&left, 3).await, after);
        exchange(&left, &right, true).await;
        assert_eq!(card(&left, 3).await, card(&right, 3).await);
        right.shutdown().await.unwrap();
        let reopened = Storage::open(dir.path().join("right.db")).await.unwrap();
        assert_eq!(card(&reopened, 3).await, after);
        rejected(
            run!(
                left,
                admin,
                tag_assignments::resolve,
                decision(&left, 14, present).await
            ),
            ErrorCode::Conflict,
        );
    }
}

#[tokio::test]
async fn independent_same_and_different_assignment_decisions_converge() {
    for (a, b) in [(false, false), (true, true), (false, true), (true, false)] {
        for reverse in [false, true] {
            let (_dir, left, right, admin) = conflict().await;
            run!(
                left,
                admin,
                tag_assignments::resolve,
                decision(&left, 20, a).await
            )
            .unwrap();
            run!(
                right,
                admin,
                tag_assignments::resolve,
                decision(&right, 21, b).await
            )
            .unwrap();
            exchange(&left, &right, reverse).await;
            let card = card(&left, 3).await;
            assert_eq!(card, super::card(&right, 3).await);
            assert_eq!(
                card.state,
                if a == b {
                    SpaceState::Active
                } else {
                    SpaceState::Conflicted
                },
                "left={a}, right={b}, reverse={reverse}"
            );
            if a != b {
                run!(
                    right,
                    admin,
                    tag_assignments::resolve,
                    decision(&right, 22, false).await
                )
                .unwrap();
                exchange(&left, &right, !reverse).await;
                assert!(super::card(&left, 3).await.tags.is_empty());
            }
        }
    }
}

#[tokio::test]
async fn assignment_resolution_keeps_unrelated_text_conflict_and_checks_rights_on_replay() {
    let (_dir, left, right, admin) = conflict().await;
    for (db, title, command) in [(&left, "Left", 50), (&right, "Right", 51)] {
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
    exchange(&left, &right, true).await;
    let before = card(&left, 3).await;
    assert_eq!(before.titles, ["Left", "Right"]);
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
    run!(
        left,
        reader,
        tag_assignments::inspect,
        req(
            52,
            InspectCardTags {
                space: sid(1),
                card: sid(3)
            }
        )
    )
    .unwrap();
    rejected(
        run!(
            left,
            reader,
            tag_assignments::resolve,
            decision(&left, 53, false).await
        ),
        ErrorCode::AccessDenied,
    );
    let request = decision(&left, 54, false).await;
    let saved = serde_json::to_vec(&request).unwrap();
    let after = run!(left, writer, tag_assignments::resolve, request)
        .unwrap()
        .result;
    assert_eq!(after.state, SpaceState::Conflicted);
    assert_eq!(after.titles, before.titles);
    assert!(after.tags.is_empty());
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
            tag_assignments::resolve,
            serde_json::from_slice::<CommandRequest<ResolveCardTag>>(&saved).unwrap()
        ),
        ErrorCode::AccessDenied,
    );
    assert_eq!(card(&left, 3).await, after);
}
