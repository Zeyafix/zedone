use super::*;

fn prepared(conflict: bool) -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("trash.sqlite3")).unwrap();
    if conflict {
        scenario::prepare(&mut store).unwrap();
    } else {
        seed(&mut store).unwrap();
    }
    (directory, store)
}

fn choose(doc: &mut Automerge, state: &str) {
    set_life(doc, "k0", &view(doc, "k0").unwrap(), state).unwrap();
}

fn change(doc: &mut Automerge, field: &str, value: &str) {
    edit(doc, "k0", &view(doc, "k0").unwrap(), field, value, false).unwrap();
}

#[test]
fn trash_against_each_independent_field_preserves_both_choices_and_inverse() {
    for (field, value) in FIELDS.iter().zip(scenario::EDITED_FIELDS) {
        for state in ["active", "trash"] {
            let (_directory, mut store) = prepared(false);
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            let original = view(&a.doc, "k0").unwrap();
            let other = view(&a.doc, "k1").unwrap();
            choose(&mut a.doc, "trash");
            change(&mut b.doc, field, value);
            a.doc.merge(&mut b.doc).unwrap();
            assert_eq!(view(&a.doc, "k0").unwrap().location(), "conflicts");
            let before = a.fingerprint().unwrap();
            for (blocked_field, blocked_value) in FIELDS.into_iter().zip(INITIAL) {
                assert!(
                    edit(
                        &mut a.doc,
                        "k0",
                        &view(&b.doc, "k0").unwrap(),
                        blocked_field,
                        blocked_value,
                        false
                    )
                    .is_err()
                );
                let observed = view(&a.doc, "k0").unwrap();
                assert!(
                    edit(
                        &mut a.doc,
                        "k0",
                        &observed,
                        blocked_field,
                        blocked_value,
                        false
                    )
                    .is_err()
                );
            }
            assert_eq!(a.fingerprint().unwrap(), before);
            // The other card remains editable; it does not stale this card's choice.
            let observed = view(&a.doc, "k0").unwrap();
            let other_view = view(&a.doc, "k1").unwrap();
            edit(
                &mut a.doc,
                "k1",
                &other_view,
                "text",
                "Другая карточка",
                false,
            )
            .unwrap();
            set_life(&mut a.doc, "k0", &observed, state).unwrap();
            let selected = view(&a.doc, "k0").unwrap();
            assert!(!selected.blocked());
            assert_eq!(selected.id, original.id);
            for key in FIELDS {
                if key == *field {
                    assert_eq!(selected.fields[key][0].value, value);
                } else {
                    assert_eq!(selected.fields[key], original.fields[key]);
                }
            }
            assert_eq!(view(&a.doc, "k1").unwrap().id, other.id);
            for next in [if state == "active" { "trash" } else { "active" }, state] {
                choose(&mut a.doc, next);
            }
            assert_eq!(view(&a.doc, "k0").unwrap().fields, selected.fields);
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
fn normal_trash_restore_and_stale_invalid_or_foreign_commands_are_atomic() {
    let (_directory, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let old = view(&a.doc, "k0").unwrap();
    change(&mut a.doc, "text", "Новая правка");
    let before = a.fingerprint().unwrap();
    assert!(set_life(&mut a.doc, "k0", &old, "trash").is_err());
    assert!(edit(&mut a.doc, "k0", &old, "title", "old", false).is_err());
    assert!(set_life(&mut a.doc, "k1", &old, "trash").is_err());
    for (field, value) in [
        ("title", " "),
        ("completed", "yes"),
        ("place", "foreign:100"),
        ("tag", "invalid"),
        ("unknown", "x"),
    ] {
        let observed = view(&a.doc, "k0").unwrap();
        assert!(edit(&mut a.doc, "k0", &observed, field, value, false).is_err());
    }
    for state in ["active", "purged"] {
        let observed = view(&a.doc, "k0").unwrap();
        assert!(set_life(&mut a.doc, "k0", &observed, state).is_err());
    }
    assert_eq!(a.fingerprint().unwrap(), before);
    choose(&mut a.doc, "trash");
    let observed = view(&a.doc, "k0").unwrap();
    let trashed = a.fingerprint().unwrap();
    assert!(set_life(&mut a.doc, "k0", &observed, "trash").is_err());
    for (field, value) in FIELDS.into_iter().zip(INITIAL) {
        assert!(edit(&mut a.doc, "k0", &observed, field, value, false).is_err());
    }
    assert_eq!(a.fingerprint().unwrap(), trashed);
    choose(&mut a.doc, "active");
    assert_eq!(view(&a.doc, "k0").unwrap().fields, observed.fields);
    change(&mut a.doc, "text", "Вернулось редактирование");
}

#[test]
fn equal_and_different_concurrent_choices_and_late_edits_remain_explicit() {
    for equal in [false, true] {
        let (_directory, store) = prepared(true);
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        choose(&mut a.doc, "active");
        choose(&mut b.doc, if equal { "active" } else { "trash" });
        change(&mut a.doc, "text", "После локального восстановления");
        a.doc.merge(&mut b.doc).unwrap();
        assert_eq!(view(&a.doc, "k0").unwrap().blocked(), !equal);
        if !equal {
            let stale = view(&a.doc, "k0").unwrap();
            // A restored replica may continue working before it hears the other choice.
            let mut late = store.read("B").unwrap();
            late.doc = late.doc.fork(); // A third independent writer, not a reused actor sequence.
            choose(&mut late.doc, "active");
            change(&mut late.doc, "title", "Поздняя правка");
            a.doc.merge(&mut late.doc).unwrap();
            let before = a.fingerprint().unwrap();
            assert!(set_life(&mut a.doc, "k0", &stale, "trash").is_err());
            assert_eq!(a.fingerprint().unwrap(), before);
            choose(&mut a.doc, "trash");
            let card = view(&a.doc, "k0").unwrap();
            assert_eq!(card.location(), "trash");
            assert_eq!(card.fields["title"][0].value, "Поздняя правка");
        }
        assert_eq!(
            view(&a.doc, "k0").unwrap().fields["text"][0].value,
            "После локального восстановления"
        );
    }
}

#[test]
fn lifecycle_and_text_conflicts_are_resolved_independently() {
    for life_first in [false, true] {
        for state in ["active", "trash"] {
            let (_directory, store) = prepared(false);
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            change(&mut a.doc, "text", "Текст A");
            choose(&mut a.doc, "trash");
            change(&mut b.doc, "text", "Текст B");
            a.doc.merge(&mut b.doc).unwrap();
            if life_first {
                choose(&mut a.doc, state);
                assert!(view(&a.doc, "k0").unwrap().blocked());
            }
            let observed = view(&a.doc, "k0").unwrap();
            edit(
                &mut a.doc,
                "k0",
                &observed,
                "text",
                "Ручное объединение",
                true,
            )
            .unwrap();
            if !life_first {
                assert_eq!(view(&a.doc, "k0").unwrap().life, observed.life);
                choose(&mut a.doc, state);
            }
            assert!(!view(&a.doc, "k0").unwrap().blocked());
            assert_eq!(
                view(&a.doc, "k0").unwrap().location(),
                if state == "trash" { "trash" } else { "board" }
            );
        }
    }
}

#[test]
fn old_and_permuted_changes_do_not_restore_a_trashed_card_after_reload() {
    let (_directory, mut store) = prepared(true);
    let mut a = store.read("A").unwrap();
    let mut late = store.read("B").unwrap();
    choose(&mut a.doc, "active");
    choose(&mut late.doc, "trash");
    a.doc.merge(&mut late.doc).unwrap();
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
        fresh.doc.apply_changes(late.doc.get_changes(&[])).unwrap();
        assert_eq!(fresh.doc.get_heads(), a.doc.get_heads());
        assert_eq!(fresh.doc.get_changes(&[]).len(), changes.len());
        assert_eq!(snapshot(&fresh.doc).unwrap(), expected);
        store.write("A", &fresh).unwrap();
        assert_eq!(snapshot(&store.read("A").unwrap().doc).unwrap(), expected);
    }
}

#[test]
fn sqlite_abort_keeps_both_documents_actors_and_peers_for_each_mutation() {
    for action in ["trash", "restore", "choose-trash", "choose-active", "edit"] {
        let (directory, mut store) = prepared(action.starts_with("choose"));
        if action == "restore" {
            let mut a = store.read("A").unwrap();
            choose(&mut a.doc, "trash");
            store.write("A", &a).unwrap();
            exchange(&mut store).unwrap();
        }
        let before = [
            store.read("A").unwrap().fingerprint().unwrap(),
            store.read("B").unwrap().fingerprint().unwrap(),
        ];
        let mut a = store.read("A").unwrap();
        match action {
            "trash" | "choose-trash" => choose(&mut a.doc, "trash"),
            "restore" | "choose-active" => choose(&mut a.doc, "active"),
            _ => change(&mut a.doc, "text", "Не должно сохраниться"),
        }
        store.0.execute_batch("CREATE TRIGGER reject_trash AFTER UPDATE ON replicas BEGIN SELECT RAISE(ABORT, 'synthetic trash failure'); END;").unwrap();
        assert!(store.write("A", &a).is_err());
        drop(store);
        let store = Store::open(&directory.path().join("trash.sqlite3")).unwrap();
        for (peer, expected) in ["A", "B"].into_iter().zip(before) {
            assert_eq!(store.read(peer).unwrap().fingerprint().unwrap(), expected);
        }
    }
}

#[test]
fn an_unseen_old_replica_edit_reopens_a_resolved_deletion_without_losing_data() {
    let (_directory, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let mut old = store.read("B").unwrap();
    let initial = view(&a.doc, "k0").unwrap();
    choose(&mut a.doc, "trash");
    choose(&mut a.doc, "active");
    choose(&mut a.doc, "trash");
    change(&mut old.doc, "text", "Очень поздняя независимая правка");
    a.doc.merge(&mut old.doc).unwrap();
    let current = view(&a.doc, "k0").unwrap();
    assert_eq!(current.location(), "conflicts");
    assert_eq!(current.id, initial.id);
    assert_eq!(
        current.fields["text"][0].value,
        "Очень поздняя независимая правка"
    );
    choose(&mut a.doc, "trash");
    a.doc.merge(&mut old.doc).unwrap();
    assert_eq!(view(&a.doc, "k0").unwrap().location(), "trash");
    choose(&mut a.doc, "active");
    assert_eq!(view(&a.doc, "k0").unwrap().fields, current.fields);
}

#[test]
fn stale_field_choices_and_ordinary_deletion_with_a_text_conflict_do_not_write() {
    let (_directory, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    change(&mut a.doc, "text", "A");
    change(&mut b.doc, "text", "B");
    a.doc.merge(&mut b.doc).unwrap();
    let observed = view(&a.doc, "k0").unwrap();
    let before = a.fingerprint().unwrap();
    assert!(set_life(&mut a.doc, "k0", &observed, "trash").is_err());
    assert!(edit(&mut a.doc, "k0", &observed, "title", "no conflict", true).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    edit(&mut a.doc, "k0", &observed, "text", "Решено", true).unwrap();
    let after = a.fingerprint().unwrap();
    assert!(edit(&mut a.doc, "k0", &observed, "text", "stale", true).is_err());
    assert_eq!(a.fingerprint().unwrap(), after);
    choose(&mut a.doc, "trash");
}

#[test]
fn each_edit_or_lifecycle_choice_is_a_single_automerge_change() {
    let (_directory, store) = prepared(false);
    let mut a = store.read("A").unwrap();
    for (field, value) in FIELDS.iter().zip(scenario::EDITED_FIELDS) {
        let count = a.doc.get_changes(&[]).len();
        change(&mut a.doc, field, value);
        assert_eq!(a.doc.get_changes(&[]).len(), count + 1);
    }
    for state in ["trash", "active"] {
        let count = a.doc.get_changes(&[]).len();
        choose(&mut a.doc, state);
        assert_eq!(a.doc.get_changes(&[]).len(), count + 1);
    }
}

#[test]
fn native_trash_phases_check_both_choices_and_refuse_overwriting_fixture() {
    let directory = tempfile::tempdir().unwrap();
    for phase in ["trash-prepare", "trash-resolve", "trash-resume"] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "trash-prepare").is_err());
}
