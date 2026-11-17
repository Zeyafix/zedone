use super::text_conflicts::{copy, exchange};
use super::*;
use zedone_core::{
    ordering::{self, MoveCard},
    placements::{self, InspectCardPlacements, ResolveCardPlacement},
};

async fn move_to(
    db: &Storage,
    admin: &PendingAccess,
    id: u8,
    card: u8,
    column: u8,
    before: Option<u8>,
) -> CardSnapshot {
    let current = db
        .transact(move |tx| Ok(tx.card_snapshot(&sid(1), &sid(card))?))
        .await
        .unwrap();
    let secret = admin.credential().0.to_vec();
    db.transact(move |tx| {
        Ok(ordering::move_card(
            tx,
            &Credential(&secret),
            &Now,
            &req(
                id,
                MoveCard {
                    space: sid(1),
                    card: sid(card),
                    column: sid(column),
                    before: before.map(sid),
                    expected: current.version,
                },
            ),
        )?)
    })
    .await
    .unwrap()
    .result
}
async fn inspect(db: &Storage, admin: &PendingAccess) -> placements::CardPlacements {
    let secret = admin.credential().0.to_vec();
    let mut request = req(
        40,
        InspectCardPlacements {
            space: sid(1),
            card: sid(9),
        },
    );
    getrandom::fill(&mut request.id).unwrap();
    db.transact(move |tx| {
        Ok(placements::inspect(
            tx,
            &Credential(&secret),
            &Now,
            &request,
        )?)
    })
    .await
    .unwrap()
    .result
}
async fn choose(
    db: &Storage,
    admin: &PendingAccess,
    id: u8,
    current: &CardSnapshot,
    index: usize,
) -> StorageResult<zedone_core::replay::Execution<CardSnapshot>> {
    let secret = admin.credential().0.to_vec();
    let request = req(
        id,
        ResolveCardPlacement {
            space: sid(1),
            card: sid(9),
            expected: current.version,
            placement: current.placements[index].clone(),
        },
    );
    db.transact(move |tx| {
        Ok(placements::resolve(
            tx,
            &Credential(&secret),
            &Now,
            &request,
        )?)
    })
    .await
}
async fn conflict(same: bool) -> (TempDir, Storage, Storage, Arc<PendingAccess>) {
    let (dir, left, admin) = setup().await;
    for (id, card) in [(1, 7), (2, 8), (3, 9)] {
        run!(&left, admin, create, creation(id, card)).unwrap();
    }
    let right = copy(&left, dir.path().join("peer.db"), &admin).await;
    move_to(&left, &admin, 4, 9, 2, Some(8)).await;
    move_to(
        &right,
        &admin,
        5,
        9,
        if same { 2 } else { 3 },
        if same { Some(7) } else { None },
    )
    .await;
    exchange(&left, &right).await;
    (dir, left, right, admin)
}

#[tokio::test]
async fn saved_whole_places_resolve_both_kinds_of_move_and_independent_decisions_reconflict() {
    for same in [false, true] {
        let (dir, left, right, admin) = conflict(same).await;
        let current = inspect(&left, &admin).await;
        assert_eq!(inspect(&right, &admin).await, current);
        assert_eq!(current.card.state, SpaceState::Conflicted);
        assert_eq!(current.choices.len(), 2);
        for choice in &current.choices {
            assert!(current.card.placements.contains(&choice.placement));
            let expected = if choice.placement.column == sid(3) {
                None
            } else if choice.placement == current.card.placements[0] && same {
                Some(sid(7))
            } else {
                Some(sid(8))
            };
            assert_eq!(choice.before, expected);
        }
        let history = versions(&left, 9).await;
        rejected(
            run!(
                &left,
                admin,
                edit,
                edit_req(6, 9, "Blocked", "", current.card.version)
            ),
            ErrorCode::Conflict,
        );
        let first = choose(&left, &admin, 7, &current.card, 0).await.unwrap();
        let second = choose(&right, &admin, 8, &current.card, 1).await.unwrap();
        assert_eq!(first.result.state, SpaceState::Active);
        assert_eq!(second.result.state, SpaceState::Active);
        exchange(&right, &left).await;
        let again = inspect(&left, &admin).await;
        assert_eq!(again.choices.len(), 2);
        assert_eq!(inspect(&right, &admin).await, again);
        assert_eq!(again.card.placements, current.card.placements);
        rejected(
            choose(&left, &admin, 9, &current.card, 0).await,
            ErrorCode::StaleState,
        );
        let chosen = choose(&right, &admin, 10, &again.card, 1)
            .await
            .unwrap()
            .result;
        exchange(&left, &right).await;
        assert_eq!(fresh(&left, &admin, 11, 9).await, chosen);
        assert_eq!(chosen.titles, current.card.titles);
        assert_eq!(chosen.descriptions, current.card.descriptions);
        assert_eq!(chosen.completed, current.card.completed);
        assert_eq!(chosen.tags, current.card.tags);
        assert_eq!(versions(&left, 9).await, history);
        move_to(&left, &admin, 12, 9, 2, Some(7)).await;
        let back = fresh(&left, &admin, 13, 9).await;
        let replay = choose(&right, &admin, 10, &again.card, 1).await.unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.result, chosen);
        exchange(&left, &right).await;
        assert_eq!(fresh(&right, &admin, 14, 9).await, back);
        left.shutdown().await.unwrap();
        right.shutdown().await.unwrap();
        let reopened = Storage::open(dir.path().join("cards.db")).await.unwrap();
        assert_eq!(fresh(&reopened, &admin, 15, 9).await, back);
        assert_eq!(versions(&reopened, 9).await, history);
    }
}

