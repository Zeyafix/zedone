use super::*;

fn prepared() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("history.sqlite3")).unwrap();
    seed_history(&mut store).unwrap();
    (dir, store)
}

#[test]
fn card_and_space_previews_count_versions_and_keep_current_state_and_other_scopes() {
    for whole_space in [false, true] {
        let (_dir, mut store) = prepared();
        let mut a = store.read("A").unwrap();
        let before = view(&a.doc).unwrap();
        let plan = preview(&a.doc, "s0", if whole_space { None } else { Some("k0") }).unwrap();
        assert_eq!(plan.removed.len(), if whole_space { 3 } else { 2 });
        assert_eq!(plan.retained_conflicts.len(), 0);
        let count = a.doc.get_changes(&[]).len();
        clear(&mut a.doc, &plan).unwrap();
        assert_eq!(a.doc.get_changes(&[]).len(), count + 1);
        assert_eq!(view(&a.doc).unwrap(), before);
        assert_eq!(
            versions(&a.doc).unwrap().len(),
            if whole_space { 1 } else { 2 }
        );
        store.write("A", &a).unwrap();
        exchange(&mut store).unwrap();
        assert_eq!(view(&store.read("B").unwrap().doc).unwrap(), before);
        assert_eq!(
            versions(&store.read("B").unwrap().doc).unwrap(),
            versions(&a.doc).unwrap()
        );
    }
}

#[test]
fn clearing_offline_keeps_unseen_versions_in_both_delivery_orders_and_after_restart() {
    for first in ["clear", "edit"] {
        let (_dir, store) = prepared();
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        let plan = preview(&a.doc, "s0", None).unwrap();
        clear(&mut a.doc, &plan).unwrap();
        save_pair(
            &mut b.doc,
            "k0",
            "Независимый заголовок",
            "Независимый текст",
        )
        .unwrap();
        save_pair(&mut b.doc, "k1", "Сосед", "Новая соседняя версия").unwrap();
        let mut joined = if first == "clear" {
            a.doc.fork()
        } else {
            b.doc.fork()
        };
        let mut other = if first == "clear" { b.doc } else { a.doc };
        joined.merge(&mut other).unwrap();
        let joined = Automerge::load(&joined.save()).unwrap();
        let entries = versions(&joined).unwrap();
        assert_eq!(entries.len(), 3);
        assert!(
            entries
                .iter()
                .any(|v| v.title == "Независимый заголовок" && v.text == "Независимый текст")
        );
        assert!(!entries.iter().any(|v| plan.removed.contains(&v.id)));
        let mut rebuilt = Automerge::new();
        rebuilt
            .apply_changes(joined.get_changes(&[]).into_iter().rev())
            .unwrap();
        rebuilt.apply_changes(joined.get_changes(&[])).unwrap();
        assert_eq!(snapshot(&rebuilt).unwrap(), snapshot(&joined).unwrap());
        assert_eq!(view(&rebuilt).unwrap(), view(&joined).unwrap());
    }
}

#[test]
fn clearing_preserves_each_unresolved_text_pair_and_current_variants() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    save_pair(&mut a.doc, "k0", "A", "Текст A").unwrap();
    save_pair(&mut b.doc, "k0", "B", "Текст B").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let before = view(&a.doc).unwrap();
    let plan = preview(&a.doc, "s0", None).unwrap();
    assert_eq!(plan.removed.len(), 3);
    assert_eq!(plan.retained_conflicts.len(), 2);
    clear(&mut a.doc, &plan).unwrap();
    assert_eq!(view(&a.doc).unwrap(), before);
    let entries = versions(&a.doc).unwrap();
    for (title, text) in [("A", "Текст A"), ("B", "Текст B")] {
        assert!(entries.iter().any(|v| v.title == title && v.text == text));
    }
    let plan = preview(&a.doc, "s0", Some("k0")).unwrap();
    let bytes = a.doc.save();
    assert!(clear(&mut a.doc, &plan).is_err());
    assert_eq!(a.doc.save(), bytes);
    // Resolving the conflict makes those versions removable by a new explicit command.
    for (field, value) in [("title", "A"), ("text", "Текст A")] {
        let current = view(&a.doc).unwrap()["k0"].clone();
        edit(&mut a.doc, "k0", &current, field, value, true).unwrap();
    }
    let plan = preview(&a.doc, "s0", Some("k0")).unwrap();
    assert_eq!(plan.removed.len(), 2);
    clear(&mut a.doc, &plan).unwrap();
    assert_eq!(
        view(&a.doc).unwrap()["k0"].fields["text"][0].value,
        "Текст A"
    );
}

