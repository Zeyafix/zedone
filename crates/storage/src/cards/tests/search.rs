use super::*;
use zedone_core::{
    ordering::{self as order, MoveCard, MoveColumn, Rank},
    search::{self, SearchCards, SearchResults, SearchScope},
};

async fn find(
    s: &Storage,
    a: &PendingAccess,
    id: u8,
    query: &str,
    scope: SearchScope,
) -> StorageResult<zedone_core::replay::Execution<SearchResults>> {
    let secret = a.credential().0.to_vec();
    let r = req(
        id,
        SearchCards {
            query: query.into(),
            scope,
        },
    );
    s.transact(move |ctx| Ok(search::search(ctx, &Credential(&secret), &Now, &r)?))
        .await
}
fn one() -> SearchScope {
    SearchScope::space(sid(1))
}
fn ids(result: &SearchResults) -> Vec<String> {
    result
        .spaces
        .iter()
        .flat_map(|s| s.hits.iter().map(|h| h.card.card.clone()))
        .collect()
}

#[tokio::test]
async fn search_literal_unicode_fragments_keep_fields_variants_and_whitespace_separate() {
    let (_dir, s, a) = setup().await;
    let mut r = creation(1, 9);
    r.parameters.title = "Перепланировка ЁЛКА Café Straße Σςσ ᾃ Й Ａ".into();
    r.parameters.description =
        "Нужно купить молоко сегодня\nC++ план* \"фраза\" ' % _ ? (x) \\ a  b c\td e\nf".into();
    let original = run!(s, a, create, r).unwrap().result;
    for (i, (query, expected)) in [
        ("ПЛАН", true),
        ("ровка", true),
        ("переп", true),
        ("ёлка", true),
        ("елка", false),
        ("Cafe\u{301}", true),
        ("STRASSE", true),
        ("σσσ", true),
        ("ἃι", true),
        ("и\u{306}", true),
        ("A", true), // Latin a occurs in Café.
        ("\u{a0}купить молоко\u{2003}", true),
        ("молоко купить", false),
        ("купить сегодня молоко", false),
        ("купить  молоко", false),
        ("купить\tмолоко", false),
        ("купить\nмолоко", false),
        ("C++", true),
        ("план*", true),
        ("план?", false),
        ("\"фраза\"", true),
        ("'", true),
        ("%", true),
        ("_", true),
        ("?", true),
        ("(x)", true),
        ("\\", true),
        ("a  b", true),
        ("a b", false),
        ("c\td", true),
        ("c d", false),
        ("e\nf", true),
        ("e f", false),
        ("Ａ Нужно", false),
        ("", true),
        ("   ", true),
        ("\u{a0}\u{2003}\t\n", true),
        ("нет", false),
    ]
    .into_iter()
    .enumerate()
    {
        let result = find(&s, &a, 10 + i as u8, query, one())
            .await
            .unwrap()
            .result;
        assert_eq!(
            ids(&result),
            if expected { vec![sid(9)] } else { vec![] },
            "{query:?}"
        );
        if expected {
            assert_eq!(result.spaces[0].hits[0].card, original);
        }
    }
    assert_eq!(fresh(&s, &a, 80, 9).await, original);
    // A history version is not a current conflicting text variant.
    run!(s, a, edit, edit_req(81, 9, "New", "body", original.version)).unwrap();
    assert!(ids(&find(&s, &a, 82, "план", one()).await.unwrap().result).is_empty());
    assert!(ids(&find(&s, &a, 83, "New body", one()).await.unwrap().result).is_empty());
}