#[tokio::test]
async fn placement_choice_retains_other_conflicts_and_rolls_back_graph_and_receipt_on_failure() {
    let (_dir, left, _right, admin) = conflict(false).await;
    left.transact(|tx| {
        let graph = tx.read().document(&sid(1))?;
        let obj = object(graph, &sid(9))?;
        let mut changes = Vec::new();
        for (actor, text) in [(61, "One"), (62, "Two")] {
            let mut branch = graph.clone().with_actor(ActorId::from(vec![actor]));
            let mut transaction = branch.transaction();
            transaction.put(&obj, "title", text)?;
            transaction.commit();
            changes.push(branch.get_last_local_change().unwrap());
        }
        tx.apply_changes(&sid(1), changes)
    })
    .await
    .unwrap();
    let current = fresh(&left, &admin, 20, 9).await;
    left.transact(|tx| { tx.connection().execute_batch("CREATE TRIGGER fail_place BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?; Ok(()) }).await.unwrap();
    rejected(
        choose(&left, &admin, 21, &current, 0).await,
        ErrorCode::Unavailable,
    );
    assert_eq!(
        left.transact(|tx| Ok(tx.card_snapshot(&sid(1), &sid(9))?))
            .await
            .unwrap(),
        current
    );
    left.transact(|tx| {
        tx.connection().execute_batch("DROP TRIGGER fail_place")?;
        Ok(())
    })
    .await
    .unwrap();
    let result = choose(&left, &admin, 21, &current, 0).await.unwrap();
    assert!(!result.replayed);
    assert_eq!(result.result.placements.len(), 1);
    assert_eq!(result.result.titles, current.titles);
    assert_eq!(result.result.state, SpaceState::Conflicted);
    rejected(
        run!(
            &left,
            admin,
            edit,
            edit_req(23, 9, "Blocked", "", result.result.version)
        ),
        ErrorCode::Conflict,
    );
}

#[tokio::test]
async fn independent_cards_between_the_same_neighbors_converge_without_manual_conflicts() {
    let (dir, left, admin) = setup().await;
    for (id, card) in [(1, 7), (2, 8), (3, 9), (4, 10)] {
        run!(&left, admin, create, creation(id, card)).unwrap();
    }
    let right = copy(&left, dir.path().join("peer.db"), &admin).await;
    move_to(&left, &admin, 5, 9, 2, Some(8)).await;
    move_to(&right, &admin, 6, 10, 2, Some(8)).await;
    exchange(&right, &left).await;
    for db in [&left, &right] {
        let cards = db.transact(|tx| Ok(tx.cards(&sid(1))?)).await.unwrap();
        let mut cards = cards;
        cards.sort_by_key(|c| c.positions[0].key(&c.card));
        assert_eq!(
            cards.iter().map(|c| c.card.clone()).collect::<Vec<_>>(),
            [sid(7), sid(9), sid(10), sid(8)]
        );
        assert!(cards.iter().all(|c| c.state == SpaceState::Active));
    }
    move_to(&right, &admin, 7, 7, 2, Some(10)).await;
    exchange(&left, &right).await;
    assert_eq!(
        fresh(&left, &admin, 8, 7).await,
        fresh(&right, &admin, 8, 7).await
    );
}

#[tokio::test]
async fn identical_independent_saved_choices_converge_to_one_active_place() {
    for same in [false, true] {
        let (_dir, left, right, admin) = conflict(same).await;
        let current = inspect(&left, &admin).await.card;
        choose(&left, &admin, 20, &current, 0).await.unwrap();
        choose(&right, &admin, 21, &current, 0).await.unwrap();
        exchange(&right, &left).await;
        let merged = inspect(&left, &admin).await;
        assert_eq!(merged.choices.len(), 1);
        assert_eq!(merged.card.state, SpaceState::Active);
        assert_eq!(inspect(&right, &admin).await, merged);
    }
}
