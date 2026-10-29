use super::*;

fn seeded() -> (tempfile::TempDir, Connection, Snapshot) {
    let directory = tempfile::tempdir().unwrap();
    let mut db = connect(directory.path().join("synthetic.sqlite3")).unwrap();
    for space in [Space::A, Space::B] {
        run(&mut db, StorageRequest::Seed { space }).unwrap();
    }
    (directory, db, synthetic(Space::A))
}

#[test]
fn every_card_move_and_inverse_preserves_identity_content_and_other_space() {
    let (_directory, mut db, initial) = seeded();
    for card in &initial.cards {
        let old_position = initial
            .cards
            .iter()
            .filter(|item| item.column == card.column)
            .position(|item| item.id == card.id)
            .unwrap() as u32;
        for column in 0..3 {
            let count = initial
                .cards
                .iter()
                .filter(|item| item.column == column && item.id != card.id)
                .count() as u32;
            for position in [0, count / 2, count] {
                let next = run(
                    &mut db,
                    StorageRequest::MoveCard {
                        expected: initial.clone(),
                        card: card.id,
                        column,
                        position,
                    },
                )
                .unwrap()
                .snapshot
                .unwrap();
                let mut content = next.cards.clone();
                for item in &mut content {
                    item.column = (item.id - 1) / 10;
                }
                content.sort_by_key(|item| item.id);
                assert_eq!(content, initial.cards);
                assert_eq!(
                    next.cards
                        .iter()
                        .filter(|item| item.column == column)
                        .nth(position as usize)
                        .unwrap()
                        .id,
                    card.id
                );
                let restored = run(
                    &mut db,
                    StorageRequest::MoveCard {
                        expected: next,
                        card: card.id,
                        column: card.column,
                        position: old_position,
                    },
                )
                .unwrap()
                .snapshot
                .unwrap();
                assert_eq!(restored, initial);
            }
        }
    }
    assert_eq!(load(&db, Space::B).unwrap(), Some(synthetic(Space::B)));
}

#[test]
fn columns_keep_their_cards_and_moves_survive_reopen() {
    let (directory, mut db, initial) = seeded();
    for column in 0..3 {
        for position in 0..3 {
            let next = run(
                &mut db,
                StorageRequest::MoveColumn {
                    expected: initial.clone(),
                    column,
                    position,
                },
            )
            .unwrap()
            .snapshot
            .unwrap();
            assert_eq!(
                next.columns[position as usize],
                initial.columns[column as usize]
            );
            assert_eq!(
                next.cards
                    .iter()
                    .filter(|item| item.column == position)
                    .map(|item| item.id)
                    .collect::<Vec<_>>(),
                initial
                    .cards
                    .iter()
                    .filter(|item| item.column == column)
                    .map(|item| item.id)
                    .collect::<Vec<_>>()
            );
            let reopened = connect(directory.path().join("synthetic.sqlite3")).unwrap();
            assert_eq!(load(&reopened, Space::A).unwrap(), Some(next.clone()));
            let restored = run(
                &mut db,
                StorageRequest::MoveColumn {
                    expected: next,
                    column: position,
                    position: column,
                },
            )
            .unwrap()
            .snapshot
            .unwrap();
            assert_eq!(restored, initial);
        }
    }
}

#[test]
fn invalid_stale_repeated_and_failed_moves_cannot_change_saved_data() {
    let (_directory, mut db, initial) = seeded();
    for (card, column, position) in [(999, 0, 0), (1, 99, 0), (1, 0, 10), (1, 1, 11)] {
        assert!(
            run(
                &mut db,
                StorageRequest::MoveCard {
                    expected: initial.clone(),
                    card,
                    column,
                    position
                }
            )
            .is_err()
        );
        assert_eq!(load(&db, Space::A).unwrap(), Some(initial.clone()));
    }
    for (column, position) in [(3, 0), (0, 3)] {
        assert!(
            run(
                &mut db,
                StorageRequest::MoveColumn {
                    expected: initial.clone(),
                    column,
                    position
                }
            )
            .is_err()
        );
    }
    db.execute_batch(
        "CREATE TRIGGER fail_move BEFORE INSERT ON prototype_cards
        WHEN NEW.id = 2 BEGIN SELECT RAISE(ABORT, 'test write failure'); END;",
    )
    .unwrap();
    assert!(
        run(
            &mut db,
            StorageRequest::MoveCard {
                expected: initial.clone(),
                card: 1,
                column: 1,
                position: 0
            }
        )
        .is_err()
    );
    assert_eq!(load(&db, Space::A).unwrap(), Some(initial.clone()));
    db.execute_batch("DROP TRIGGER fail_move;").unwrap();
    let next = run(
        &mut db,
        StorageRequest::MoveCard {
            expected: initial.clone(),
            card: 1,
            column: 1,
            position: 0,
        },
    )
    .unwrap()
    .snapshot
    .unwrap();
    for _ in 0..2 {
        assert!(
            run(
                &mut db,
                StorageRequest::MoveCard {
                    expected: initial.clone(),
                    card: 1,
                    column: 1,
                    position: 0
                }
            )
            .is_err()
        );
        assert!(
            run(
                &mut db,
                StorageRequest::MoveColumn {
                    expected: initial.clone(),
                    column: 0,
                    position: 2
                }
            )
            .is_err()
        );
        assert_eq!(load(&db, Space::A).unwrap(), Some(next.clone()));
    }
    assert_eq!(load(&db, Space::B).unwrap(), Some(synthetic(Space::B)));
}

#[test]
fn empty_destination_and_last_card_can_be_moved_back() {
    let (_directory, mut db, mut initial) = seeded();
    initial.cards.truncate(1);
    run(
        &mut db,
        StorageRequest::Restore {
            snapshot: initial.clone(),
        },
    )
    .unwrap();
    let next = run(
        &mut db,
        StorageRequest::MoveCard {
            expected: initial.clone(),
            card: 1,
            column: 2,
            position: 0,
        },
    )
    .unwrap()
    .snapshot
    .unwrap();
    assert_eq!(next.cards.len(), 1);
    assert_eq!(next.cards[0].column, 2);
    assert_eq!(
        run(
            &mut db,
            StorageRequest::MoveCard {
                expected: next,
                card: 1,
                column: 0,
                position: 0
            }
        )
        .unwrap()
        .snapshot,
        Some(initial)
    );
}

#[tokio::test]
async fn concurrent_commands_with_same_source_have_one_winner() {
    let (directory, db, initial) = seeded();
    drop(db);
    let storage = Storage::new(directory.path().into());
    let (a, b) = tokio::join!(
        storage.execute(StorageRequest::MoveCard {
            expected: initial.clone(),
            card: 1,
            column: 1,
            position: 0
        }),
        storage.execute(StorageRequest::MoveCard {
            expected: initial,
            card: 2,
            column: 2,
            position: 0
        })
    );
    assert_ne!(a.is_ok(), b.is_ok());
    let current = storage
        .execute(StorageRequest::Snapshot { space: Space::A })
        .await
        .unwrap()
        .snapshot
        .unwrap();
    assert_eq!(current.cards.len(), 30);
    assert_eq!(current, a.or(b).unwrap().snapshot.unwrap());
}
