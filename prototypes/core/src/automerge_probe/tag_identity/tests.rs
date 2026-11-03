use super::*;

fn prepared() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("automerge-tag-identity.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    (directory, store)
}

fn add(store: &mut Store, peer: &str, home: &str, name: &str, card: &str) -> ObjId {
    let mut replica = store.read(peer).unwrap();
    let home = object(&replica.doc, home).unwrap().to_string();
    let id = create(&mut replica.doc, &home, name).unwrap();
    assign(&mut replica.doc, card, &id, true).unwrap();
    store.write(peer, &replica).unwrap();
    id
}

fn merged(store: &mut Store) -> (ObjId, ObjId) {
    let left = add(store, "A", "s0", "РАБОТА", "k0");
    let right = add(store, "B", "s0", "Работа", "k1");
    exchange(store).unwrap();
    (left, right)
}

fn assert_link(doc: &Automerge, card: &str, id: &ObjId, expected: &[&str]) {
    let mut values: Vec<_> = links(doc, &object(doc, card).unwrap(), &tag(doc, id).unwrap())
        .unwrap()
        .into_iter()
        .map(|v| v.value)
        .collect();
    values.sort();
    values.dedup();
    assert_eq!(values, expected);
}

#[test]
fn independent_creations_merge_only_equal_names_in_the_same_space() {
    for (left_name, right_name, home, count) in [
        ("РАБОТА", "Работа", "s0", 1),
        ("  ВСЁ  ", "Всё", "s0", 1),
        ("Все", "Всё", "s0", 2),
        ("Работа", "Работа", "s1", 2),
    ] {
        let (_directory, mut store) = prepared();
        let left = add(&mut store, "A", "s0", left_name, "k0");
        let right = add(
            &mut store,
            "B",
            home,
            right_name,
            if home == "s0" { "k1" } else { "k2" },
        );
        exchange(&mut store).unwrap();
        let a = store.read("A").unwrap();
        let groups = tags(&a.doc).unwrap();
        assert_eq!(groups.len(), count);
        let group = tag(&a.doc, &left).unwrap();
        assert_eq!(group.ids.len(), if count == 1 { 2 } else { 1 });
        assert!([left_name, right_name].contains(&group.name()));
        assert!(!group.conflicting());
        assert_link(&a.doc, "k0", &left, &["present"]);
        assert_link(
            &a.doc,
            if home == "s0" { "k1" } else { "k2" },
            &right,
            &["present"],
        );
        assert_eq!(
            snapshot(&a.doc).unwrap(),
            snapshot(&store.read("B").unwrap().doc).unwrap()
        );
    }
}

#[test]
fn merged_aliases_support_reverse_renames_and_memberships_without_rewriting_ids() {
    let (_directory, mut store) = prepared();
    let (left, right) = merged(&mut store);
    let mut a = store.read("A").unwrap();
    let ids = tag(&a.doc, &left).unwrap().ids;
    for (id, name) in [(&left, "ЛИЧНОЕ"), (&right, "личное"), (&left, "РАБОТА")] {
        rename(&mut a.doc, id, name).unwrap();
        assert_eq!(tag(&a.doc, &right).unwrap().name(), name);
        for card in ["k0", "k1"] {
            assign(&mut a.doc, card, id, false).unwrap();
            assert_link(&a.doc, card, id, &["absent"]);
            assign(&mut a.doc, card, id, true).unwrap();
            assert_link(&a.doc, card, id, &["present"]);
        }
        assert_eq!(tag(&a.doc, id).unwrap().ids, ids);
    }
    let before = a.fingerprint().unwrap();
    assign(&mut a.doc, "k0", &right, true).unwrap();
    assert_eq!(a.fingerprint().unwrap(), before);
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    assert_eq!(
        snapshot(&a.doc).unwrap(),
        snapshot(&store.read("B").unwrap().doc).unwrap()
    );
}

#[test]
fn vacated_birth_name_can_be_created_again_without_merging_causal_creations() {
    let (_directory, mut store) = prepared();
    let (left, right) = merged(&mut store);
    let mut a = store.read("A").unwrap();
    rename(&mut a.doc, &left, "Личное").unwrap();
    let home = object(&a.doc, "s0").unwrap().to_string();
    let new = create(&mut a.doc, &home, "РАБОТА").unwrap();
    assign(&mut a.doc, "k1", &new, true).unwrap();
    assert_eq!(tags(&a.doc).unwrap().len(), 2);
    assert_eq!(tag(&a.doc, &new).unwrap().ids, std::slice::from_ref(&new));
    assert_eq!(tag(&a.doc, &right).unwrap().ids.len(), 2);
    assert_link(&a.doc, "k0", &new, &[]);
    assert_link(&a.doc, "k1", &right, &["present"]);
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    assert_eq!(
        snapshot(&a.doc).unwrap(),
        snapshot(&store.read("B").unwrap().doc).unwrap()
    );
}