#[test]
fn simultaneous_clears_and_later_known_clears_converge_without_clock_order() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    let plan_a = preview(&a.doc, "s0", Some("k0")).unwrap();
    let plan_b = preview(&b.doc, "s0", None).unwrap();
    clear(&mut a.doc, &plan_a).unwrap();
    clear(&mut b.doc, &plan_b).unwrap();
    save_pair(&mut a.doc, "k0", "Новая", "После команды A").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    assert_eq!(versions(&a.doc).unwrap().len(), 2);
    let plan = preview(&a.doc, "s0", None).unwrap();
    assert_eq!(plan.removed.len(), 1);
    clear(&mut a.doc, &plan).unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    assert_eq!(versions(&a.doc).unwrap().len(), 1);
    assert_eq!(
        view(&a.doc).unwrap()["k0"].fields["title"][0].value,
        "Новая"
    );
}

#[test]
fn known_concurrent_versions_remain_protected_and_unseen_versions_survive_clear() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    let mut c = b.doc.fork();
    save_pair(&mut a.doc, "k0", "A", "A").unwrap();
    save_pair(&mut b.doc, "k0", "B", "B").unwrap();
    save_pair(&mut c, "k0", "C", "C").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let plan = preview(&a.doc, "s0", Some("k0")).unwrap();
    clear(&mut a.doc, &plan).unwrap();
    a.doc.merge(&mut c).unwrap();
    assert_eq!(
        versions(&a.doc)
            .unwrap()
            .iter()
            .filter(|v| v.card == "k0")
            .count(),
        3
    );
    assert_eq!(view(&a.doc).unwrap()["k0"].fields["title"].len(), 3);
}

#[test]
fn invalid_stale_and_empty_commands_do_not_change_state_or_history() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    for (space, card) in [
        ("s0", Some("foreign")),
        ("c0", None),
        ("s0", Some("c0")),
        ("missing", None),
    ] {
        let before = a.doc.save();
        assert!(preview(&a.doc, space, card).is_err());
        assert_eq!(a.doc.save(), before);
    }
    let stale = preview(&a.doc, "s0", Some("k0")).unwrap();
    save_pair(&mut a.doc, "k0", "Ещё одна", "Версия").unwrap();
    let before = a.doc.save();
    assert!(clear(&mut a.doc, &stale).is_err());
    assert_eq!(a.doc.save(), before);
    let plan = preview(&a.doc, "s0", Some("k0")).unwrap();
    save_pair(&mut a.doc, "foreign", "Другое", "Не обесценивает").unwrap();
    clear(&mut a.doc, &plan).unwrap();
    let plan = preview(&a.doc, "s0", Some("k0")).unwrap();
    let before = a.doc.save();
    assert!(clear(&mut a.doc, &plan).is_err());
    assert_eq!(a.doc.save(), before);
    assert!(record(&mut a.doc, "k0", &stale.observed["k0"], "bad", "stale").is_err());
    assert!(save_pair(&mut a.doc, "k0", "   ", "invalid").is_err());
    assert_eq!(a.doc.save(), before);
}

#[test]
fn history_clear_keeps_trash_groups_and_lifecycle_conflicts() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    scenario::choose(&mut a.doc, "s0", "trash").unwrap();
    save_pair(&mut b.doc, "k0", "Поздняя", "Правка").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let before = view(&a.doc).unwrap();
    assert_eq!(location(&before, "k0"), "conflicts");
    let plan = preview(&a.doc, "s0", None).unwrap();
    clear(&mut a.doc, &plan).unwrap();
    assert_eq!(view(&a.doc).unwrap(), before);
}

