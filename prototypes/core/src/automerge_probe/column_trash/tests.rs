use super::*;

fn prepared(conflict: bool) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("column.sqlite3")).unwrap();
    if conflict {
        scenario::prepare(&mut store).unwrap();
    } else {
        seed(&mut store).unwrap();
    }
    (dir, store)
}

fn choose(doc: &mut Automerge, state: &str) {
    set_column(doc, "c0", &column(doc, "c0").unwrap(), state).unwrap();
}

fn change(doc: &mut Automerge, key: &str, field: &str, value: &str) {
    edit(doc, key, &card(doc, key).unwrap(), field, value, false).unwrap();
}

fn location(doc: &Automerge, key: &str, parent: &str) -> &'static str {
    let col = column(doc, parent).unwrap();
    col.location(&col.cards[key])
}

#[test]
fn every_independent_card_edit_preserves_data_and_both_group_choices() {
    for (field, value) in FIELDS.into_iter().zip(scenario::EDITED) {
        for state in ["active", "trash"] {
            let (_dir, mut store) = prepared(false);
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            let initial = column(&a.doc, "c0").unwrap();
            choose(&mut a.doc, "trash");
            change(&mut b.doc, "k0", field, value);
            a.doc.merge(&mut b.doc).unwrap();
            assert!(conflicting(&column(&a.doc, "c0").unwrap().life));
            for key in ["k0", "k1"] {
                assert_eq!(location(&a.doc, key, "c0"), "conflicts");
            }
            assert_eq!(location(&a.doc, "old", "c0"), "trash");
            let observed = column(&a.doc, "c0").unwrap();
            for key in ["k0", "k1"] {
                for (f, v) in FIELDS.into_iter().zip(INITIAL) {
                    let read = card(&a.doc, key).unwrap();
                    let before = a.fingerprint().unwrap();
                    assert!(edit(&mut a.doc, key, &read, f, v, false).is_err());
                    assert_eq!(a.fingerprint().unwrap(), before);
                }
            }
            assert!(create(&mut a.doc, "new-a", "c0").is_err());
            // Another column remains usable and does not invalidate this scope.
            change(&mut a.doc, "other", "text", "Независимая колонка");
            set_column(&mut a.doc, "c0", &observed, state).unwrap();
            let selected = column(&a.doc, "c0").unwrap();
            assert_eq!(selected.id, initial.id);
            assert_eq!(selected.cards["old"], initial.cards["old"]);
            assert_eq!(selected.cards["k0"].id, initial.cards["k0"].id);
            for f in FIELDS {
                assert_eq!(
                    selected.cards["k0"].fields[f][0].value,
                    if f == field {
                        value
                    } else {
                        &initial.cards["k0"].fields[f][0].value
                    }
                );
            }
            assert_eq!(
                location(&a.doc, "k0", "c0"),
                if state == "active" { "board" } else { "trash" }
            );
            if state == "trash" {
                choose(&mut a.doc, "active");
            }
            assert_eq!(location(&a.doc, "k0", "c0"), "board");
            assert_eq!(location(&a.doc, "old", "c0"), "trash");
            store.write("A", &a).unwrap();
            exchange(&mut store).unwrap();
            assert_eq!(
                snapshot(&store.read("A").unwrap().doc).unwrap(),
                snapshot(&store.read("B").unwrap().doc).unwrap()
            );
        }
    }
}

#[test]
fn new_cards_are_absorbed_into_the_original_deletion_group_and_restore_together() {
    for state in ["active", "trash"] {
        let (_dir, store) = prepared(false);
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        choose(&mut a.doc, "trash");
        let group = column(&a.doc, "c0").unwrap().cycles;
        create(&mut b.doc, "new-a", "c0").unwrap();
        let id = card(&b.doc, "new-a").unwrap().id;
        a.doc.merge(&mut b.doc).unwrap();
        assert_eq!(location(&a.doc, "new-a", "c0"), "conflicts");
        choose(&mut a.doc, state);
        assert_eq!(column(&a.doc, "c0").unwrap().cycles, group);
        if state == "trash" {
            let col = column(&a.doc, "c0").unwrap();
            assert!(col.member(&col.cards["new-a"]));
            choose(&mut a.doc, "active");
        }
        assert_eq!(card(&a.doc, "new-a").unwrap().id, id);
        for key in ["k0", "k1", "new-a"] {
            assert_eq!(location(&a.doc, key, "c0"), "board");
        }
        assert_eq!(location(&a.doc, "old", "c0"), "trash");
    }
}

