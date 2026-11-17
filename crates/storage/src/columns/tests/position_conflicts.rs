use super::*;
use zedone_core::{
    cards::{self, CardStore, CreateCard, SetCardCompleted},
    column_positions::{self, InspectColumnPositions, ResolveColumnPosition},
    ordering::{self, MoveColumn},
    sync::SyncStore,
};

async fn copy(db: &Storage, path: std::path::PathBuf, admin: &Arc<PendingAccess>) -> Storage {
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
async fn exchange(a: &Storage, b: &Storage) {
    let mut pages = Vec::new();
    for db in [a, b] {
        pages.push(
            db.transact(|tx| {
                crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
            })
            .await
            .unwrap(),
        );
    }
    for (db, page) in [(a, pages.remove(1)), (b, pages.remove(0))] {
        db.transact(move |tx| {
            tx.push_sync(&sid(1), "test", &vec![], &page.changes)?;
            tx.push_sync(&sid(1), "test", &vec![], &page.changes)?;
            Ok(())
        })
        .await
        .unwrap();
    }
}
async fn move_to(
    db: &Storage,
    admin: &PendingAccess,
    id: u8,
    column: u8,
    before: Option<u8>,
) -> StorageResult<zedone_core::replay::Execution<ColumnSnapshot>> {
    let current = db
        .transact(move |tx| Ok(tx.column_snapshot(&sid(1), &sid(column))?))
        .await
        .unwrap();
    let secret = admin.credential().0.to_vec();
    db.transact(move |tx| {
        Ok(ordering::move_column(
            tx,
            &Credential(&secret),
            &Now,
            &req(
                id,
                MoveColumn {
                    space: sid(1),
                    column: sid(column),
                    before: before.map(sid),
                    expected: current.version,
                },
            ),
        )?)
    })
    .await
}
async fn inspect(db: &Storage, admin: &PendingAccess) -> column_positions::ColumnPositions {
    let secret = admin.credential().0.to_vec();
    let mut request = req(
        201,
        InspectColumnPositions {
            space: sid(1),
            column: sid(9),
        },
    );
    getrandom::fill(&mut request.id).unwrap();
    db.transact(move |tx| {
        Ok(column_positions::inspect(
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
    current: &ColumnSnapshot,
    index: usize,
) -> StorageResult<zedone_core::replay::Execution<ColumnSnapshot>> {
    let secret = admin.credential().0.to_vec();
    let request = req(
        id,
        ResolveColumnPosition {
            space: sid(1),
            column: sid(9),
            position: current.positions[index].clone(),
            expected: current.version,
        },
    );
    db.transact(move |tx| {
        Ok(column_positions::resolve(
            tx,
            &Credential(&secret),
            &Now,
            &request,
        )?)
    })
    .await
}
async fn conflict() -> (TempDir, Storage, Storage, Arc<PendingAccess>) {
    let (dir, left, admin) = setup().await;
    for (id, column) in [(1, 7), (2, 8), (3, 9), (4, 10), (5, 11)] {
        run!(&left, admin, create, creation(id, column)).unwrap();
    }
    let owner = admin.clone();
    left.transact(move |tx| {
        cards::create(
            tx,
            &owner.credential(),
            &Now,
            &req(
                6,
                CreateCard {
                    space: sid(1),
                    column: sid(9),
                    card: sid(20),
                    title: "Saved".into(),
                    description: "Body".into(),
                },
            ),
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    move_to(&left, &admin, 7, 9, Some(8)).await.unwrap();
    move_to(&right, &admin, 8, 9, None).await.unwrap();
    exchange(&left, &right).await;
    (dir, left, right, admin)
}

#[tokio::test]
async fn saved_positions_keep_children_available_and_independent_decisions_reconflict() {
    for (first_index, second_index) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
        let (dir, left, right, admin) = conflict().await;
        let current = inspect(&left, &admin).await;
        assert_eq!(inspect(&right, &admin).await, current);
        assert_eq!(current.choices.len(), 2);
        assert_eq!(current.choices[0].before, Some(sid(8)));
        assert_eq!(current.choices[1].before, None);
        rejected(
            move_to(&left, &admin, 9, 9, Some(7)).await,
            ErrorCode::Conflict,
        );
        let before = left
            .transact(|tx| Ok(tx.card_snapshot(&sid(1), &sid(20))?))
            .await
            .unwrap();
        let owner = admin.clone();
        let child = left
            .transact(move |tx| {
                Ok(cards::set_completed(
                    tx,
                    &owner.credential(),
                    &Now,
                    &req(
                        10,
                        SetCardCompleted {
                            space: sid(1),
                            card: sid(20),
                            completed: true,
                            expected: before.version,
                        },
                    ),
                )?)
            })
            .await
            .unwrap()
            .result;
        rejected(
            choose(&left, &admin, 11, &current.column, 0).await,
            ErrorCode::StaleState,
        );
        exchange(&right, &left).await;
        let current = inspect(&left, &admin).await;
        let first = choose(&left, &admin, 12, &current.column, first_index)
            .await
            .unwrap()
            .result;
        let second = choose(&right, &admin, 13, &current.column, second_index)
            .await
            .unwrap()
            .result;
        assert_eq!(first.positions.len(), 1);
        assert_eq!(second.positions.len(), 1);
        exchange(&right, &left).await;
        let merged = inspect(&left, &admin).await;
        assert_eq!(inspect(&right, &admin).await, merged);
        assert_eq!(
            merged.choices.len(),
            if first_index == second_index { 1 } else { 2 }
        );
        let chosen = if merged.choices.len() == 2 {
            choose(&left, &admin, 14, &merged.column, 0)
                .await
                .unwrap()
                .result
        } else {
            merged.column
        };
        assert_eq!(chosen.names, current.column.names);
        assert_eq!(chosen.contents, current.column.contents);
        assert_eq!(
            left.transact(|tx| Ok(tx.card_snapshot(&sid(1), &sid(20))?))
                .await
                .unwrap(),
            child
        );
        let back = move_to(&left, &admin, 15, 9, Some(7)).await.unwrap().result;
        let replay = choose(&right, &admin, 13, &current.column, second_index)
            .await
            .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.result, second);
        exchange(&left, &right).await;
        assert_eq!(inspect(&right, &admin).await.column, back);
        left.shutdown().await.unwrap();
        right.shutdown().await.unwrap();
        let reopened = Storage::open(dir.path().join("columns.db")).await.unwrap();
        assert_eq!(inspect(&reopened, &admin).await.column, back);
    }
}

#[tokio::test]
async fn position_resolution_rolls_back_and_preserves_name_conflicts() {
    let (_dir, left, _right, admin) = conflict().await;
    left.transact(|tx| {
        let graph = tx.read().document(&sid(1))?;
        let obj = object(graph, &sid(9))?;
        let mut changes = Vec::new();
        for (actor, name) in [(61, "One"), (62, "Two")] {
            let mut branch = graph.clone().with_actor(ActorId::from(vec![actor]));
            let mut transaction = branch.transaction();
            transaction.put(&obj, "name", name)?;
            transaction.commit();
            changes.push(branch.get_last_local_change().unwrap());
        }
        tx.apply_changes(&sid(1), changes)
    })
    .await
    .unwrap();
    let current = inspect(&left, &admin).await.column;
    assert_eq!(current.names.len(), 2);
    left.transact(|tx| { tx.connection().execute_batch("CREATE TRIGGER fail_position BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?; Ok(()) }).await.unwrap();
    rejected(
        choose(&left, &admin, 20, &current, 0).await,
        ErrorCode::Unavailable,
    );
    assert_eq!(
        left.transact(|tx| Ok(tx.column_snapshot(&sid(1), &sid(9))?))
            .await
            .unwrap(),
        current
    );
    left.transact(|tx| {
        tx.connection()
            .execute_batch("DROP TRIGGER fail_position")?;
        Ok(())
    })
    .await
    .unwrap();
    let chosen = choose(&left, &admin, 20, &current, 0).await.unwrap();
    assert!(!chosen.replayed);
    assert_eq!(chosen.result.positions.len(), 1);
    assert_eq!(chosen.result.names, current.names);
    assert_eq!(chosen.result.contents, current.contents);
    rejected(
        move_to(&left, &admin, 21, 9, None).await,
        ErrorCode::Conflict,
    );
}

#[tokio::test]
async fn different_columns_between_the_same_neighbors_converge_in_both_delivery_orders() {
    let (dir, left, admin) = setup().await;
    for (id, column) in [(1, 7), (2, 8), (3, 10), (4, 11)] {
        run!(&left, admin, create, creation(id, column)).unwrap();
    }
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    move_to(&left, &admin, 5, 10, Some(8)).await.unwrap();
    move_to(&right, &admin, 6, 11, Some(8)).await.unwrap();
    exchange(&right, &left).await;
    for db in [&left, &right] {
        let columns = db.transact(|tx| Ok(tx.columns(&sid(1))?)).await.unwrap();
        let mut columns = columns;
        columns.sort_by_key(|c| c.positions[0].key(&c.column));
        assert_eq!(
            columns.iter().map(|c| c.column.clone()).collect::<Vec<_>>(),
            [sid(7), sid(10), sid(11), sid(8)]
        );
        assert!(columns.iter().all(|c| c.positions.len() == 1));
    }
    exchange(&left, &right).await;
    assert_eq!(
        fresh(&left, &admin, 199, 10).await,
        fresh(&right, &admin, 199, 10).await
    );
}

#[tokio::test]
async fn position_resolution_never_restores_deleted_containers_or_clears_lifecycle_conflicts() {
    for (target, values) in [
        (
            9,
            vec!["trash:000000000000000000000000000000a0:000000000000000000000000000000a1"],
        ),
        (
            9,
            vec![
                "active",
                "trash:000000000000000000000000000000a0:000000000000000000000000000000a1",
            ],
        ),
        (
            1,
            vec!["trash:000000000000000000000000000000a0:000000000000000000000000000000a1"],
        ),
        (
            1,
            vec![
                "active",
                "trash:000000000000000000000000000000a0:000000000000000000000000000000a1",
            ],
        ),
    ] {
        let (_dir, left, _right, admin) = conflict().await;
        left.transact(move |tx| {
            let graph = tx.read().document(&sid(1))?;
            let obj = if target == 9 {
                object(graph, &sid(9))?
            } else {
                automerge::ROOT
            };
            let mut changes = Vec::new();
            for (actor, value) in values.into_iter().enumerate() {
                let mut branch = graph
                    .clone()
                    .with_actor(ActorId::from(vec![70 + actor as u8]));
                let mut transaction = branch.transaction();
                transaction.delete(&obj, "lifecycle")?;
                transaction.put(&obj, "lifecycle", value)?;
                transaction.commit();
                changes.push(branch.get_last_local_change().unwrap());
            }
            tx.apply_changes(&sid(1), changes)
        })
        .await
        .unwrap();
        let current = inspect(&left, &admin).await.column;
        let result = choose(&left, &admin, 30, &current, 0).await;
        assert!(matches!(
            result,
            Err(StorageError::Rejected(CommandError {
                code: ErrorCode::Conflict | ErrorCode::InvalidRequest
            }))
        ));
        assert_eq!(inspect(&left, &admin).await.column, current);
    }
}

#[tokio::test]
async fn position_labels_skip_disputed_neighbors_and_neighbor_moves_do_not_rewrite_the_choice() {
    let (_dir, left, _right, admin) = conflict().await;
    left.transact(|tx| {
        let graph = tx.read().document(&sid(1))?;
        let disputed = object(graph, &sid(8))?;
        let named = object(graph, &sid(10))?;
        let mut changes = Vec::new();
        for (actor, name) in [(80, "One"), (81, "Two")] {
            let mut branch = graph.clone().with_actor(ActorId::from(vec![actor]));
            let mut transaction = branch.transaction();
            transaction.put(&disputed, "position", u64::from(actor))?;
            transaction.put(&named, "name", name)?;
            transaction.commit();
            changes.push(branch.get_last_local_change().unwrap());
        }
        tx.apply_changes(&sid(1), changes)
    })
    .await
    .unwrap();
    let current = inspect(&left, &admin).await;
    assert_eq!(current.choices[0].before, Some(sid(10)));
    let chosen_position = current.choices[0].position.clone();
    let owner = admin.clone();
    left.transact(move |tx| {
        let snapshot = tx.column_snapshot(&sid(1), &sid(10))?;
        core::resolve_name(
            tx,
            &owner.credential(),
            &Now,
            &req(
                40,
                ResolveColumnName {
                    space: sid(1),
                    column: sid(10),
                    name: "Resolved".into(),
                    expected: snapshot.version,
                },
            ),
        )?;
        Ok(())
    })
    .await
    .unwrap();
    move_to(&left, &admin, 41, 10, None).await.unwrap();
    assert_eq!(
        inspect(&left, &admin).await.column.version,
        current.column.version
    );
    let chosen = choose(&left, &admin, 42, &current.column, 0)
        .await
        .unwrap()
        .result;
    assert_eq!(chosen.positions, [chosen_position]);
}
