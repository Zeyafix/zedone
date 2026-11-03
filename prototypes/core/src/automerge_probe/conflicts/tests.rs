use super::*;

#[test]
fn opposite_existing_choices_always_create_new_assignments() {
    // Choose both the visible winner and the other variant on each actor.
    for swap in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        run(directory.path(), "conflicts-prepare").unwrap();
        let mut store = Store::open(&directory.path().join("automerge-conflicts.sqlite3")).unwrap();
        let original = store.read("A").unwrap();
        let winner = original
            .doc
            .get(ROOT, "text")
            .unwrap()
            .unwrap()
            .1
            .to_string();
        let variants = original.variants(Field::Text).unwrap();
        for (id, choose_winner) in [("A", swap), ("B", !swap)] {
            let selected = variants
                .iter()
                .find(|v| (v.operation == winner) == choose_winner)
                .unwrap();
            let mut replica = store.read(id).unwrap();
            replica
                .resolve(&replica.view().unwrap(), Field::Text, &selected.value)
                .unwrap();
            let result = replica.variants(Field::Text).unwrap();
            assert_eq!(result.len(), 1);
            assert!(
                variants
                    .iter()
                    .all(|old| old.operation != result[0].operation)
            );
            store.write(id, &replica).unwrap();
        }
        exchange(&mut store).unwrap();
        for id in ["A", "B"] {
            expect(&store.read(id).unwrap(), Field::Text, &[LEFT, RIGHT]).unwrap();
        }
    }
}

#[test]
fn conflicts_and_offline_decisions_survive_restart_then_unlock_reverse_edits() {
    let directory = tempfile::tempdir().unwrap();
    for phase in ["conflicts-prepare", "conflicts-resolve", "conflicts-resume"] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "conflicts-prepare").is_err());
}

#[test]
fn text_only_conflict_blocks_both_fields_and_existing_variant_resolves_it() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("probe.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    for (id, text) in [("A", LEFT), ("B", RIGHT)] {
        let mut replica = store.read(id).unwrap();
        replica.edit(Field::Text, text).unwrap();
        store.write(id, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let conflict = a.view().unwrap();
    expect(&a, Field::Title, &[ORIGINAL]).unwrap();
    expect(&a, Field::Text, &[LEFT, RIGHT]).unwrap();
    for field in [Field::Title, Field::Text] {
        assert!(a.edit(field, "Blocked").is_err());
    }
    assert_eq!(a.view().unwrap(), conflict);
    assert!(
        a.resolve(&conflict, Field::Title, "No conflict here")
            .is_err()
    );
    a.resolve(&conflict, Field::Text, LEFT).unwrap();
    assert_ne!(a.doc.get_heads(), conflict.heads);
    expect(&a, Field::Text, &[LEFT]).unwrap();
    let old = a.doc.get_all_at(ROOT, "text", &conflict.heads).unwrap();
    assert_eq!(old.len(), 2, "Resolution must preserve the prior history");
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    expect(&store.read("B").unwrap(), Field::Text, &[LEFT]).unwrap();
}

#[test]
fn stale_resolution_rejects_new_version_without_mutating_any_field() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "conflicts-prepare").unwrap();
    let mut store = Store::open(&directory.path().join("automerge-conflicts.sqlite3")).unwrap();
    let observed = store.read("A").unwrap().view().unwrap();
    let mut b = store.read("B").unwrap();
    b.resolve(&b.view().unwrap(), Field::Text, RIGHT).unwrap();
    store.write("B", &b).unwrap();
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let before = a.fingerprint().unwrap();
    assert!(
        a.resolve(&observed, Field::Title, "Outdated choice")
            .is_err()
    );
    assert_eq!(a.fingerprint().unwrap(), before);
    expect(&a, Field::Text, &[RIGHT]).unwrap();
    // The title is still conflicted; a fresh explicit choice is permitted.
    a.resolve(&a.view().unwrap(), Field::Title, "Fresh choice")
        .unwrap();
    a.edit(Field::Text, COMBINED).unwrap();
}

#[test]
fn unseen_edit_survives_a_local_resolution_as_a_new_conflict() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "conflicts-prepare").unwrap();
    let mut store = Store::open(&directory.path().join("automerge-conflicts.sqlite3")).unwrap();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    a.resolve(&a.view().unwrap(), Field::Text, LEFT).unwrap();
    expect(&a, Field::Text, &[LEFT]).unwrap();
    b.resolve(&b.view().unwrap(), Field::Text, RIGHT).unwrap();
    b.resolve(&b.view().unwrap(), Field::Title, "Решение B")
        .unwrap();
    b.edit(Field::Text, "Поздняя независимая правка").unwrap();
    store.write("A", &a).unwrap();
    store.write("B", &b).unwrap();
    exchange(&mut store).unwrap();
    for id in ["A", "B"] {
        expect(
            &store.read(id).unwrap(),
            Field::Text,
            &[LEFT, "Поздняя независимая правка"],
        )
        .unwrap();
    }
}

#[test]
fn duplicate_reordered_changes_preserve_variant_ids_and_competing_decisions() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "conflicts-prepare").unwrap();
    run(directory.path(), "conflicts-resolve").unwrap();
    let mut store = Store::open(&directory.path().join("automerge-conflicts.sqlite3")).unwrap();
    exchange(&mut store).unwrap();
    let expected = store.read("A").unwrap();
    let changes = expected.doc.get_changes(&[]);
    let view = expected.view().unwrap();
    assert_eq!(view, store.read("B").unwrap().view().unwrap());
    expect(&expected, Field::Text, &[LEFT, COMBINED]).unwrap();
    // Every cyclic offset in both directions, with each change delivered twice.
    // This tests the CRDT change set, not out-of-order sync-session messages.
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
            assert_eq!(target.doc.get_missing_deps(&[]), []);
            assert_eq!(target.view().unwrap(), view);
            assert_eq!(target.doc.get_changes(&[]).len(), changes.len());
            store.write("A", &target).unwrap();
            assert_eq!(store.read("A").unwrap().view().unwrap(), view);
        }
    }
}

#[test]
fn failed_resolution_save_retains_all_original_variants_after_reopen() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "conflicts-prepare").unwrap();
    let path = directory.path().join("automerge-conflicts.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let before = store.read("A").unwrap().fingerprint().unwrap();
    let other = store.read("B").unwrap().fingerprint().unwrap();
    let mut candidate = store.read("A").unwrap();
    candidate
        .resolve(&candidate.view().unwrap(), Field::Text, COMBINED)
        .unwrap();
    store
        .0
        .execute_batch(
            "CREATE TRIGGER reject_resolution AFTER UPDATE ON replicas
         BEGIN SELECT RAISE(ABORT, 'synthetic resolution failure'); END;",
        )
        .unwrap();
    assert!(store.write("A", &candidate).is_err());
    drop(store);
    let store = Store::open(&path).unwrap();
    assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), before);
    assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), other);
}

#[test]
fn identical_concurrent_values_keep_operation_ids_without_a_false_text_conflict() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("probe.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    for id in ["A", "B"] {
        let mut replica = store.read(id).unwrap();
        replica.edit(Field::Text, LEFT).unwrap();
        store.write(id, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    assert_eq!(a.variants(Field::Text).unwrap().len(), 2);
    expect(&a, Field::Text, &[LEFT]).unwrap();
    a.edit(Field::Title, "No different texts").unwrap();
    a.edit(Field::Text, RIGHT).unwrap();
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    expect(&store.read("B").unwrap(), Field::Text, &[RIGHT]).unwrap();
}
