use super::*;

fn prepared() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("automerge-order.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    (directory, store)
}

fn save_move(
    store: &mut Store,
    id: &str,
    key: &str,
    parent: &str,
    left: Option<&str>,
    right: Option<&str>,
) {
    let mut replica = store.read(id).unwrap();
    relocate(&mut replica.doc, key, parent, left, right, None).unwrap();
    store.write(id, &replica).unwrap();
}

fn group(doc: &Automerge, key: &str) -> Value {
    let id = object(doc, key).unwrap().to_string();
    snapshot(doc).unwrap()["groups"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["parent"] == id)
        .unwrap()
        .clone()
}

#[test]
fn ranks_allow_repeated_insertion_at_edges_and_between_same_gap_siblings() {
    let x = ranks::between(None, None, "x").unwrap();
    let y = ranks::between(None, None, "y").unwrap();
    assert!(x < y);
    assert_eq!(x, ranks::between(None, None, "x").unwrap());
    let mut keys = vec![x, y];
    let mut random = 17_u64;
    for n in 0..1000 {
        random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
        let index = (random as usize) % (keys.len() + 1);
        let key = ranks::between(
            index.checked_sub(1).map(|i| &keys[i]),
            keys.get(index),
            &n.to_string(),
        )
        .unwrap();
        keys.insert(index, key);
    }
    for _ in 0..100 {
        let first = ranks::between(None, keys.first(), "start").unwrap();
        keys.insert(0, first);
        let last = ranks::between(keys.last(), None, "end").unwrap();
        keys.push(last);
    }
    assert!(keys.windows(2).all(|p| p[0] < p[1]));
    assert!(ranks::between(keys.last(), keys.first(), "invalid").is_err());
}