#[test]
fn restoration_to_another_column_detaches_only_the_selected_card() {
    let (_dir, mut store) = prepared(true);
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, "trash");
    for key in ["old", "k0"] {
        let observed = card(&a.doc, key).unwrap();
        let before = a.fingerprint().unwrap();
        assert!(set_card(&mut a.doc, key, &observed, "active", "c0").is_err());
        assert!(set_card(&mut a.doc, key, &observed, "active", "foreign-space").is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
        set_card(&mut a.doc, key, &observed, "active", "c1").unwrap();
        assert_eq!(location(&a.doc, key, "c1"), "board");
        assert_eq!(card(&a.doc, key).unwrap().id, observed.id);
        for field in ["title", "text", "completed", "tag"] {
            assert_eq!(
                card(&a.doc, key).unwrap().fields[field],
                observed.fields[field]
            );
        }
    }
    assert_eq!(column(&a.doc, "c0").unwrap().life[0].value, "trash");
    assert_eq!(location(&a.doc, "k1", "c0"), "trash");
    choose(&mut a.doc, "active");
    choose(&mut a.doc, "trash");
    choose(&mut a.doc, "active");
    assert_eq!(location(&a.doc, "k0", "c1"), "board");
    assert_eq!(location(&a.doc, "old", "c1"), "board");
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    assert_eq!(
        snapshot(&store.read("A").unwrap().doc).unwrap(),
        snapshot(&store.read("B").unwrap().doc).unwrap()
    );
}

#[test]
fn repeated_cycles_do_not_capture_earlier_or_independently_deleted_cards() {
    let (_dir, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, "trash");
    let first = column(&a.doc, "c0").unwrap().cycles;
    choose(&mut a.doc, "active");
    let observed = card(&a.doc, "k0").unwrap();
    set_card(&mut a.doc, "k0", &observed, "trash", "").unwrap();
    choose(&mut a.doc, "trash");
    let col = column(&a.doc, "c0").unwrap();
    assert_ne!(col.cycles, first);
    assert!(!col.member(&col.cards["k0"]));
    assert!(!col.member(&col.cards["old"]));
    assert!(col.member(&col.cards["k1"]));
    choose(&mut a.doc, "active");
    assert_eq!(location(&a.doc, "k0", "c0"), "trash");
    assert_eq!(location(&a.doc, "old", "c0"), "trash");
    assert_eq!(location(&a.doc, "k1", "c0"), "board");
}

