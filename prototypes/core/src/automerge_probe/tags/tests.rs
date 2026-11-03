use super::*;

fn prepared() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("automerge-tags.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    (directory, store)
}

fn save_assignment(store: &mut Store, id: &str, card: &str, key: &str, present: bool) {
    let mut replica = store.read(id).unwrap();
    let tag = object(&replica.doc, key).unwrap().to_string();
    assign(&mut replica.doc, card, &tag, present, None).unwrap();
    store.write(id, &replica).unwrap();
}

fn conflict(store: &mut Store) {
    save_assignment(store, "A", "k0", "t0", false);
    save_assignment(store, "B", "k0", "t0", false);
    save_assignment(store, "B", "k0", "t0", true);
    exchange(store).unwrap();
    assert!(view(&store.read("A").unwrap().doc, "k0").unwrap().blocked());
}

fn membership(doc: &Automerge, card: &str, key: &str) -> Link {
    let tag = object(doc, key).unwrap().to_string();
    view(doc, card)
        .unwrap()
        .links
        .into_iter()
        .find(|v| v.tag == tag)
        .unwrap()
}

#[test]
fn distinct_and_identical_additions_preserve_existing_links_and_reverse() {
    for same_tag in [false, true] {
        let (_directory, mut store) = prepared();
        let original = store.read("A").unwrap();
        let original_card = view(&original.doc, "k1").unwrap();
        let heads = original.doc.get_heads();
        save_assignment(&mut store, "A", "k1", "t0", true);
        save_assignment(
            &mut store,
            "B",
            "k1",
            if same_tag { "t0" } else { "t1" },
            true,
        );
        exchange(&mut store).unwrap();
        let mut a = store.read("A").unwrap();
        assert!(membership(&a.doc, "k1", "t0").present());
        assert_eq!(membership(&a.doc, "k1", "t1").present(), !same_tag);
        assert!(membership(&a.doc, "k0", "t0").present());
        assert!(membership(&a.doc, "k1", "t3").present());
        assert!(!view(&a.doc, "k1").unwrap().blocked());
        assert_eq!(view(&a.doc, "k1").unwrap().id, original_card.id);
        assert_eq!(tags(&a.doc).unwrap(), tags(&original.doc).unwrap());
        let tag = object(&a.doc, "t0").unwrap().to_string();
        let before = a.fingerprint().unwrap();
        assign(&mut a.doc, "k1", &tag, true, None).unwrap();
        assert_eq!(before, a.fingerprint().unwrap());
        assign(&mut a.doc, "k1", &tag, false, None).unwrap();
        assert!(!membership(&a.doc, "k1", "t0").present());
        assign(&mut a.doc, "k1", &tag, true, None).unwrap();
        assert!(membership(&a.doc, "k1", "t0").present());
        assert_eq!(
            view(&a.doc.fork_at(&heads).unwrap(), "k1").unwrap(),
            original_card
        );
        store.write("A", &a).unwrap();
        exchange(&mut store).unwrap();
        assert_eq!(
            snapshot(&a.doc).unwrap(),
            snapshot(&store.read("B").unwrap().doc).unwrap()
        );
    }
}