#[test]
fn different_card_positions_conflict_once_and_keep_identity_text_and_history() {
    for cross_column in [false, true] {
        let (_directory, mut store) = prepared();
        let original = view(&store.read("A").unwrap().doc, "k1").unwrap();
        let heads = store.read("A").unwrap().doc.get_heads();
        if cross_column {
            save_move(&mut store, "A", "k1", "c1", None, None);
            save_move(&mut store, "B", "k1", "c2", None, None);
        } else {
            save_move(&mut store, "A", "k1", "c0", None, Some("k0"));
            save_move(&mut store, "B", "k1", "c0", Some("k3"), None);
        }
        exchange(&mut store).unwrap();
        let mut a = store.read("A").unwrap();
        let observed = view(&a.doc, "k1").unwrap();
        assert_eq!(observed.id, original.id);
        assert_eq!(observed.text, original.text);
        assert_eq!(observed.place.as_ref().unwrap().len(), 2);
        assert_eq!(group(&a.doc, "s")["conflicts"], json!([original.id]));
        for parent in ["c0", "c1", "c2"] {
            assert!(!ordered(&a.doc, parent).unwrap().contains(&"k1".into()));
        }
        let before = a.fingerprint().unwrap();
        assert!(edit(&mut a.doc, "k1", "name", "Blocked", None).is_err());
        assert!(relocate(&mut a.doc, "k1", "c1", None, None, None).is_err());
        assert_eq!(before, a.fingerprint().unwrap());
        relocate(&mut a.doc, "k1", "c1", None, None, Some(&observed)).unwrap();
        assert!(
            group(&a.doc, "s")["conflicts"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        relocate(&mut a.doc, "k1", "c0", Some("k0"), Some("k2"), None).unwrap();
        assert_eq!(ordered(&a.doc, "c0").unwrap(), ["k0", "k1", "k2", "k3"]);
        let historical = a.doc.fork_at(&heads).unwrap();
        assert_eq!(view(&historical, "k1").unwrap(), original);
        store.write("A", &a).unwrap();
        exchange(&mut store).unwrap();
        assert_eq!(
            snapshot(&a.doc).unwrap(),
            snapshot(&store.read("B").unwrap().doc).unwrap()
        );
    }
}

#[test]
fn column_position_conflict_keeps_children_available_and_other_conflicts_block() {
    let (_directory, mut store) = prepared();
    save_move(&mut store, "A", "c0", "s", Some("c1"), Some("c2"));
    save_move(&mut store, "B", "c0", "s", Some("c3"), None);
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let column = view(&a.doc, "c0").unwrap();
    assert_eq!(group(&a.doc, "s")["conflicts"], json!([column.id]));
    assert_eq!(ordered(&a.doc, "c0").unwrap(), ["k0", "k1", "k2", "k3"]);
    assert!(edit(&mut a.doc, "c0", "name", "Blocked", None).is_err());
    edit(&mut a.doc, "k0", "text", "Доступно", None).unwrap();
    relocate(&mut a.doc, "k3", "c1", None, None, None).unwrap();
    // Child changes do not invalidate an unchanged column choice.
    relocate(&mut a.doc, "c0", "s", None, Some("c1"), Some(&column)).unwrap();
    edit(&mut a.doc, "c0", "name", "Доступно", None).unwrap();
    assert_eq!(ordered(&a.doc, "s").unwrap(), ["c0", "c1", "c2", "c3"]);
    assert!(
        group(&a.doc, "s")["conflicts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn resolving_position_does_not_clear_text_conflicts_and_late_moves_remain_variants() {
    let (_directory, mut store) = prepared();
    for (id, parent) in [("A", "c1"), ("B", "c2")] {
        let mut replica = store.read(id).unwrap();
        relocate(&mut replica.doc, "k1", parent, None, None, None).unwrap();
        edit(&mut replica.doc, "k1", "text", id, None).unwrap();
        store.write(id, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let initial = view(&store.read("A").unwrap().doc, "k1").unwrap();
    for (id, parent) in [("A", "c1"), ("B", "c2")] {
        let mut replica = store.read(id).unwrap();
        let observed = view(&replica.doc, "k1").unwrap();
        relocate(&mut replica.doc, "k1", parent, None, None, Some(&observed)).unwrap();
        assert!(view(&replica.doc, "k1").unwrap().blocked());
        assert!(relocate(&mut replica.doc, "k1", "c3", None, None, None).is_err());
        let observed = view(&replica.doc, "k1").unwrap();
        edit(
            &mut replica.doc,
            "k1",
            "text",
            "Объединено",
            Some(&observed),
        )
        .unwrap();
        if id == "B" {
            relocate(&mut replica.doc, "k1", "c3", None, None, None).unwrap();
        }
        store.write(id, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let current = view(&a.doc, "k1").unwrap();
    let parents: std::collections::BTreeSet<_> = current
        .place
        .as_ref()
        .unwrap()
        .iter()
        .map(|v| serde_json::from_str::<Place>(&v.value).unwrap().parent)
        .collect();
    assert_eq!(
        parents,
        ["c1", "c3"]
            .map(|k| object(&a.doc, k).unwrap().to_string())
            .into()
    );
    assert!(!conflicting(current.text.as_ref().unwrap()));
    let before = a.fingerprint().unwrap();
    assert!(relocate(&mut a.doc, "k1", "c2", None, None, Some(&initial)).is_err());
    assert_eq!(before, a.fingerprint().unwrap());
}

#[test]
fn same_gap_objects_converge_for_cards_and_columns_and_equal_moves_are_not_conflicts() {
    for column in [false, true] {
        let (_directory, mut store) = prepared();
        let (x, y, parent, l, r) = if column {
            ("c0", "c3", "s", "c1", "c2")
        } else {
            ("k0", "k3", "c0", "k1", "k2")
        };
        save_move(&mut store, "A", x, parent, Some(l), Some(r));
        save_move(&mut store, "B", y, parent, Some(l), Some(r));
        exchange(&mut store).unwrap();
        let a = store.read("A").unwrap();
        let mut middle = [x, y];
        middle.sort_by_key(|key| object(&a.doc, key).unwrap().to_string());
        assert_eq!(
            ordered(&a.doc, parent).unwrap(),
            [l, middle[0], middle[1], r]
        );
        assert!(!view(&a.doc, x).unwrap().blocked());
        assert!(!view(&a.doc, y).unwrap().blocked());
        assert_eq!(
            snapshot(&a.doc).unwrap(),
            snapshot(&store.read("B").unwrap().doc).unwrap()
        );
        // A subsequent insertion between the formerly concurrent objects works too.
        let mut a = store.read("A").unwrap();
        relocate(
            &mut a.doc,
            l,
            parent,
            Some(middle[0]),
            Some(middle[1]),
            None,
        )
        .unwrap();
        assert_eq!(
            ordered(&a.doc, parent).unwrap(),
            [middle[0], l, middle[1], r]
        );
    }
    let (_directory, mut store) = prepared();
    for id in ["A", "B"] {
        save_move(&mut store, id, "k0", "c1", None, None);
    }
    exchange(&mut store).unwrap();
    let card = view(&store.read("A").unwrap().doc, "k0").unwrap();
    assert_eq!(card.place.as_ref().unwrap().len(), 2);
    assert!(!card.blocked());
    assert_eq!(
        ordered(&store.read("A").unwrap().doc, "c1").unwrap(),
        ["k0"]
    );
}

#[test]
fn invalid_destinations_gaps_and_stale_choices_never_mutate_document() {
    let (_directory, mut store) = prepared();
    let mut a = store.read("A").unwrap();
    let before = a.fingerprint().unwrap();
    for (parent, l, r) in [
        ("foreign", None, None),
        ("s", None, None),
        ("missing", None, None),
        ("c0", Some("k0"), Some("k3")),
        ("c0", Some("k1"), Some("k2")),
        ("c0", Some("k3"), Some("k0")),
    ] {
        assert!(relocate(&mut a.doc, "k1", parent, l, r, None).is_err());
        assert_eq!(before, a.fingerprint().unwrap());
    }
    drop(store);
    // Prepare the conflict using the real restart scenario in a fresh directory.
    let other = tempfile::tempdir().unwrap();
    run(other.path(), "order-prepare").unwrap();
    store = Store::open(&other.path().join("automerge-order.sqlite3")).unwrap();
    let mut a = store.read("A").unwrap();
    let choice = view(&a.doc, "k1").unwrap();
    let wrong = view(&a.doc, "c1").unwrap();
    let before = a.fingerprint().unwrap();
    assert!(relocate(&mut a.doc, "k1", "c1", None, None, Some(&wrong)).is_err());
    assert_eq!(before, a.fingerprint().unwrap());
    relocate(&mut a.doc, "k1", "c1", None, None, Some(&choice)).unwrap();
    let before = a.fingerprint().unwrap();
    assert!(relocate(&mut a.doc, "k1", "c2", None, None, Some(&choice)).is_err());
    assert_eq!(before, a.fingerprint().unwrap());
}

#[test]
fn process_phases_and_duplicate_reordered_changes_preserve_order_and_all_variants() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "order-prepare").unwrap();
    run(directory.path(), "order-resolve").unwrap();
    let mut store = Store::open(&directory.path().join("automerge-order.sqlite3")).unwrap();
    exchange(&mut store).unwrap();
    let original = store.read("A").unwrap();
    let expected = snapshot(&original.doc).unwrap();
    let changes = original.doc.get_changes(&[]);
    for reverse in [false, true] {
        for offset in 0..changes.len() {
            let mut delivery = changes.clone();
            delivery.rotate_left(offset);
            if reverse {
                delivery.reverse();
            }
            let mut target = Replica::new();
            for change in delivery {
                target.doc.apply_changes([change.clone(), change]).unwrap();
            }
            assert!(target.doc.get_missing_deps(&[]).is_empty());
            assert_eq!(target.doc.get_heads(), original.doc.get_heads());
            assert_eq!(target.doc.get_changes(&[]).len(), changes.len());
            assert_eq!(snapshot(&target.doc).unwrap(), expected);
            store.write("A", &target).unwrap();
            assert_eq!(snapshot(&store.read("A").unwrap().doc).unwrap(), expected);
        }
    }
    // A separate run tests the exact restart checkpoints without test mutation.
    let directory = tempfile::tempdir().unwrap();
    for phase in ["order-prepare", "order-resolve", "order-resume"] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "order-prepare").is_err());
}

#[test]
fn failed_sqlite_commit_preserves_order_variants_and_other_replica_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "order-prepare").unwrap();
    let mut store = Store::open(&directory.path().join("automerge-order.sqlite3")).unwrap();
    let before = store.read("A").unwrap().fingerprint().unwrap();
    let other = store.read("B").unwrap().fingerprint().unwrap();
    let mut candidate = store.read("A").unwrap();
    let observed = view(&candidate.doc, "k1").unwrap();
    relocate(&mut candidate.doc, "k1", "c1", None, None, Some(&observed)).unwrap();
    store.0.execute_batch("CREATE TRIGGER reject_order AFTER UPDATE ON replicas BEGIN SELECT RAISE(ABORT, 'synthetic order failure'); END;").unwrap();
    assert!(store.write("A", &candidate).is_err());
    drop(store);
    let store = Store::open(&directory.path().join("automerge-order.sqlite3")).unwrap();
    assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), before);
    assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), other);
}

