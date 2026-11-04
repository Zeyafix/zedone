use super::{
    scenario::{change, choose},
    *,
};

fn prepared(conflict: bool) -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("space.sqlite3")).unwrap();
    if conflict {
        scenario::prepare(&mut store).unwrap();
    } else {
        seed(&mut store).unwrap();
    }
    (dir, store)
}

fn assert_location(doc: &Automerge, keys: &[&str], state: &str) {
    let nodes = view(doc).unwrap();
    for key in keys {
        assert_eq!(location(&nodes, key), state, "{key}");
    }
}

fn group_member(nodes: &View, key: &str, parent: &str) -> bool {
    let values = if nodes[parent].kind == "space" {
        &nodes[key].space_group
    } else {
        &nodes[key].column_group
    };
    values
        .iter()
        .any(|g| !g.value.is_empty() && nodes[parent].cycles.iter().any(|c| g.value == c.value))
}

#[test]
fn every_independent_edit_preserves_both_choices_ids_and_prior_nested_trash() {
    let edits: Vec<_> = CARD_FIELDS
        .into_iter()
        .zip(scenario::EDITED)
        .map(|((f, _), v)| ("k0", f, v))
        .chain([
            ("s0", "name", "Новое пространство"),
            ("c0", "name", "Новая колонка"),
            ("c0", "order", "200"),
            ("tag", "name", "Новый тег"),
        ])
        .collect();
    for (key, field, value) in edits {
        for state in ["active", "trash"] {
            let (_dir, mut store) = prepared(false);
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            let initial = view(&a.doc).unwrap();
            choose(&mut a.doc, "s0", "trash").unwrap();
            change(&mut b.doc, key, field, value).unwrap();
            a.doc.merge(&mut b.doc).unwrap();
            assert_location(
                &a.doc,
                &["s0", "c0", "k0", "k1", "c1", "other"],
                "conflicts",
            );
            let observed = scope(&view(&a.doc).unwrap(), "s0");
            // An unrelated space stays usable without invalidating this choice.
            change(&mut a.doc, "foreign", "text", "Другое пространство").unwrap();
            set_container(&mut a.doc, "s0", &observed, state).unwrap();
            let selected = view(&a.doc).unwrap();
            assert_eq!(selected[key].fields[field][0].value, value);
            for k in initial.keys() {
                assert_eq!(selected[k].id, initial[k].id);
            }
            for k in ["old", "old-col", "old-col-card"] {
                assert_eq!(selected[k], initial[k]);
            }
            if state == "trash" {
                choose(&mut a.doc, "s0", "active").unwrap();
            }
            assert_location(&a.doc, &["c0", "k0", "k1", "other"], "board");
            assert_location(&a.doc, &["old", "old-col", "old-col-card"], "trash");
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
fn new_columns_and_cards_join_original_space_group_and_restore_together() {
    for creation in ["column", "card", "both"] {
        for state in ["active", "trash"] {
            let (_dir, store) = prepared(false);
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            choose(&mut a.doc, "s0", "trash").unwrap();
            let cycles = view(&a.doc).unwrap()["s0"].cycles.clone();
            let mut new_keys = vec![];
            if creation != "card" {
                create(&mut b.doc, "new-col", "s0").unwrap();
                create(&mut b.doc, "new-a", "new-col").unwrap();
                new_keys.extend(["new-col", "new-a"]);
            }
            if creation != "column" {
                create(&mut b.doc, "new-b", "c0").unwrap();
                new_keys.push("new-b");
            }
            let created = view(&b.doc).unwrap();
            a.doc.merge(&mut b.doc).unwrap();
            assert_location(&a.doc, &new_keys, "conflicts");
            choose(&mut a.doc, "s0", state).unwrap();
            let nodes = view(&a.doc).unwrap();
            assert_eq!(nodes["s0"].cycles, cycles);
            for key in &new_keys {
                assert_eq!(nodes[*key].id, created[*key].id);
                if state == "trash" {
                    assert!(group_member(&nodes, key, "s0"));
                }
            }
            if state == "trash" {
                choose(&mut a.doc, "s0", "active").unwrap();
            }
            assert_location(&a.doc, &new_keys, "board");
        }
    }
}

#[test]
fn nested_and_repeated_deletion_cycles_keep_independent_membership() {
    let (_dir, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let prior = view(&a.doc).unwrap();
    choose(&mut a.doc, "s0", "trash").unwrap();
    let first = view(&a.doc).unwrap()["s0"].cycles.clone();
    choose(&mut a.doc, "s0", "active").unwrap();
    let nodes = view(&a.doc).unwrap();
    set_card(&mut a.doc, "k0", &nodes["k0"], "trash", "").unwrap();
    choose(&mut a.doc, "c0", "trash").unwrap();
    let nested = view(&a.doc).unwrap();
    assert!(group_member(&nested, "k1", "c0"));
    assert!(!group_member(&nested, "k0", "c0"));
    choose(&mut a.doc, "s0", "trash").unwrap();
    let second = view(&a.doc).unwrap();
    assert_ne!(second["s0"].cycles, first);
    for key in ["c0", "k0", "k1", "old"] {
        assert!(!group_member(&second, key, "s0"));
    }
    assert!(group_member(&second, "c1", "s0"));
    choose(&mut a.doc, "s0", "active").unwrap();
    assert_location(&a.doc, &["c0", "k0", "k1", "old-col-card"], "trash");
    choose(&mut a.doc, "c0", "active").unwrap();
    assert_location(&a.doc, &["k1"], "board");
    assert_location(&a.doc, &["k0", "old", "old-col-card"], "trash");
    let after = view(&a.doc).unwrap();
    for key in ["old-col", "old-col-card"] {
        assert_eq!(after[key], prior[key]);
    }
}

#[test]
fn individual_recovery_requires_entire_space_then_only_selected_card_detaches() {
    let (_dir, mut store) = prepared(true);
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, "s0", "trash").unwrap();
    let nodes = view(&a.doc).unwrap();
    let before = a.fingerprint().unwrap();
    for key in ["old", "k0", "old-col-card"] {
        assert!(set_card(&mut a.doc, key, &nodes[key], "active", "c1").is_err());
    }
    assert!(choose(&mut a.doc, "old-col", "active").is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    choose(&mut a.doc, "s0", "active").unwrap();
    assert_location(&a.doc, &["k0", "k1", "new-a", "new-b"], "board");
    assert_location(&a.doc, &["old", "old-col-card"], "trash");
    for key in ["old", "old-col-card"] {
        let old = view(&a.doc).unwrap()[key].clone();
        let before = a.fingerprint().unwrap();
        for target in ["old-col", "foreign-col", "s0", "unknown"] {
            assert!(set_card(&mut a.doc, key, &old, "active", target).is_err());
        }
        assert_eq!(a.fingerprint().unwrap(), before);
        set_card(&mut a.doc, key, &old, "active", "c1").unwrap();
        let restored = view(&a.doc).unwrap()[key].clone();
        assert_eq!(restored.id, old.id);
        for field in ["title", "text", "completed", "tag"] {
            assert_eq!(restored.fields[field], old.fields[field]);
        }
        assert!(restored.space_group.iter().all(|v| v.value.is_empty()));
        assert!(restored.column_group.iter().all(|v| v.value.is_empty()));
    }
    choose(&mut a.doc, "old-col", "active").unwrap();
    choose(&mut a.doc, "old-col", "trash").unwrap();
    assert_location(&a.doc, &["old", "old-col-card"], "board");
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    assert_location(
        &store.read("B").unwrap().doc,
        &["old", "old-col-card"],
        "board",
    );
}

#[test]
fn equal_different_late_choices_and_concurrent_deletions_preserve_all_variants() {
    for equal in [true, false] {
        let (_dir, store) = prepared(true);
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        let mut late = b.doc.fork();
        choose(&mut a.doc, "s0", "active").unwrap();
        choose(&mut b.doc, "s0", if equal { "active" } else { "trash" }).unwrap();
        change(&mut a.doc, "k0", "text", "После решения A").unwrap();
        a.doc.merge(&mut b.doc).unwrap();
        assert_eq!(conflicting(&view(&a.doc).unwrap()["s0"].life), !equal);
        let stale = scope(&view(&a.doc).unwrap(), "s0");
        choose(&mut late, "s0", "active").unwrap();
        create(&mut late, "late-col", "s0").unwrap();
        change(&mut late, "k1", "text", "Поздняя правка").unwrap();
        a.doc.merge(&mut late).unwrap();
        let before = a.fingerprint().unwrap();
        assert!(set_container(&mut a.doc, "s0", &stale, "trash").is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
        choose(&mut a.doc, "s0", "trash").unwrap();
        assert!(group_member(&view(&a.doc).unwrap(), "late-col", "s0"));
        choose(&mut a.doc, "s0", "active").unwrap();
        let nodes = view(&a.doc).unwrap();
        assert_eq!(nodes["k0"].fields["text"][0].value, "После решения A");
        assert_eq!(nodes["k1"].fields["text"][0].value, "Поздняя правка");
    }
    let (_dir, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    choose(&mut a.doc, "s0", "trash").unwrap();
    choose(&mut b.doc, "s0", "trash").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let nodes = view(&a.doc).unwrap();
    assert!(!conflicting(&nodes["s0"].life));
    assert_eq!(nodes["s0"].cycles.len(), 2);
    assert_eq!(nodes["k0"].space_group.len(), 2);
    choose(&mut a.doc, "s0", "active").unwrap();
    assert_location(&a.doc, &["k0", "k1", "other"], "board");
    assert_location(&a.doc, &["old", "old-col-card"], "trash");
}

#[test]
fn independent_column_and_card_deletions_survive_space_choices() {
    for target in ["card", "column"] {
        for state in ["active", "trash"] {
            let (_dir, store) = prepared(false);
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            choose(&mut a.doc, "s0", "trash").unwrap();
            if target == "card" {
                let observed = view(&b.doc).unwrap()["k0"].clone();
                set_card(&mut b.doc, "k0", &observed, "trash", "").unwrap();
            } else {
                choose(&mut b.doc, "c0", "trash").unwrap();
            }
            a.doc.merge(&mut b.doc).unwrap();
            assert!(conflicting(&view(&a.doc).unwrap()["s0"].life));
            let before = view(&a.doc).unwrap();
            choose(&mut a.doc, "s0", state).unwrap();
            if state == "trash" {
                choose(&mut a.doc, "s0", "active").unwrap();
            }
            let after = view(&a.doc).unwrap();
            assert_eq!(before["k0"].life, after["k0"].life);
            assert_eq!(before["c0"].life, after["c0"].life);
            assert_location(&a.doc, &["k0"], "trash");
            assert_location(
                &a.doc,
                &["k1"],
                if target == "card" { "board" } else { "trash" },
            );
        }
    }
}

#[test]
fn field_conflicts_can_be_chosen_in_either_order_without_restoring_ancestors() {
    for (key, field) in [
        ("k0", "text"),
        ("s0", "name"),
        ("c0", "name"),
        ("c0", "order"),
    ] {
        for first in [true, false] {
            for state in ["active", "trash"] {
                let (_dir, store) = prepared(false);
                let mut a = store.read("A").unwrap();
                let mut b = store.read("B").unwrap();
                let (left, right) = if field == "order" {
                    ("100", "200")
                } else {
                    ("A", "B")
                };
                change(&mut a.doc, key, field, left).unwrap();
                choose(&mut a.doc, "s0", "trash").unwrap();
                change(&mut b.doc, key, field, right).unwrap();
                a.doc.merge(&mut b.doc).unwrap();
                if first {
                    choose(&mut a.doc, "s0", state).unwrap();
                }
                let before = view(&a.doc).unwrap();
                assert!(before[key].blocked());
                edit(&mut a.doc, key, &before[key], field, left, true).unwrap();
                if !first || state == "trash" {
                    assert_eq!(view(&a.doc).unwrap()["s0"].life, before["s0"].life);
                }
                if !first {
                    choose(&mut a.doc, "s0", state).unwrap();
                }
                assert_location(
                    &a.doc,
                    &["k0"],
                    if state == "active" { "board" } else { "trash" },
                );
                assert_eq!(view(&a.doc).unwrap()[key].fields[field][0].value, left);
            }
        }
    }
}

#[test]
fn strict_space_scope_detects_data_catalog_and_membership_changes_without_partial_write() {
    for action in [
        "name",
        "column-name",
        "column-order",
        "title",
        "text",
        "completed",
        "place",
        "tag",
        "tag-name",
        "new-card",
        "new-column",
        "delete-card",
        "delete-column",
        "restore-card",
        "restore-column",
        "old-data",
        "old-column-data",
        "tag-set",
        "same-count",
    ] {
        let (_dir, store) = prepared(false);
        let mut a = store.read("A").unwrap();
        let nodes = view(&a.doc).unwrap();
        let observed = scope(&nodes, "s0");
        match action {
            "name" => change(&mut a.doc, "s0", "name", "Changed").unwrap(),
            "column-name" => change(&mut a.doc, "c0", "name", "Changed").unwrap(),
            "column-order" => change(&mut a.doc, "c0", "order", "200").unwrap(),
            "tag-name" => change(&mut a.doc, "tag", "name", "Changed").unwrap(),
            "new-card" => create(&mut a.doc, "new-a", "c0").unwrap(),
            "new-column" => create(&mut a.doc, "new-col", "s0").unwrap(),
            "delete-card" => set_card(&mut a.doc, "k0", &nodes["k0"], "trash", "").unwrap(),
            "delete-column" => choose(&mut a.doc, "c0", "trash").unwrap(),
            "restore-card" => set_card(&mut a.doc, "old", &nodes["old"], "active", "c0").unwrap(),
            "restore-column" => choose(&mut a.doc, "old-col", "active").unwrap(),
            "old-data" | "old-column-data" | "tag-set" | "same-count" => {
                let mut tx = a.doc.transaction();
                if action == "same-count" {
                    put(&mut tx, &nodes["k0"].id, "place", "c1:100").unwrap();
                    put(&mut tx, &nodes["other"].id, "place", "c0:100").unwrap();
                } else {
                    let (key, field, value) = match action {
                        "old-data" => ("old", "text", "Changed trash"),
                        "old-column-data" => ("old-col", "name", "Changed trash"),
                        _ => ("tag", "life", "deleted"),
                    };
                    put(&mut tx, &nodes[key].id, field, value).unwrap();
                }
                tx.commit();
            }
            field => {
                let value =
                    scenario::EDITED[CARD_FIELDS.iter().position(|(f, _)| *f == field).unwrap()];
                change(&mut a.doc, "k0", field, value).unwrap();
            }
        }
        let before = a.fingerprint().unwrap();
        assert!(
            set_container(&mut a.doc, "s0", &observed, "trash").is_err(),
            "{action}"
        );
        assert_eq!(a.fingerprint().unwrap(), before, "{action}");
    }
}

#[test]
fn ordinary_actions_are_blocked_in_trash_and_conflicts_and_invalid_commands_do_not_write() {
    for conflict in [false, true] {
        let (_dir, store) = prepared(conflict);
        let mut a = store.read("A").unwrap();
        if !conflict {
            choose(&mut a.doc, "s0", "trash").unwrap();
        }
        let nodes = view(&a.doc).unwrap();
        let before = a.fingerprint().unwrap();
        for (key, field, value) in [
            ("s0", "name", "s0"),
            ("c0", "name", "c0"),
            ("c0", "order", "100"),
            ("tag", "name", "tag"),
        ] {
            assert!(edit(&mut a.doc, key, &nodes[key], field, value, false).is_err());
        }
        for (field, value) in CARD_FIELDS {
            assert!(edit(&mut a.doc, "k0", &nodes["k0"], field, value, false).is_err());
        }
        for (key, parent) in [("late-col", "s0"), ("new-b", "c0")] {
            assert!(create(&mut a.doc, key, parent).is_err());
        }
        assert!(set_card(&mut a.doc, "old", &nodes["old"], "active", "foreign-col").is_err());
        assert!(choose(&mut a.doc, "c0", "trash").is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
    }
    let (_dir, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let nodes = view(&a.doc).unwrap();
    let before = a.fingerprint().unwrap();
    for (field, value) in [
        ("title", " "),
        ("completed", "yes"),
        ("tag", "?"),
        ("place", "foreign-col:100"),
        ("unknown", "x"),
    ] {
        assert!(edit(&mut a.doc, "k0", &nodes["k0"], field, value, false).is_err());
    }
    assert!(edit(&mut a.doc, "k1", &nodes["k0"], "text", "x", false).is_err());
    assert!(edit(&mut a.doc, "k0", &nodes["k0"], "text", "x", true).is_err());
    assert!(create(&mut a.doc, "k0", "c0").is_err());
    assert!(create(&mut a.doc, "new-a", "s0").is_err());
    assert!(create(&mut a.doc, "new-a", "unknown").is_err());
    assert!(set_container(&mut a.doc, "s1", &scope(&nodes, "s0"), "trash").is_err());
    assert!(choose(&mut a.doc, "s0", "purged").is_err());
    assert!(choose(&mut a.doc, "tag", "trash").is_err());
    assert!(choose(&mut a.doc, "s0", "active").is_err());
    assert!(set_card(&mut a.doc, "k1", &nodes["k0"], "trash", "").is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
}

#[test]
fn old_unseen_edit_and_creation_conflict_but_repeated_known_changes_never_revive() {
    let (dir, mut store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let mut late = store.read("B").unwrap().doc.fork();
    let known = a.doc.get_changes(&[]);
    choose(&mut a.doc, "s0", "trash").unwrap();
    let deletion = a.doc.get_changes(&[]);
    change(&mut late, "k0", "text", "Старая реплика").unwrap();
    create(&mut late, "late-col", "s0").unwrap();
    create(&mut late, "new-b", "late-col").unwrap();
    a.doc.merge(&mut late).unwrap();
    assert_location(&a.doc, &["k0", "late-col", "new-b"], "conflicts");
    choose(&mut a.doc, "s0", "trash").unwrap();
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    drop(store);
    let mut store = Store::open(&dir.path().join("space.sqlite3")).unwrap();
    let mut a = store.read("A").unwrap();
    let expected = snapshot(&a.doc).unwrap();
    for changes in [known, deletion] {
        for change in changes.into_iter().rev() {
            a.doc.apply_changes([change.clone(), change]).unwrap();
        }
        assert_eq!(snapshot(&a.doc).unwrap(), expected);
    }
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    for id in ["A", "B"] {
        assert_location(
            &store.read(id).unwrap().doc,
            &["k0", "late-col", "new-b"],
            "trash",
        );
    }
}

#[test]
fn each_mutation_is_one_change_and_sqlite_abort_retains_both_replicas() {
    for action in [
        "trash",
        "restore",
        "choose-trash",
        "choose-active",
        "edit",
        "column-edit",
        "space-edit",
        "tag-edit",
        "create-column",
        "create-card",
        "delete-column",
        "restore-column",
        "trash-card",
        "restore-card",
        "resolve-field",
    ] {
        let (dir, mut store) = prepared(action.starts_with("choose"));
        let mut a = store.read("A").unwrap();
        if action == "restore" {
            choose(&mut a.doc, "s0", "trash").unwrap();
        }
        if action == "resolve-field" {
            let mut b = store.read("B").unwrap();
            change(&mut a.doc, "k0", "text", "A").unwrap();
            change(&mut b.doc, "k0", "text", "B").unwrap();
            a.doc.merge(&mut b.doc).unwrap();
        }
        store.write("A", &a).unwrap();
        let before = [
            store.read("A").unwrap().fingerprint().unwrap(),
            store.read("B").unwrap().fingerprint().unwrap(),
        ];
        let count = a.doc.get_changes(&[]).len();
        match action {
            "trash" | "choose-trash" => choose(&mut a.doc, "s0", "trash").unwrap(),
            "restore" | "choose-active" => choose(&mut a.doc, "s0", "active").unwrap(),
            "space-edit" => change(&mut a.doc, "s0", "name", "Updated").unwrap(),
            "column-edit" => change(&mut a.doc, "c0", "name", "Updated").unwrap(),
            "tag-edit" => change(&mut a.doc, "tag", "name", "Updated").unwrap(),
            "edit" => change(&mut a.doc, "k0", "text", "Updated").unwrap(),
            "create-column" => create(&mut a.doc, "late-col", "s0").unwrap(),
            "create-card" => create(&mut a.doc, "new-a", "c0").unwrap(),
            "delete-column" => choose(&mut a.doc, "c0", "trash").unwrap(),
            "restore-column" => choose(&mut a.doc, "old-col", "active").unwrap(),
            "resolve-field" => {
                let n = view(&a.doc).unwrap();
                edit(&mut a.doc, "k0", &n["k0"], "text", "Resolved", true).unwrap();
            }
            _ => {
                let n = view(&a.doc).unwrap();
                let key = if action == "trash-card" {
                    "k0"
                } else {
                    "old-col-card"
                };
                set_card(
                    &mut a.doc,
                    key,
                    &n[key],
                    if action == "trash-card" {
                        "trash"
                    } else {
                        "active"
                    },
                    "c1",
                )
                .unwrap();
            }
        }
        assert_eq!(a.doc.get_changes(&[]).len(), count + 1, "{action}");
        store.0.execute_batch("CREATE TRIGGER reject_space AFTER UPDATE ON replicas BEGIN SELECT RAISE(ABORT, 'synthetic space failure'); END;").unwrap();
        assert!(store.write("A", &a).is_err());
        drop(store);
        let store = Store::open(&dir.path().join("space.sqlite3")).unwrap();
        for (id, expected) in ["A", "B"].into_iter().zip(before) {
            assert_eq!(
                store.read(id).unwrap().fingerprint().unwrap(),
                expected,
                "{action}"
            );
        }
    }
}

#[test]
fn native_phases_check_both_choices_and_refuse_overwriting_existing_data() {
    let dir = tempfile::tempdir().unwrap();
    for phase in [
        "space-trash-prepare",
        "space-trash-resolve",
        "space-trash-resume",
    ] {
        run(dir.path(), phase).unwrap();
    }
    assert!(run(dir.path(), "space-trash-prepare").is_err());
}

#[test]
fn own_card_life_and_multiple_field_conflicts_are_not_chosen_by_space_resolution() {
    for state in ["active", "trash"] {
        let (_dir, store) = prepared(false);
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        let nodes = view(&a.doc).unwrap();
        set_card(&mut a.doc, "k0", &nodes["k0"], "trash", "").unwrap();
        choose(&mut a.doc, "s0", "trash").unwrap();
        change(&mut b.doc, "k0", "text", "Independent edit").unwrap();
        a.doc.merge(&mut b.doc).unwrap();
        let before = view(&a.doc).unwrap();
        assert!(conflicting(&before["k0"].life));
        choose(&mut a.doc, "s0", state).unwrap();
        assert_eq!(view(&a.doc).unwrap()["k0"].life, before["k0"].life);
        assert_location(&a.doc, &["k0"], "conflicts");
    }
    let (_dir, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    for (doc, value) in [(&mut a.doc, "A"), (&mut b.doc, "B")] {
        change(doc, "k0", "title", value).unwrap();
        change(doc, "k0", "text", value).unwrap();
    }
    choose(&mut a.doc, "s0", "trash").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    choose(&mut a.doc, "s0", "active").unwrap();
    let n = view(&a.doc).unwrap();
    edit(&mut a.doc, "k0", &n["k0"], "title", "Chosen", true).unwrap();
    let n = view(&a.doc).unwrap();
    assert!(n["k0"].blocked());
    let before = a.fingerprint().unwrap();
    assert!(edit(&mut a.doc, "k0", &n["k0"], "completed", "true", false).is_err());
    assert!(choose(&mut a.doc, "s0", "trash").is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    // A sibling's usable card remains available while k0 is conflicted.
    change(&mut a.doc, "k1", "text", "Allowed").unwrap();
}

#[test]
fn unchanged_scope_ignores_reads_peer_progress_and_other_space_but_keeps_name_blocking_local() {
    let (_dir, mut store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let observed = scope(&view(&a.doc).unwrap(), "s0");
    let _ = snapshot(&a.doc).unwrap();
    change(&mut a.doc, "foreign", "text", "Unrelated").unwrap();
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    set_container(&mut a.doc, "s0", &observed, "trash").unwrap();
    choose(&mut a.doc, "s0", "active").unwrap();
    let observed = scope(&view(&a.doc).unwrap(), "c0");
    let n = view(&a.doc).unwrap();
    let mut tx = a.doc.transaction();
    put(&mut tx, &n["old"].id, "text", "Earlier trash").unwrap();
    tx.commit();
    set_container(&mut a.doc, "c0", &observed, "trash").unwrap();
    choose(&mut a.doc, "c0", "active").unwrap();
    let mut b = a.doc.fork();
    change(&mut a.doc, "s0", "name", "A").unwrap();
    change(&mut b, "s0", "name", "B").unwrap();
    a.doc.merge(&mut b).unwrap();
    assert!(choose(&mut a.doc, "s0", "trash").is_err());
    change(&mut a.doc, "k0", "text", "Content remains available").unwrap();
    create(&mut a.doc, "new-col", "s0").unwrap();
}

#[test]
fn reordered_new_changes_reconstruct_same_groups_in_an_empty_document() {
    let (_dir, store) = prepared(true);
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, "s0", "trash").unwrap();
    let expected = snapshot(&a.doc).unwrap();
    let changes = a.doc.get_changes(&[]);
    let mut received = Automerge::new();
    for change in changes.into_iter().rev() {
        received.apply_changes([change.clone(), change]).unwrap();
    }
    assert_eq!(snapshot(&received).unwrap(), expected);
    let restored = Automerge::load(&received.save()).unwrap();
    assert_eq!(snapshot(&restored).unwrap(), expected);
}

#[test]
fn a_conflicted_card_in_an_independently_deleted_column_keeps_its_existing_groups() {
    let (_dir, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    let mut late = b.doc.fork();
    choose(&mut a.doc, "s0", "trash").unwrap();
    let n = view(&b.doc).unwrap();
    set_card(&mut b.doc, "k0", &n["k0"], "trash", "").unwrap();
    choose(&mut b.doc, "c0", "trash").unwrap();
    change(&mut late, "k0", "text", "Independent card edit").unwrap();
    b.doc.merge(&mut late).unwrap();
    choose(&mut b.doc, "c0", "trash").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let before = view(&a.doc).unwrap();
    assert!(conflicting(&before["k0"].life));
    assert_eq!(before["c0"].life[0].value, "trash");
    choose(&mut a.doc, "s0", "trash").unwrap();
    let after = view(&a.doc).unwrap();
    for key in ["c0", "k0", "k1"] {
        assert_eq!(after[key].space_group, before[key].space_group);
        assert_eq!(after[key].column_group, before[key].column_group);
    }
    choose(&mut a.doc, "s0", "active").unwrap();
    assert_location(&a.doc, &["c0", "k1"], "trash");
    assert_location(&a.doc, &["k0"], "conflicts");
}