#[test]
fn equal_different_and_late_choices_do_not_lose_group_members_or_edits() {
    for equal in [false, true] {
        let (_dir, store) = prepared(true);
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        let mut late = store.read("B").unwrap();
        late.doc = late.doc.fork();
        choose(&mut a.doc, "active");
        choose(&mut b.doc, if equal { "active" } else { "trash" });
        change(&mut a.doc, "new-a", "title", "После решения A");
        a.doc.merge(&mut b.doc).unwrap();
        assert_eq!(conflicting(&column(&a.doc, "c0").unwrap().life), !equal);
        let stale = column(&a.doc, "c0").unwrap();
        choose(&mut late.doc, "active");
        create(&mut late.doc, "new-b", "c0").unwrap();
        change(&mut late.doc, "k1", "text", "Поздняя правка");
        a.doc.merge(&mut late.doc).unwrap();
        let before = a.fingerprint().unwrap();
        assert!(set_column(&mut a.doc, "c0", &stale, "trash").is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
        choose(&mut a.doc, "trash");
        let col = column(&a.doc, "c0").unwrap();
        for key in ["k0", "k1", "new-a", "new-b"] {
            assert!(col.member(&col.cards[key]));
        }
        choose(&mut a.doc, "active");
        assert_eq!(
            card(&a.doc, "new-a").unwrap().fields["title"][0].value,
            "После решения A"
        );
        assert_eq!(
            card(&a.doc, "k1").unwrap().fields["text"][0].value,
            "Поздняя правка"
        );
        assert_eq!(location(&a.doc, "old", "c0"), "trash");
    }
    // Independent ordinary deletions have different group IDs but the same outcome.
    let (_dir, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    choose(&mut a.doc, "trash");
    choose(&mut b.doc, "trash");
    a.doc.merge(&mut b.doc).unwrap();
    assert!(!conflicting(&column(&a.doc, "c0").unwrap().life));
    choose(&mut a.doc, "active");
    assert_eq!(location(&a.doc, "old", "c0"), "trash");
    assert_eq!(location(&a.doc, "k0", "c0"), "board");
}

#[test]
fn own_card_conflicts_remain_independent_in_both_resolution_orders() {
    for first in [false, true] {
        for state in ["active", "trash"] {
            let (_dir, store) = prepared(false);
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            change(&mut a.doc, "k0", "text", "A");
            choose(&mut a.doc, "trash");
            change(&mut b.doc, "k0", "text", "B");
            a.doc.merge(&mut b.doc).unwrap();
            if first {
                choose(&mut a.doc, state);
            }
            let before = column(&a.doc, "c0").unwrap().life;
            let observed = card(&a.doc, "k0").unwrap();
            assert!(observed.blocked());
            edit(&mut a.doc, "k0", &observed, "text", "Объединение", true).unwrap();
            if !first || state == "trash" {
                assert_eq!(column(&a.doc, "c0").unwrap().life, before);
            }
            if !first {
                choose(&mut a.doc, state);
            }
            assert_eq!(
                location(&a.doc, "k0", "c0"),
                if state == "active" { "board" } else { "trash" }
            );
            assert_eq!(
                card(&a.doc, "k0").unwrap().fields["text"][0].value,
                "Объединение"
            );
        }
    }
}

#[test]
fn stale_scope_foreign_and_invalid_commands_leave_document_unchanged() {
    for action in [
        "create",
        "card",
        "old",
        "column-name",
        "column-order",
        "membership",
    ] {
        let (_dir, store) = prepared(false);
        let mut a = store.read("A").unwrap();
        let observed = column(&a.doc, "c0").unwrap();
        match action {
            "create" => create(&mut a.doc, "new-a", "c0").unwrap(),
            "card" => change(&mut a.doc, "k0", "text", "Свежее"),
            "old" => {
                let c = card(&a.doc, "old").unwrap();
                set_card(&mut a.doc, "old", &c, "active", "c0").unwrap();
            }
            "membership" => {
                let c = card(&a.doc, "k0").unwrap();
                set_card(&mut a.doc, "k0", &c, "trash", "").unwrap();
            }
            field => {
                let mut tx = a.doc.transaction();
                put(
                    &mut tx,
                    &observed.id,
                    if field == "column-name" {
                        "name"
                    } else {
                        "order"
                    },
                    "changed",
                )
                .unwrap();
                tx.commit();
            }
        }
        let before = a.fingerprint().unwrap();
        assert!(set_column(&mut a.doc, "c0", &observed, "trash").is_err());
        assert!(set_column(&mut a.doc, "c1", &observed, "trash").is_err());
        let fresh = column(&a.doc, "c0").unwrap();
        assert!(set_column(&mut a.doc, "c0", &fresh, "purged").is_err());
        assert!(set_column(&mut a.doc, "c0", &fresh, "active").is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
        set_column(&mut a.doc, "c0", &fresh, "trash").unwrap();
        let before = a.fingerprint().unwrap();
        let c = card(&a.doc, "k0").unwrap();
        for (field, value) in FIELDS.into_iter().zip(INITIAL) {
            assert!(edit(&mut a.doc, "k0", &c, field, value, false).is_err());
        }
        assert!(create(&mut a.doc, "new-b", "c0").is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
    }
}

#[test]
fn unseen_old_edit_reopens_deletion_but_replayed_history_does_not() {
    let (_dir, mut store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let mut old = store.read("B").unwrap();
    choose(&mut a.doc, "trash");
    choose(&mut a.doc, "active");
    choose(&mut a.doc, "trash");
    change(&mut old.doc, "k0", "text", "Не виденная прежде правка");
    create(&mut old.doc, "new-a", "c0").unwrap();
    a.doc.merge(&mut old.doc).unwrap();
    assert_eq!(location(&a.doc, "new-a", "c0"), "conflicts");
    choose(&mut a.doc, "trash");
    let expected = snapshot(&a.doc).unwrap();
    let changes = a.doc.get_changes(&[]);
    for reverse in [false, true] {
        let mut delivery = changes.clone();
        if reverse {
            delivery.reverse();
        }
        delivery.rotate_left(changes.len() / 2);
        let mut fresh = Replica::new();
        for change in delivery {
            fresh.doc.apply_changes([change.clone(), change]).unwrap();
        }
        fresh.doc.apply_changes(old.doc.get_changes(&[])).unwrap();
        assert_eq!(fresh.doc.get_heads(), a.doc.get_heads());
        assert_eq!(fresh.doc.get_changes(&[]).len(), changes.len());
        assert_eq!(snapshot(&fresh.doc).unwrap(), expected);
        store.write("A", &fresh).unwrap();
        assert_eq!(snapshot(&store.read("A").unwrap().doc).unwrap(), expected);
    }
}

#[test]
fn sqlite_abort_and_each_single_change_cover_all_mutations_without_partial_groups() {
    for action in [
        "trash",
        "restore",
        "choose-trash",
        "choose-active",
        "edit",
        "create",
        "restore-card",
        "trash-card",
        "resolve-field",
    ] {
        let (dir, mut store) = prepared(action.starts_with("choose"));
        let mut a = store.read("A").unwrap();
        if ["restore", "restore-card"].contains(&action) {
            choose(&mut a.doc, "trash");
            store.write("A", &a).unwrap();
            exchange(&mut store).unwrap();
            a = store.read("A").unwrap();
        }
        if action == "resolve-field" {
            let mut b = store.read("B").unwrap();
            change(&mut a.doc, "k0", "text", "A");
            change(&mut b.doc, "k0", "text", "B");
            a.doc.merge(&mut b.doc).unwrap();
            store.write("A", &a).unwrap();
        }
        let before = [
            store.read("A").unwrap().fingerprint().unwrap(),
            store.read("B").unwrap().fingerprint().unwrap(),
        ];
        let count = a.doc.get_changes(&[]).len();
        match action {
            "trash" | "choose-trash" => choose(&mut a.doc, "trash"),
            "restore" | "choose-active" => choose(&mut a.doc, "active"),
            "create" => create(&mut a.doc, "new-b", "c0").unwrap(),
            "restore-card" | "trash-card" => {
                let c = card(&a.doc, "k0").unwrap();
                set_card(
                    &mut a.doc,
                    "k0",
                    &c,
                    if action == "restore-card" {
                        "active"
                    } else {
                        "trash"
                    },
                    "c1",
                )
                .unwrap();
            }
            "resolve-field" => {
                let c = card(&a.doc, "k0").unwrap();
                edit(&mut a.doc, "k0", &c, "text", "Решено", true).unwrap();
            }
            _ => change(&mut a.doc, "k0", "text", "Не сохранять"),
        }
        assert_eq!(a.doc.get_changes(&[]).len(), count + 1);
        store.0.execute_batch("CREATE TRIGGER reject_column AFTER UPDATE ON replicas BEGIN SELECT RAISE(ABORT, 'synthetic column failure'); END;").unwrap();
        assert!(store.write("A", &a).is_err());
        drop(store);
        let store = Store::open(&dir.path().join("column.sqlite3")).unwrap();
        for (id, expected) in ["A", "B"].into_iter().zip(before) {
            assert_eq!(store.read(id).unwrap().fingerprint().unwrap(), expected);
        }
    }
}

#[test]
fn native_phases_cover_both_choices_and_refuse_overwriting_fixture() {
    let directory = tempfile::tempdir().unwrap();
    for phase in [
        "column-trash-prepare",
        "column-trash-resolve",
        "column-trash-resume",
    ] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "column-trash-prepare").is_err());
}

#[test]
fn column_choices_preserve_own_name_and_order_conflicts_and_individual_card_deletions() {
    for field in ["name", "order"] {
        for state in ["active", "trash"] {
            let (_dir, store) = prepared(false);
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            let id = column(&a.doc, "c0").unwrap().id;
            // Same immutable register writes as the existing container/order probes.
            for (doc, value) in [(&mut a.doc, "A"), (&mut b.doc, "B")] {
                let mut tx = doc.transaction();
                put(&mut tx, &id, field, value).unwrap();
                tx.commit();
            }
            choose(&mut a.doc, "trash");
            change(&mut b.doc, "k0", "text", "Сохранить правку");
            let observed = card(&b.doc, "k1").unwrap();
            set_card(&mut b.doc, "k1", &observed, "trash", "").unwrap();
            a.doc.merge(&mut b.doc).unwrap();
            let before = column(&a.doc, "c0").unwrap();
            assert!(conflicting(if field == "name" {
                &before.name
            } else {
                &before.order
            }));
            choose(&mut a.doc, state);
            let after = column(&a.doc, "c0").unwrap();
            assert_eq!(before.name, after.name);
            assert_eq!(before.order, after.order);
            assert_eq!(before.cards["k1"].life, after.cards["k1"].life);
            assert_eq!(location(&a.doc, "k1", "c0"), "trash");
            assert_eq!(location(&a.doc, "old", "c0"), "trash");
            if state == "active" {
                // Name/order conflict blocks the column, not its unconflicted cards.
                change(&mut a.doc, "k0", "text", "Доступно содержимое");
            }
        }
    }
}

#[test]
fn unchanged_active_scope_ignores_earlier_trash_and_invalid_commands_do_not_write() {
    let (_dir, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let observed = column(&a.doc, "c0").unwrap();
    // Old trash is outside an ordinary column deletion's active scope (ADR-0014).
    let old = card(&a.doc, "old").unwrap();
    let mut tx = a.doc.transaction();
    put(&mut tx, &old.id, "text", "Данные прежней корзины").unwrap();
    tx.commit();
    let before = a.fingerprint().unwrap();
    let current = card(&a.doc, "k0").unwrap();
    for (field, value) in [
        ("title", " "),
        ("completed", "yes"),
        ("tag", "unknown"),
        ("place", "foreign:100"),
        ("unknown", "x"),
    ] {
        assert!(edit(&mut a.doc, "k0", &current, field, value, false).is_err());
    }
    assert!(edit(&mut a.doc, "k0", &current, "text", "x", true).is_err());
    assert!(edit(&mut a.doc, "k1", &current, "text", "x", false).is_err());
    assert!(create(&mut a.doc, "k0", "c0").is_err());
    assert!(create(&mut a.doc, "new-a", "foreign").is_err());
    assert!(set_card(&mut a.doc, "k1", &current, "trash", "").is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    set_column(&mut a.doc, "c0", &observed, "trash").unwrap();
    choose(&mut a.doc, "active");
    assert_eq!(
        card(&a.doc, "old").unwrap().fields["text"][0].value,
        "Данные прежней корзины"
    );
    assert_eq!(location(&a.doc, "old", "c0"), "trash");
}