#[test]
fn name_conflict_space_group_is_unique_stable_and_returns_to_automatic_order() {
    let (_directory, mut store) = prepared();
    for (id, name) in [("A", "Первое"), ("B", "Второе")] {
        let mut replica = store.read(id).unwrap();
        edit(&mut replica.doc, "s", "name", name, None).unwrap();
        store.write(id, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let selected = view(&a.doc, "s").unwrap();
    let spaces = snapshot(&a.doc).unwrap()["spaces"].clone();
    assert_eq!(spaces.as_array().unwrap().len(), 4);
    assert_eq!(spaces[0], json!([false, "", selected.id]));
    assert!(edit(&mut a.doc, "s", "name", "Blocked", None).is_err());
    // Parent name conflict does not prevent moves or child edits.
    relocate(&mut a.doc, "k0", "c1", None, None, None).unwrap();
    edit(&mut a.doc, "s", "name", "М", Some(&selected)).unwrap();
    let spaces = snapshot(&a.doc).unwrap()["spaces"].clone();
    let mut expected: Vec<_> = ["s", "a", "z", "t"]
        .into_iter()
        .map(|k| {
            let item = view(&a.doc, k).unwrap();
            (true, item.name[0].value.clone(), item.id)
        })
        .collect();
    expected.sort();
    assert_eq!(spaces, json!(expected));
    assert_eq!(spaces[0][1], "А");
    assert_eq!(spaces[3][1], "Я");
}
