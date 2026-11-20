use super::super::text_conflicts::{copy, exchange};
use super::*;
use zedone_core::{
    column_positions::{self, ResolveColumnPosition},
    columns::{self, ColumnSnapshot, ReadColumn},
    placements::{self, ResolveCardPlacement},
    spaces::{ReadSpace, RenameSpace, ResolveSpaceName},
};

macro_rules! invoke {
    ($db:expr, $access:expr, $handler:path, $parameters:expr) => {{
        let secret = $access.credential().0.to_vec();
        let mut request = req(0, $parameters);
        getrandom::fill(&mut request.id).unwrap();
        $db.transact(move |tx| Ok($handler(tx, &Credential(&secret), &Now, &request)?))
            .await
            .unwrap()
            .result
    }};
}

async fn current_card(db: &Storage, access: &PendingAccess, card: u8) -> CardSnapshot {
    invoke!(
        db,
        access,
        core::read,
        ReadCard {
            space: sid(1),
            card: sid(card)
        }
    )
}

async fn current_column(db: &Storage, access: &PendingAccess) -> ColumnSnapshot {
    invoke!(
        db,
        access,
        columns::read,
        ReadColumn {
            space: sid(1),
            column: sid(2)
        }
    )
}

async fn current_space(db: &Storage, access: &PendingAccess) -> zedone_core::spaces::SpaceSnapshot {
    invoke!(db, access, spaces::read, ReadSpace { space: sid(1) })
}

async fn fresh_search(db: &Storage, access: &PendingAccess, query: &str) -> SearchResults {
    invoke!(
        db,
        access,
        search::search,
        SearchCards {
            query: query.into(),
            scope: SearchScope::all()
        }
    )
}

async fn verify(left: &Storage, right: &Storage, access: &PendingAccess, remaining: &[&str]) {
    let space = current_space(left, access).await;
    assert_eq!(
        space.names.len(),
        if remaining.contains(&"space") { 2 } else { 1 }
    );
    let expected_conflicts = if remaining.contains(&"column") {
        vec![sid(7), sid(8), sid(9)]
    } else if remaining.contains(&"card") {
        vec![sid(9)]
    } else {
        vec![]
    };
    let queries: &[&str] = if remaining.len() == 3 {
        &[
            "",
            " \u{2003}\t",
            "Ёж",
            "STRASSE",
            "Cafe\u{301}",
            "title-only",
            "body-only",
            "C++",
            "missing",
        ]
    } else {
        &["", "Ёж"]
    };
    for &query in queries {
        let result = fresh_search(left, access, query).await;
        assert_eq!(
            fresh_search(right, access, query).await,
            result,
            "{query:?}"
        );
        let hits: Vec<_> = result.spaces.iter().flat_map(|s| &s.hits).collect();
        if query.trim().is_empty() {
            let expected = if remaining.contains(&"space") {
                [sid(1), sid(10)]
            } else {
                [sid(10), sid(1)]
            };
            assert_eq!(
                result
                    .spaces
                    .iter()
                    .map(|item| item.space.space.clone())
                    .collect::<Vec<_>>(),
                expected
            );
        }
        let hits: Vec<_> = hits
            .into_iter()
            .filter(|hit| hit.card.space == sid(1))
            .collect();
        let found: Vec<_> = hits.iter().map(|h| h.card.card.clone()).collect();
        assert_eq!(
            found
                .iter()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            found.len()
        );
        match query {
            "missing" => assert!(result.spaces.is_empty()),
            "STRASSE" | "Cafe\u{301}" | "title-only" | "body-only" | "C++" => {
                assert_eq!(found, [sid(8)]);
                assert_eq!(hits[0].placement_conflict, remaining.contains(&"column"));
                assert_eq!(hits[0].card.titles.len(), 2);
            }
            _ => {
                assert_eq!(found.len(), 3);
                assert_eq!(
                    hits.iter()
                        .filter(|h| h.placement_conflict)
                        .map(|h| h.card.card.clone())
                        .collect::<Vec<_>>(),
                    expected_conflicts
                );
                if expected_conflicts.is_empty() {
                    let listed = invoke!(
                        left,
                        access,
                        core::list,
                        ListCards {
                            space: sid(1),
                            include_trashed: false
                        }
                    );
                    let columns = invoke!(
                        left,
                        access,
                        columns::list,
                        columns::ListColumns {
                            space: sid(1),
                            include_trashed: false
                        }
                    );
                    let expected: Vec<_> = columns
                        .columns
                        .iter()
                        .flat_map(|column| {
                            listed
                                .cards
                                .iter()
                                .filter(|card| card.columns == [column.column.clone()])
                                .map(|card| card.card.clone())
                        })
                        .collect();
                    assert_eq!(found, expected);
                }
            }
        }
    }
    let list = invoke!(
        left,
        access,
        spaces::list,
        spaces::ListSpaces {
            include_trashed: false
        }
    );
    assert_eq!(list.spaces.len(), 2);
    assert_eq!(
        list.spaces
            .iter()
            .find(|item| item.space == sid(1))
            .unwrap()
            .names,
        space.names
    );
}