#[tokio::test]
async fn search_follows_moves_completion_trash_restore_and_frozen_receipts_after_restart() {
    let (dir, s, a) = setup().await;
    let first = run!(s, a, create, creation(1, 9)).unwrap().result;
    let second = run!(s, a, create, creation(2, 8)).unwrap().result;
    let original = find(&s, &a, 3, "Ёж", one()).await.unwrap().result;
    assert_eq!(ids(&original), [sid(9), sid(8)]);
    let b = a.clone();
    s.transact(move |ctx| {
        Ok(order::move_card(
            ctx,
            &b.credential(),
            &Now,
            &req(
                4,
                MoveCard {
                    space: sid(1),
                    card: sid(8),
                    column: sid(3),
                    before: None,
                    expected: second.version,
                },
            ),
        )?)
    })
    .await
    .unwrap();
    let column = s
        .transact(|ctx| Ok(ctx.column_snapshot(&sid(1), &sid(3))?))
        .await
        .unwrap();
    let b = a.clone();
    s.transact(move |ctx| {
        Ok(order::move_column(
            ctx,
            &b.credential(),
            &Now,
            &req(
                5,
                MoveColumn {
                    space: sid(1),
                    column: sid(3),
                    before: Some(sid(2)),
                    expected: column.version,
                },
            ),
        )?)
    })
    .await
    .unwrap();
    assert_eq!(
        ids(&find(&s, &a, 6, "", one()).await.unwrap().result),
        [sid(8), sid(9)]
    );
    run!(
        s,
        a,
        set_completed,
        req(
            7,
            zedone_core::cards::SetCardCompleted {
                space: sid(1),
                card: sid(9),
                completed: true,
                expected: first.version
            }
        )
    )
    .unwrap();
    let current = fresh(&s, &a, 8, 9).await;
    let trash = run!(
        s,
        a,
        trash,
        req(
            9,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: current.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(
        ids(&find(&s, &a, 10, "", one()).await.unwrap().result),
        [sid(8)]
    );
    run!(
        s,
        a,
        restore,
        req(
            11,
            RestoreCard {
                column: None,
                expected_column: None,
                space: sid(1),
                card: sid(9),
                expected: trash.version
            }
        )
    )
    .unwrap();
    let current = find(&s, &a, 12, "Ёж", one()).await.unwrap().result;
    assert_eq!(ids(&current), [sid(8), sid(9)]);
    assert_eq!(current.spaces[0].hits[1].card.completed, [true]);
    s.shutdown().await.unwrap();
    let s = Storage::open(dir.path().join("cards.db")).await.unwrap();
    let old = find(&s, &a, 3, "Ёж", one()).await.unwrap();
    assert!(old.replayed);
    assert_eq!(old.result, original);
    assert_eq!(find(&s, &a, 13, "Ёж", one()).await.unwrap().result, current);
    rejected(find(&s, &a, 3, "", one()).await, ErrorCode::CommandMismatch);
}

// Conflict fixtures represent independently committed changes; the product
// sync/resolution commands are deliberately outside this API slice.
async fn conflict(s: &Storage, target: u8, field: &'static str, values: Vec<String>) {
    s.transact(move |ctx| {
        let graph = ctx.read().document(&sid(1))?;
        let object = if target == 1 {
            ROOT
        } else if target <= 3 {
            column_object(graph, &sid(target))?
        } else {
            object(graph, &sid(target))?
        };
        let mut changes = Vec::new();
        for (i, value) in values.into_iter().enumerate() {
            let mut branch = graph.clone().with_actor(ActorId::from(vec![71 + i as u8]));
            let mut tx = branch.transaction();
            tx.put(&object, field, value)?;
            tx.commit();
            changes.push(branch.get_last_local_change().unwrap());
        }
        ctx.apply_changes(&sid(1), changes)?;
        Ok(())
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn search_keeps_text_conflicts_on_board_and_groups_disputed_places_without_duplicates() {
    let (_dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    run!(s, a, create, creation(2, 8)).unwrap();
    conflict(
        &s,
        9,
        "title",
        vec!["Купить молоко".into(), "Молоко купить".into()],
    )
    .await;
    let result = find(&s, &a, 3, "молоко", one()).await.unwrap().result;
    assert_eq!(ids(&result), [sid(9)]);
    assert!(!result.spaces[0].hits[0].placement_conflict);
    assert_eq!(result.spaces[0].hits[0].card.state, SpaceState::Conflicted);
    assert!(
        ids(&find(&s, &a, 4, "молоко молоко", one())
            .await
            .unwrap()
            .result)
        .is_empty()
    );
    conflict(
        &s,
        9,
        "placement",
        [2, 3]
            .map(|c| {
                serde_json::to_string(&order::CardPlacement {
                    column: sid(c),
                    rank: Rank::Legacy(21),
                })
                .unwrap()
            })
            .to_vec(),
    )
    .await;
    let result = find(&s, &a, 5, "", one()).await.unwrap().result;
    assert_eq!(ids(&result), [sid(9), sid(8)]);
    assert!(result.spaces[0].hits[0].placement_conflict);
    // Resolve card's place only; text conflict remains on its unambiguous board place.
    s.transact(|ctx| {
        let obj = object(ctx.read().document(&sid(1))?, &sid(9))?;
        ctx.edit(&sid(1), |tx| {
            tx.put(
                &obj,
                "placement",
                serde_json::to_string(&order::CardPlacement {
                    column: sid(3),
                    rank: Rank::Legacy(20),
                })
                .unwrap(),
            )?;
            Ok(())
        })?;
        Ok(())
    })
    .await
    .unwrap();
    let result = find(&s, &a, 6, "", one()).await.unwrap().result;
    assert_eq!(ids(&result), [sid(8), sid(9)]);
    assert!(result.spaces[0].hits.iter().all(|h| !h.placement_conflict));
    // Conflicted column position, even with one effective card column.
    s.transact(|ctx| {
        let graph = ctx.read().document(&sid(1))?;
        let obj = column_object(graph, &sid(3))?;
        let mut changes = Vec::new();
        for n in [91_u8, 92] {
            let mut branch = graph.clone().with_actor(ActorId::from(vec![n]));
            let mut tx = branch.transaction();
            tx.put(&obj, "position", u64::from(n))?;
            tx.commit();
            changes.push(branch.get_last_local_change().unwrap());
        }
        ctx.apply_changes(&sid(1), changes)?;
        Ok(())
    })
    .await
    .unwrap();
    let result = find(&s, &a, 7, "", one()).await.unwrap().result;
    assert_eq!(ids(&result), [sid(9), sid(8)]);
    assert!(result.spaces[0].hits[0].placement_conflict);
}

#[tokio::test]
async fn search_orders_name_conflicts_and_equal_names_and_rechecks_narrowed_grants_on_replay() {
    use crate::access::AccessKey;
    use std::collections::BTreeMap;
    let (_dir, s, a) = setup().await;
    run!(s, a, create, creation(1, 9)).unwrap();
    let b = a.clone();
    s.transact(move |ctx| {
        for (i, space) in [10, 11].into_iter().enumerate() {
            let id = 100 + 3 * i as u8;
            spaces::create(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    id,
                    CreateSpace {
                        space: sid(space),
                        name: "alpha".into(),
                    },
                ),
            )?;
            zedone_core::columns::create(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    id + 1,
                    CreateColumn {
                        space: sid(space),
                        column: sid(2),
                        name: "To do".into(),
                    },
                ),
            )?;
            let mut r = creation(id + 2, 9);
            r.parameters.space = sid(space);
            core::create(ctx, &b.credential(), &Now, &r)?;
        }
        Ok(())
    })
    .await
    .unwrap();
    let all = find(&s, &a, 2, "", SearchScope::all())
        .await
        .unwrap()
        .result;
    let order = |r: &SearchResults| {
        r.spaces
            .iter()
            .map(|s| s.space.space.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(order(&all), [sid(10), sid(11), sid(1)]);
    conflict(&s, 1, "name", vec!["Z".into(), "Y".into()]).await;
    let result = find(&s, &a, 3, "ёж", SearchScope::all())
        .await
        .unwrap()
        .result;
    assert_eq!(order(&result), [sid(1), sid(10), sid(11)]);
    assert_eq!(result.spaces[0].space.names, ["Y", "Z"]);
    assert!(!result.spaces[0].hits[0].placement_conflict);
    assert!(
        find(&s, &a, 4, "unmatched", SearchScope::all())
            .await
            .unwrap()
            .result
            .spaces
            .is_empty()
    );
    s.transact(|ctx| {
        ctx.edit(&sid(1), |tx| {
            tx.put(ROOT, "name", "Z")?;
            Ok(())
        })?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        order(
            &find(&s, &a, 5, "", SearchScope::all())
                .await
                .unwrap()
                .result
        ),
        [sid(10), sid(11), sid(1)]
    );
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (b, m) = (a.clone(), member.clone());
    s.transact(move |ctx| {
        ctx.issue_access(
            &b.credential(),
            &Now,
            &m,
            &GrantScope::Spaces(BTreeMap::from([
                (sid(1), SpacePermission::Read),
                (sid(10), SpacePermission::Read),
            ])),
            None,
        )
    })
    .await
    .unwrap();
    let original = find(&s, &member, 6, "", SearchScope::all())
        .await
        .unwrap()
        .result;
    assert_eq!(order(&original), [sid(10), sid(1)]);
    rejected(
        find(&s, &member, 7, "", SearchScope::space(sid(11))).await,
        ErrorCode::AccessDenied,
    );
    let replacement = Arc::new(AccessKey::generate().unwrap());
    let (b, m, k) = (a.clone(), member.clone(), replacement.clone());
    s.transact(move |ctx| {
        ctx.replace_access(
            &b.credential(),
            &Now,
            m.subject(),
            &k,
            &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
            None,
        )
    })
    .await
    .unwrap();
    let secret = replacement.credential().0.to_vec();
    let error = s
        .transact(move |ctx| {
            Ok(search::search(
                ctx,
                &Credential(&secret),
                &Now,
                &req(
                    6,
                    SearchCards {
                        query: "".into(),
                        scope: SearchScope::all(),
                    },
                ),
            )?)
        })
        .await;
    rejected(error, ErrorCode::AccessDenied);
    // The old credential is invalid even for a recorded read.
    rejected(
        find(&s, &member, 6, "", SearchScope::all()).await,
        ErrorCode::AccessDenied,
    );
    let secret = replacement.credential().0.to_vec();
    let result = s
        .transact(move |ctx| {
            Ok(search::search(
                ctx,
                &Credential(&secret),
                &Now,
                &req(
                    8,
                    SearchCards {
                        query: "".into(),
                        scope: SearchScope::all(),
                    },
                ),
            )?)
        })
        .await
        .unwrap()
        .result;
    assert_eq!(order(&result), [sid(1)]);
}

#[tokio::test]
async fn search_receipt_failure_does_not_change_domain_data_and_can_be_retried() {
    let (_dir, s, a) = setup().await;
    let original = run!(s, a, create, creation(1, 9)).unwrap().result;
    s.transact(|ctx|{ctx.connection().execute_batch("CREATE TRIGGER fail_search BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;Ok(())}).await.unwrap();
    rejected(find(&s, &a, 2, "", one()).await, ErrorCode::Unavailable);
    s.transact(|ctx| {
        ctx.connection().execute_batch("DROP TRIGGER fail_search")?;
        Ok(())
    })
    .await
    .unwrap();
    let result = find(&s, &a, 2, "", one()).await.unwrap();
    assert!(!result.replayed);
    assert_eq!(result.result.spaces[0].hits[0].card, original);
    assert_eq!(fresh(&s, &a, 3, 9).await, original);
}

#[tokio::test]
async fn search_thousand_card_probe_uses_authoritative_graph_without_an_index() {
    let (_dir, s, a) = setup().await;
    // Trusted fixture builds a valid large graph in one commit; creation and
    // editing correctness are independently covered through real commands.
    s.transact(|ctx| {
        ctx.edit(&sid(1), |tx| {
            for i in 1000..2000_u64 {
                let card = format!("{i:032x}");
                let obj = tx.put_object(ROOT, format!("card:{card}"), ObjType::Map)?;
                tx.put(&obj, "format", "zedone.card.v1")?;
                tx.put(&obj, "title", format!("Планирование {i}"))?;
                tx.put(&obj, "description", "Café Straße **текст**")?;
                tx.put(&obj, "column", sid(2))?;
                tx.put(&obj, "position", i)?;
                tx.put(&obj, "completed", i % 2 == 0)?;
                tx.put_object(&obj, "tags", ObjType::Map)?;
                tx.put(&obj, "lifecycle", "active")?;
                let h = tx.put_object(&obj, "history", ObjType::Map)?;
                append_version(tx, &h, &format!("{}:{}", sid(1), card), 0, "Title", "Body")?;
            }
            Ok(())
        })?;
        Ok(())
    })
    .await
    .unwrap();
    let start = std::time::Instant::now();
    let result = find(&s, &a, 1, "STRASSE", one()).await.unwrap().result;
    let elapsed = start.elapsed();
    assert_eq!(ids(&result).len(), 1000);
    assert_eq!(ids(&result)[0], format!("{:032x}", 1000));
    assert_eq!(ids(&result)[999], format!("{:032x}", 1999));
    assert_eq!(
        result.spaces[0]
            .hits
            .iter()
            .filter(|h| h.card.completed == [true])
            .count(),
        500
    );
    eprintln!(
        "search_probe cards=1000 matching=1000 elapsed_ms={}",
        elapsed.as_millis()
    );
    // A generous deadlock/runaway guard, not an accepted user latency budget.
    assert!(elapsed < Duration::from_secs(30));
}