#[test]
fn invalid_creation_rename_and_assignment_do_not_mutate_the_document() {
    let (_directory, mut store) = prepared();
    let (left, _) = merged(&mut store);
    let mut a = store.read("A").unwrap();
    let home = object(&a.doc, "s0").unwrap().to_string();
    let other = create(&mut a.doc, &home, "Все").unwrap();
    create(&mut a.doc, &home, "Всё").unwrap();
    let before = a.fingerprint().unwrap();
    for name in ["", " \t\n", " РАБОТА ", "ВСЁ"] {
        assert!(create(&mut a.doc, &home, name).is_err());
    }
    for name in ["", " \t\n", " ВСЕ ", "всё"] {
        assert!(rename(&mut a.doc, &left, name).is_err());
    }
    assert!(create(&mut a.doc, "foreign", "Другой").is_err());
    assert!(rename(&mut a.doc, &ROOT, "Другой").is_err());
    assert!(assign(&mut a.doc, "k2", &left, true).is_err());
    assert!(assign(&mut a.doc, "unknown", &left, false).is_err());
    assert!(assign(&mut a.doc, "k0", &ROOT, true).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    // Е and Ё remain distinct, including when changing an existing tag's name.
    rename(&mut a.doc, &other, "Новый").unwrap();
    rename(&mut a.doc, &other, "Все").unwrap();
}

#[test]
fn cross_alias_concurrent_membership_changes_preserve_both_values() {
    let (_directory, mut store) = prepared();
    let (left, right) = merged(&mut store);
    for (peer, id, again) in [("A", &left, false), ("B", &right, true)] {
        let mut replica = store.read(peer).unwrap();
        assign(&mut replica.doc, "k0", id, false).unwrap();
        if again {
            assign(&mut replica.doc, "k0", id, true).unwrap();
        }
        store.write(peer, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    assert_link(&a.doc, "k0", &left, &["absent", "present"]);
    let before = a.fingerprint().unwrap();
    assert!(assign(&mut a.doc, "k0", &right, true).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    let home = object(&a.doc, "s0").unwrap().to_string();
    let other = create(&mut a.doc, &home, "Другой").unwrap();
    let before = a.fingerprint().unwrap();
    assert!(assign(&mut a.doc, "k0", &other, true).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    assign(&mut a.doc, "k1", &right, false).unwrap();
    assert_link(&a.doc, "k1", &left, &["absent"]);
}

#[test]
fn late_bridge_does_not_silently_merge_causally_distinct_tags() {
    let (_directory, mut store) = prepared();
    let left = add(&mut store, "A", "s0", "Работа", "k0");
    let mut a = store.read("A").unwrap();
    rename(&mut a.doc, &left, "Личное").unwrap();
    let home = object(&a.doc, "s0").unwrap().to_string();
    let next = create(&mut a.doc, &home, "Работа").unwrap();
    assign(&mut a.doc, "k1", &next, true).unwrap();
    store.write("A", &a).unwrap();
    let late = add(&mut store, "B", "s0", "Работа", "k0");
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    assert_eq!(tags(&a.doc).unwrap().len(), 3);
    for id in [&left, &next, &late] {
        let group = tag(&a.doc, id).unwrap();
        assert_eq!(group.ids, std::slice::from_ref(id));
        assert!(group.blocked);
        let before = a.fingerprint().unwrap();
        assert!(rename(&mut a.doc, id, "Другое").is_err());
        assert!(assign(&mut a.doc, "k0", id, false).is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
    }
    assert_link(&a.doc, "k0", &left, &["present"]);
    assert_link(&a.doc, "k1", &next, &["present"]);
    assert_link(&a.doc, "k0", &late, &["present"]);
}

#[test]
fn rename_collisions_preserve_ids_and_links_without_auto_merging() {
    for rename_against_creation in [false, true] {
        let (_directory, mut store) = prepared();
        let left = add(&mut store, "A", "s0", "Работа", "k0");
        let right = add(&mut store, "A", "s0", "Личное", "k1");
        exchange(&mut store).unwrap();
        let mut a = store.read("A").unwrap();
        rename(&mut a.doc, &left, "Важное").unwrap();
        store.write("A", &a).unwrap();
        let right = if rename_against_creation {
            add(&mut store, "B", "s0", " ВАЖНОЕ ", "k1")
        } else {
            let mut b = store.read("B").unwrap();
            rename(&mut b.doc, &right, "ВАЖНОЕ").unwrap();
            store.write("B", &b).unwrap();
            right
        };
        exchange(&mut store).unwrap();
        let mut a = store.read("A").unwrap();
        for id in [&left, &right] {
            assert_eq!(tag(&a.doc, id).unwrap().ids, std::slice::from_ref(id));
            assert!(tag(&a.doc, id).unwrap().blocked);
            let before = a.fingerprint().unwrap();
            assert!(assign(&mut a.doc, "k0", id, true).is_err());
            assert_eq!(a.fingerprint().unwrap(), before);
        }
        assert_link(&a.doc, "k0", &left, &["present"]);
        assert_link(&a.doc, "k1", &right, &["present"]);
    }
}

#[test]
fn late_independent_third_creation_keeps_aliases_assignments_and_original_spelling() {
    let (_directory, mut store) = prepared();
    let mut late = store.read("B").unwrap();
    late.doc.set_actor(automerge::ActorId::random());
    let home = object(&late.doc, "s0").unwrap().to_string();
    let third = create(&mut late.doc, &home, " работа ").unwrap();
    assign(&mut late.doc, "k0", &third, true).unwrap();
    let (left, _) = merged(&mut store);
    let mut a = store.read("A").unwrap();
    a.doc.merge(&mut late.doc).unwrap();
    let group = tag(&a.doc, &third).unwrap();
    assert_eq!(tags(&a.doc).unwrap().len(), 1);
    assert_eq!(group.ids.len(), 3);
    assert!(["РАБОТА", "Работа", " работа "].contains(&group.name()));
    assert_link(&a.doc, "k0", &left, &["present"]);
    rename(&mut a.doc, &left, "Новое").unwrap();
    assign(&mut a.doc, "k0", &third, false).unwrap();
    assert_link(&a.doc, "k0", &left, &["absent"]);
}

#[test]
fn repeated_reordered_changes_do_not_undo_a_later_rename_or_alias_removal() {
    let (_directory, mut store) = prepared();
    let (left, right) = merged(&mut store);
    let mut a = store.read("A").unwrap();
    let old = a.doc.get_changes(&[]);
    rename(&mut a.doc, &right, "Новое").unwrap();
    assign(&mut a.doc, "k0", &left, false).unwrap();
    let expected = snapshot(&a.doc).unwrap();
    let changes = a.doc.get_changes(&[]);
    for reverse in [false, true] {
        for shift in 0..changes.len() {
            let mut delivery = changes.clone();
            if reverse {
                delivery.reverse();
            }
            delivery.rotate_left(shift);
            let mut replica = Replica::new();
            for change in delivery {
                replica.doc.apply_changes([change.clone(), change]).unwrap();
            }
            replica.doc.apply_changes(old.clone()).unwrap();
            assert_eq!(snapshot(&replica.doc).unwrap(), expected);
            assert_eq!(replica.doc.get_heads(), a.doc.get_heads());
            assert_eq!(replica.doc.get_changes(&[]).len(), changes.len());
            store.write("A", &replica).unwrap();
            assert_eq!(snapshot(&store.read("A").unwrap().doc).unwrap(), expected);
        }
    }
}

#[test]
fn sqlite_rejects_the_entire_identity_rename_and_membership_write() {
    let (directory, mut store) = prepared();
    let (left, right) = merged(&mut store);
    let before_a = store.read("A").unwrap().fingerprint().unwrap();
    let before_b = store.read("B").unwrap().fingerprint().unwrap();
    let mut a = store.read("A").unwrap();
    rename(&mut a.doc, &right, "Новое").unwrap();
    assign(&mut a.doc, "k0", &left, false).unwrap();
    store
        .0
        .execute_batch(
            "CREATE TRIGGER reject_save AFTER UPDATE ON replicas
        BEGIN SELECT RAISE(ABORT, 'synthetic identity save failure'); END;",
        )
        .unwrap();
    assert!(store.write("A", &a).is_err());
    drop(store);
    let store = Store::open(&directory.path().join("automerge-tag-identity.sqlite3")).unwrap();
    assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), before_a);
    assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), before_b);
}

#[test]
fn native_phases_preserve_aliases_and_later_changes_between_processes() {
    let directory = tempfile::tempdir().unwrap();
    for phase in ["identity-prepare", "identity-rename", "identity-resume"] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "identity-prepare").is_err());
}