#[test]
fn identical_removals_do_not_conflict_or_delete_the_tag() {
    let (_directory, mut store) = prepared();
    let original = tags(&store.read("A").unwrap().doc).unwrap();
    for id in ["A", "B"] {
        save_assignment(&mut store, id, "k0", "t0", false);
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let link = membership(&a.doc, "k0", "t0");
    assert!(!link.present());
    assert_eq!(link.variants.len(), 2);
    assert!(!view(&a.doc, "k0").unwrap().blocked());
    assert_eq!(tags(&a.doc).unwrap(), original);
    let tag = object(&a.doc, "t0").unwrap().to_string();
    let before = a.fingerprint().unwrap();
    assign(&mut a.doc, "k0", &tag, false, None).unwrap();
    assert_eq!(a.fingerprint().unwrap(), before);
}

#[test]
fn tag_and_text_changes_are_independent_and_untouched_cards_stay_unchanged() {
    let (_directory, mut store) = prepared();
    let original = view(&store.read("A").unwrap().doc, "k2").unwrap();
    save_assignment(&mut store, "A", "k0", "t1", true);
    let mut b = store.read("B").unwrap();
    edit(&mut b.doc, "k0", Field::Title, "Новый заголовок").unwrap();
    edit(&mut b.doc, "k0", Field::Text, "Другой текст — ёж 😀").unwrap();
    store.write("B", &b).unwrap();
    exchange(&mut store).unwrap();
    for id in ["A", "B"] {
        let replica = store.read(id).unwrap();
        let card = view(&replica.doc, "k0").unwrap();
        assert!(!card.blocked());
        assert_eq!(card.title[0].value, "Новый заголовок");
        assert_eq!(card.text[0].value, "Другой текст — ёж 😀");
        for key in ["t0", "t1"] {
            assert!(membership(&replica.doc, "k0", key).present());
        }
        assert_eq!(view(&replica.doc, "k2").unwrap(), original);
    }
}

#[test]
fn removal_versus_reassignment_blocks_only_affected_card_and_allows_both_choices() {
    for choice in [false, true] {
        let (_directory, mut store) = prepared();
        let original = store.read("A").unwrap();
        conflict(&mut store);
        let mut a = store.read("A").unwrap();
        let card = view(&a.doc, "k0").unwrap();
        assert_eq!(card.title, view(&original.doc, "k0").unwrap().title);
        assert_eq!(card.text, view(&original.doc, "k0").unwrap().text);
        assert_eq!(membership(&a.doc, "k0", "t0").variants.len(), 2);
        let tag = object(&a.doc, "t0").unwrap().to_string();
        let other = object(&a.doc, "t1").unwrap().to_string();
        let before = a.fingerprint().unwrap();
        for field in [Field::Title, Field::Text] {
            assert!(edit(&mut a.doc, "k0", field, "Blocked").is_err());
        }
        for key in [&tag, &other] {
            for present in [false, true] {
                assert!(assign(&mut a.doc, "k0", key, present, None).is_err());
            }
        }
        assert_eq!(before, a.fingerprint().unwrap());
        edit(&mut a.doc, "k1", Field::Text, "Доступно").unwrap();
        assign(&mut a.doc, "k1", &tag, true, None).unwrap();
        // An edit to another card does not invalidate this read.
        assign(&mut a.doc, "k0", &tag, choice, Some(&card)).unwrap();
        assert_eq!(membership(&a.doc, "k0", "t0").present(), choice);
        assert!(!view(&a.doc, "k0").unwrap().blocked());
        edit(&mut a.doc, "k0", Field::Title, "Разблокировано").unwrap();
        assert_eq!(tags(&a.doc).unwrap(), tags(&original.doc).unwrap());
        store.write("A", &a).unwrap();
        exchange(&mut store).unwrap();
        assert_eq!(
            snapshot(&a.doc).unwrap(),
            snapshot(&store.read("B").unwrap().doc).unwrap()
        );
    }
}

#[test]
fn independent_decisions_keep_different_choices_and_do_not_invent_equal_conflicts() {
    for different in [false, true] {
        let (_directory, mut store) = prepared();
        conflict(&mut store);
        for (id, choice) in [("A", true), ("B", !different)] {
            let mut replica = store.read(id).unwrap();
            let observed = view(&replica.doc, "k0").unwrap();
            let tag = object(&replica.doc, "t0").unwrap().to_string();
            assign(&mut replica.doc, "k0", &tag, choice, Some(&observed)).unwrap();
            store.write(id, &replica).unwrap();
        }
        exchange(&mut store).unwrap();
        let a = store.read("A").unwrap();
        assert_eq!(view(&a.doc, "k0").unwrap().blocked(), different);
        assert_eq!(membership(&a.doc, "k0", "t0").variants.len(), 2);
        assert_eq!(
            snapshot(&a.doc).unwrap(),
            snapshot(&store.read("B").unwrap().doc).unwrap()
        );
    }
}

#[test]
fn invalid_and_stale_choices_leave_every_assignment_unchanged() {
    let (_directory, mut store) = prepared();
    conflict(&mut store);
    let mut a = store.read("A").unwrap();
    let old = view(&a.doc, "k0").unwrap();
    let tag = object(&a.doc, "t0").unwrap().to_string();
    let foreign = object(&a.doc, "t2").unwrap().to_string();
    let other = object(&a.doc, "t1").unwrap().to_string();
    let before = a.fingerprint().unwrap();
    assert!(assign(&mut a.doc, "k0", "unknown", true, Some(&old)).is_err());
    assert!(assign(&mut a.doc, "k0", &foreign, true, Some(&old)).is_err());
    assert!(assign(&mut a.doc, "k0", &other, true, Some(&old)).is_err());
    assert!(assign(&mut a.doc, "t0", &tag, true, Some(&old)).is_err());
    assert!(assign(&mut a.doc, "k1", &tag, true, Some(&old)).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    assign(&mut a.doc, "k0", &tag, true, Some(&old)).unwrap();
    // Return to a conflict with the same values but new assignment IDs.
    store.write("A", &a).unwrap();
    let mut b = store.read("B").unwrap();
    let observed = view(&b.doc, "k0").unwrap();
    assign(&mut b.doc, "k0", &tag, false, Some(&observed)).unwrap();
    store.write("B", &b).unwrap();
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let before = a.fingerprint().unwrap();
    assert!(assign(&mut a.doc, "k0", &tag, true, Some(&old)).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    // Same-named tag in another space is independent.
    assign(&mut a.doc, "k2", &foreign, true, None).unwrap();
    assert!(membership(&a.doc, "k2", "t2").present());
}

#[test]
fn resolving_a_link_does_not_unlock_a_card_with_a_text_conflict() {
    let (_directory, mut store) = prepared();
    for (id, value) in [("A", "Текст A"), ("B", "Текст B")] {
        let mut replica = store.read(id).unwrap();
        edit(&mut replica.doc, "k0", Field::Text, value).unwrap();
        store.write(id, &replica).unwrap();
    }
    // Changes are still local: ordinary tag actions can accompany the edits.
    conflict(&mut store);
    let mut a = store.read("A").unwrap();
    let observed = view(&a.doc, "k0").unwrap();
    let tag = object(&a.doc, "t0").unwrap().to_string();
    assign(&mut a.doc, "k0", &tag, true, Some(&observed)).unwrap();
    assert!(!conflicting(&membership(&a.doc, "k0", "t0").variants));
    assert!(view(&a.doc, "k0").unwrap().blocked());
    assert!(edit(&mut a.doc, "k0", Field::Title, "Blocked").is_err());
    assert!(assign(&mut a.doc, "k0", &tag, false, None).is_err());
}

#[test]
fn native_phases_keep_conflicts_and_independent_decisions_across_restarts() {
    let directory = tempfile::tempdir().unwrap();
    for phase in ["tags-prepare", "tags-resolve", "tags-resume"] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "tags-prepare").is_err());
}

#[test]
fn sqlite_failure_preserves_both_replicas_including_the_unresolved_choice() {
    let (directory, mut store) = prepared();
    conflict(&mut store);
    let before_a = store.read("A").unwrap().fingerprint().unwrap();
    let before_b = store.read("B").unwrap().fingerprint().unwrap();
    let mut a = store.read("A").unwrap();
    let observed = view(&a.doc, "k0").unwrap();
    let tag = object(&a.doc, "t0").unwrap().to_string();
    assign(&mut a.doc, "k0", &tag, false, Some(&observed)).unwrap();
    store
        .0
        .execute_batch(
            "CREATE TRIGGER reject_save AFTER UPDATE ON replicas
        BEGIN SELECT RAISE(ABORT, 'synthetic tag write failure'); END;",
        )
        .unwrap();
    assert!(store.write("A", &a).is_err());
    drop(store);
    let store = Store::open(&directory.path().join("automerge-tags.sqlite3")).unwrap();
    assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), before_a);
    assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), before_b);
}