#[test]
fn saved_preclear_snapshot_keeps_its_history_and_a_new_edit_starts_fresh_history() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let backup = a.doc.save();
    let old = snapshot(&a.doc).unwrap();
    let plan = preview(&a.doc, "s0", Some("k0")).unwrap();
    clear(&mut a.doc, &plan).unwrap();
    save_pair(&mut a.doc, "k0", "После", "Новая версия").unwrap();
    assert_eq!(
        versions(&a.doc)
            .unwrap()
            .iter()
            .filter(|v| v.card == "k0")
            .count(),
        1
    );
    assert_eq!(snapshot(&Automerge::load(&backup).unwrap()).unwrap(), old);
}

#[test]
fn sqlite_abort_never_commits_history_without_content_or_clear_without_atomicity() {
    for operation in ["record", "card", "space", "purge"] {
        let (dir, mut store) = prepared();
        let mut a = store.read("A").unwrap();
        if operation == "purge" {
            scenario::choose(&mut a.doc, "s0", "trash").unwrap();
            store.write("A", &a).unwrap();
        }
        let before = [
            a.fingerprint().unwrap(),
            store.read("B").unwrap().fingerprint().unwrap(),
        ];
        let count = a.doc.get_changes(&[]).len();
        if operation == "record" {
            save_pair(&mut a.doc, "k0", "Атомарно", "Данные и версия").unwrap();
        } else if operation == "purge" {
            purge::purge(&mut a.doc, "s0", &["s0"], false).unwrap();
        } else {
            let plan = preview(
                &a.doc,
                "s0",
                if operation == "card" {
                    Some("k0")
                } else {
                    None
                },
            )
            .unwrap();
            clear(&mut a.doc, &plan).unwrap();
        }
        assert_eq!(a.doc.get_changes(&[]).len(), count + 1);
        store.0.execute_batch("CREATE TRIGGER fail AFTER UPDATE ON replicas BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(store.write("A", &a).is_err());
        drop(store);
        let mut reopened = Store::open(&dir.path().join("history.sqlite3")).unwrap();
        for (i, id) in ["A", "B"].iter().enumerate() {
            assert_eq!(reopened.read(id).unwrap().fingerprint().unwrap(), before[i]);
        }
        reopened.0.execute_batch("DROP TRIGGER fail;").unwrap();
        reopened.write("A", &a).unwrap();
        assert_eq!(
            reopened.read("A").unwrap().fingerprint().unwrap(),
            a.fingerprint().unwrap()
        );
    }
}

#[test]
fn three_history_phases_reopen_sqlite_and_keep_unseen_versions() {
    let dir = tempfile::tempdir().unwrap();
    for phase in ["history-prepare", "history-clear", "history-resume"] {
        run(dir.path(), phase).unwrap();
    }
}

#[test]
fn final_purge_clears_known_history_and_both_choices_keep_only_the_late_data() {
    for restore in [false, true] {
        let (_dir, store) = prepared();
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        scenario::choose(&mut a.doc, "s0", "trash").unwrap();
        purge::purge(&mut a.doc, "s0", &["s0"], false).unwrap();
        assert_eq!(versions(&a.doc).unwrap().len(), 1);
        save_pair(&mut b.doc, "k0", "Поздняя версия", "Сохраняется независимо").unwrap();
        a.doc.merge(&mut b.doc).unwrap();
        let plan = purge::late_preview(&a.doc, "s0").unwrap();
        purge::resolve(&mut a.doc, &plan, restore).unwrap();
        assert_eq!(
            versions(&a.doc)
                .unwrap()
                .iter()
                .filter(|v| v.card == "foreign")
                .count(),
            1
        );
        assert_eq!(
            versions(&a.doc)
                .unwrap()
                .iter()
                .filter(|v| v.card == "k0")
                .count(),
            usize::from(restore)
        );
        assert_eq!(
            versions(&a.doc)
                .unwrap()
                .iter()
                .filter(|v| v.card == "k1")
                .count(),
            0
        );
        assert_eq!(location(&view(&a.doc).unwrap(), "k1"), "purged");
    }
}

#[test]
fn first_independent_versions_use_one_shared_list_without_losing_either_branch() {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("first.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    save_pair(&mut a.doc, "k0", "A", "First A").unwrap();
    save_pair(&mut b.doc, "k0", "B", "First B").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    assert_eq!(versions(&a.doc).unwrap().len(), 2);
    assert_eq!(
        preview(&a.doc, "s0", None)
            .unwrap()
            .retained_conflicts
            .len(),
        2
    );
}