#[tokio::test]
async fn search_exchange_partial_resolutions_keep_variants_groups_order_and_history() {
    let orders = [
        ["space", "card", "column"],
        ["space", "column", "card"],
        ["card", "space", "column"],
        ["card", "column", "space"],
        ["column", "space", "card"],
        ["column", "card", "space"],
    ];
    for same in [false, true] {
        for order in orders {
            let (dir, left, access) = setup().await;
            invoke!(
                &left,
                access,
                columns::create,
                CreateColumn {
                    space: sid(1),
                    column: sid(4),
                    name: "Third".into()
                }
            );
            for (operation, card) in [(1, 7), (2, 9), (3, 8)] {
                run!(&left, access, create, creation(operation, card)).unwrap();
            }
            invoke!(
                &left,
                access,
                spaces::create,
                CreateSpace {
                    space: sid(10),
                    name: "Alpha".into()
                }
            );
            invoke!(
                &left,
                access,
                columns::create,
                CreateColumn {
                    space: sid(10),
                    column: sid(11),
                    name: "Other".into()
                }
            );
            invoke!(
                &left,
                access,
                core::create,
                CreateCard {
                    space: sid(10),
                    column: sid(11),
                    card: sid(12),
                    title: "Ordinary".into(),
                    description: "Unchanged".into()
                }
            );
            let other_page = left
                .transact(|tx| {
                    crate::sync_engine::page(tx.read().document(&sid(10))?, &sid(10), &vec![])
                })
                .await
                .unwrap();
            let right = copy(&left, dir.path().join("peer.db"), &access).await;
            let owner = access.clone();
            right
                .transact(move |tx| {
                    let selection = tx.read().sync_selection(&sid(10), &sid(99))?;
                    tx.set_sync_selected(&sid(10), &sid(99), true, selection.version)?;
                    tx.receive_sync_page(&sid(10), &sid(99), owner.subject(), &other_page)
                })
                .await
                .unwrap();
            for (db, label, before) in [
                (&left, "Straße title-only", Some(sid(7))),
                (&right, "Café C++", None),
            ] {
                let space = current_space(db, &access).await;
                invoke!(
                    db,
                    access,
                    spaces::rename,
                    RenameSpace {
                        space: sid(1),
                        name: label.into(),
                        expected: space.version
                    }
                );
                let column = current_column(db, &access).await;
                invoke!(
                    db,
                    access,
                    order::move_column,
                    MoveColumn {
                        space: sid(1),
                        column: sid(2),
                        before: before.as_ref().map(|_| sid(4)),
                        expected: column.version
                    }
                );
                let card = current_card(db, &access, 9).await;
                invoke!(
                    db,
                    access,
                    order::move_card,
                    MoveCard {
                        space: sid(1),
                        card: sid(9),
                        column: sid(if same || before.is_some() { 2 } else { 3 }),
                        before,
                        expected: card.version
                    }
                );
                let text = current_card(db, &access, 8).await;
                invoke!(
                    db,
                    access,
                    core::edit,
                    EditCard {
                        space: sid(1),
                        card: sid(8),
                        title: format!("Ёж {label}"),
                        description: format!("body-only {label}"),
                        expected: text.version
                    }
                );
            }
            exchange(&left, &right).await;
            let initial = current_card(&left, &access, 9).await;
            let history = versions(&left, 8).await;
            verify(&left, &right, &access, &order).await;
            for (index, kind) in order.iter().enumerate() {
                match *kind {
                    "space" => {
                        let s = current_space(&left, &access).await;
                        invoke!(
                            &left,
                            access,
                            spaces::resolve_name,
                            ResolveSpaceName {
                                space: sid(1),
                                name: "Resolved".into(),
                                expected: s.version
                            }
                        );
                    }
                    "column" => {
                        let c = current_column(&left, &access).await;
                        invoke!(
                            &left,
                            access,
                            column_positions::resolve,
                            ResolveColumnPosition {
                                space: sid(1),
                                column: sid(2),
                                position: c.positions[0].clone(),
                                expected: c.version
                            }
                        );
                    }
                    "card" => {
                        let c = current_card(&left, &access, 9).await;
                        let placement = c
                            .placements
                            .iter()
                            .find(|p| p.column == sid(2))
                            .unwrap()
                            .clone();
                        invoke!(
                            &left,
                            access,
                            placements::resolve,
                            ResolveCardPlacement {
                                space: sid(1),
                                card: sid(9),
                                placement,
                                expected: c.version
                            }
                        );
                    }
                    _ => unreachable!(),
                }
                exchange(&left, &right).await;
                verify(&left, &right, &access, &order[index + 1..]).await;
                assert_eq!(versions(&left, 8).await, history);
                assert_eq!(current_card(&left, &access, 9).await.titles, initial.titles);
            }
            let conflicted = current_card(&left, &access, 8).await;
            invoke!(
                &left,
                access,
                core::resolve_text,
                core::ResolveCardText {
                    space: sid(1),
                    card: sid(8),
                    title: "Ёж resolved".into(),
                    description: "Selected text".into(),
                    expected: conflicted.version
                }
            );
            exchange(&left, &right).await;
            assert!(
                fresh_search(&left, &access, "STRASSE")
                    .await
                    .spaces
                    .is_empty()
            );
            assert!(
                fresh_search(&right, &access, "Cafe\u{301}")
                    .await
                    .spaces
                    .is_empty()
            );
            let result = fresh_search(&left, &access, "").await;
            assert!(result.spaces[0].hits.iter().all(|h| !h.placement_conflict));
            assert_eq!(versions(&left, 8).await.len(), history.len() + 1);
            left.shutdown().await.unwrap();
            let reopened = Storage::open(dir.path().join("cards.db")).await.unwrap();
            assert_eq!(fresh_search(&reopened, &access, "").await, result);
            assert_eq!(fresh_search(&right, &access, "").await, result);
            reopened.shutdown().await.unwrap();
            right.shutdown().await.unwrap();
            println!("search exchange {same:?}/{order:?}: passed");
        }
    }
}