#[test]
fn duplicated_reordered_changes_keep_choices_heads_and_ids_after_sqlite_reload() {
    let (_directory, mut store) = prepared();
    conflict(&mut store);
    // Keep a late decision on a branch while A resolves the known conflict.
    let mut late = store.read("B").unwrap();
    let observed = view(&late.doc, "k0").unwrap();
    let tag = object(&late.doc, "t0").unwrap().to_string();
    assign(&mut late.doc, "k0", &tag, false, Some(&observed)).unwrap();
    let mut a = store.read("A").unwrap();
    assign(&mut a.doc, "k0", &tag, true, Some(&observed)).unwrap();
    // This unknown decision must survive the later merge.
    a.doc.merge(&mut late.doc).unwrap();
    assert!(view(&a.doc, "k0").unwrap().blocked());
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
            for change in &delivery {
                replica
                    .doc
                    .apply_changes([change.clone(), change.clone()])
                    .unwrap();
            }
            assert_eq!(replica.doc.get_heads(), a.doc.get_heads());
            assert_eq!(replica.doc.get_changes(&[]).len(), changes.len());
            assert_eq!(snapshot(&replica.doc).unwrap(), expected);
            store.write("A", &replica).unwrap();
            assert_eq!(snapshot(&store.read("A").unwrap().doc).unwrap(), expected);
        }
    }
}
