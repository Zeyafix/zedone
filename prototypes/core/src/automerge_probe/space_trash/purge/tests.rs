use super::*;

fn prepared() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("purge.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    (dir, store)
}

fn choose(doc: &mut Automerge, key: &str, state: &str) {
    scenario::choose(doc, key, state).unwrap();
}

fn state(doc: &Automerge, key: &str) -> &'static str {
    location(&view(doc).unwrap(), key)
}

fn reject(doc: &mut Automerge, action: impl FnOnce(&mut Automerge) -> Result<()>) {
    let before = doc.save();
    assert!(action(doc).is_err());
    assert_eq!(doc.save(), before);
}

#[test]
fn column_purge_includes_earlier_trash_but_excludes_a_recovered_card_and_other_space() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, "c0", "trash");
    let nodes = view(&a.doc).unwrap();
    set_card(&mut a.doc, "k1", &nodes["k1"], "active", "c1").unwrap();
    let before = view(&a.doc).unwrap();
    let plan = preview(&a.doc, "s0", &["c0", "old", "c0"], false).unwrap();
    assert_eq!(plan.targets, ["c0", "k0", "old"]);
    assert!(plan.conflicts.is_empty());
    apply(&mut a.doc, &plan).unwrap();
    let after = view(&a.doc).unwrap();
    for key in ["c0", "k0", "old"] {
        assert_eq!(state(&a.doc, key), "purged");
        assert_eq!(after[key].id, before[key].id);
        assert!(snapshot(&a.doc).unwrap()["visible"].get(key).is_none());
    }
    for key in ["k1", "c1", "other", "foreign", "old-col-card"] {
        assert_eq!(after[key], before[key]);
    }
    assert_eq!(state(&a.doc, "k1"), "board");
}

#[test]
fn whole_space_and_whole_trash_have_exact_scopes() {
    for all in [false, true] {
        let (_dir, store) = prepared();
        let mut a = store.read("A").unwrap();
        if !all {
            choose(&mut a.doc, "s0", "trash");
        }
        let plan = preview(&a.doc, "s0", if all { &[] } else { &["s0"] }, all).unwrap();
        if all {
            assert_eq!(plan.targets, ["old", "old-col", "old-col-card"]);
        } else {
            assert_eq!(plan.targets.len(), 10);
        }
        apply(&mut a.doc, &plan).unwrap();
        assert_eq!(state(&a.doc, "k0"), if all { "board" } else { "purged" });
        assert_eq!(state(&a.doc, "foreign"), "board");
        for key in ["old", "old-col", "old-col-card"] {
            assert_eq!(state(&a.doc, key), "purged");
        }
        reject(&mut a.doc, |d| apply(d, &plan));
    }
}

#[test]
fn nested_conflicts_abort_every_selected_object_and_return_all_blockers() {
    for all in [false, true] {
        let (_dir, store) = prepared();
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        scenario::change(&mut a.doc, "k0", "text", "A").unwrap();
        scenario::change(&mut b.doc, "k0", "text", "B").unwrap();
        choose(&mut a.doc, "c0", "trash");
        a.doc.merge(&mut b.doc).unwrap();
        let plan = preview(
            &a.doc,
            "s0",
            if all { &[] } else { &["c0", "old-col"] },
            all,
        )
        .unwrap();
        assert!(plan.conflicts.contains(&"k0".into()));
        let before = view(&a.doc).unwrap();
        reject(&mut a.doc, |d| apply(d, &plan));
        assert_eq!(view(&a.doc).unwrap(), before);
        assert_eq!(state(&a.doc, "old-col-card"), "trash");
    }
}

#[test]
fn invalid_stale_foreign_active_and_empty_selections_never_write() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    for (space, keys, all) in [
        ("s0", vec!["foreign"], false),
        ("s0", vec!["k0"], false),
        ("c0", vec!["old"], false),
        ("s0", vec!["missing"], false),
        ("s0", vec!["old"], true),
    ] {
        reject(&mut a.doc, |d| purge(d, space, &keys, all));
    }
    reject(&mut a.doc, |d| purge(d, "s0", &[], false));
    let plan = preview(&a.doc, "s0", &[], true).unwrap();
    scenario::change(&mut a.doc, "k0", "completed", "true").unwrap();
    reject(&mut a.doc, |d| apply(d, &plan));
    let plan = preview(&a.doc, "s0", &[], true).unwrap();
    scenario::change(&mut a.doc, "foreign", "text", "unrelated").unwrap();
    apply(&mut a.doc, &plan).unwrap();
}

#[test]
fn every_late_field_change_keeps_ids_and_only_restores_necessary_ancestors() {
    for ((field, _), value) in CARD_FIELDS.into_iter().zip(scenario::EDITED) {
        for restore in [false, true] {
            let (_dir, store) = prepared();
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            choose(&mut a.doc, "s0", "trash");
            purge(&mut a.doc, "s0", &["s0"], false).unwrap();
            scenario::change(&mut b.doc, "k0", field, value).unwrap();
            let edited = view(&b.doc).unwrap();
            a.doc.merge(&mut b.doc).unwrap();
            assert_eq!(state(&a.doc, "k0"), "conflicts");
            assert_eq!(state(&a.doc, "k1"), "purged");
            let plan = late_preview(&a.doc, "s0").unwrap();
            assert_eq!(plan.targets, ["c0", "k0", "s0"]);
            reject(&mut a.doc, |d| {
                scenario::change(d, "k0", "title", "ordinary")
            });
            reject(&mut a.doc, |d| scenario::choose(d, "s0", "active"));
            resolve(&mut a.doc, &plan, restore).unwrap();
            assert_eq!(
                state(&a.doc, "k0"),
                if restore { "board" } else { "purged" }
            );
            for key in ["k1", "other", "old", "old-col-card"] {
                assert_eq!(state(&a.doc, key), "purged");
            }
            assert_eq!(view(&a.doc).unwrap()["k0"].fields, edited["k0"].fields);
            assert_eq!(view(&a.doc).unwrap()["k0"].id, edited["k0"].id);
            reject(&mut a.doc, |d| resolve(d, &plan, restore));
        }
    }
}

#[test]
fn late_new_objects_under_purged_column_and_space_restore_minimally() {
    for container in ["c0", "s0"] {
        for restore in [false, true] {
            let (_dir, store) = prepared();
            let mut a = store.read("A").unwrap();
            let mut b = store.read("B").unwrap();
            choose(&mut a.doc, container, "trash");
            purge(&mut a.doc, "s0", &[container], false).unwrap();
            create(&mut b.doc, "new-b", "c0").unwrap();
            if container == "s0" {
                create(&mut b.doc, "new-col", "s0").unwrap();
                create(&mut b.doc, "new-a", "new-col").unwrap();
            }
            a.doc.merge(&mut b.doc).unwrap();
            let plan = late_preview(&a.doc, "s0").unwrap();
            assert!(plan.targets.contains(&"new-b".into()));
            assert!(!plan.targets.contains(&"k0".into()));
            resolve(&mut a.doc, &plan, restore).unwrap();
            assert_eq!(
                state(&a.doc, "new-b"),
                if restore { "board" } else { "purged" }
            );
            for key in ["k0", "k1", "old"] {
                assert_eq!(state(&a.doc, key), "purged");
            }
            assert_eq!(
                state(&a.doc, "other"),
                if container == "s0" { "purged" } else { "board" }
            );
        }
    }
}

#[test]
fn purged_objects_cannot_use_ordinary_trash_recovery_or_creation_paths() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, "s0", "trash");
    purge(&mut a.doc, "s0", &["s0"], false).unwrap();
    reject(&mut a.doc, |d| scenario::choose(d, "s0", "active"));
    reject(&mut a.doc, |d| scenario::choose(d, "c0", "active"));
    reject(&mut a.doc, |d| {
        set_card(d, "k0", &view(d)?["k0"], "active", "c1")
    });
    reject(&mut a.doc, |d| create(d, "new-col", "s0"));
    reject(&mut a.doc, |d| {
        scenario::change(d, "k0", "text", "ordinary")
    });
}

#[test]
fn different_equal_and_unseen_late_decisions_preserve_variants() {
    for right in [false, true] {
        let (_dir, store) = prepared();
        let mut a = store.read("A").unwrap();
        let mut old = store.read("B").unwrap();
        choose(&mut a.doc, "s0", "trash");
        purge(&mut a.doc, "s0", &["s0"], false).unwrap();
        scenario::change(&mut old.doc, "k0", "text", "late").unwrap();
        a.doc.merge(&mut old.doc).unwrap();
        let mut b = a.doc.fork();
        resolve(&mut a.doc, &late_preview(&b, "s0").unwrap(), true).unwrap();
        let plan = late_preview(&b, "s0").unwrap();
        resolve(&mut b, &plan, right).unwrap();
        a.doc.merge(&mut b).unwrap();
        assert_eq!(
            state(&a.doc, "k0"),
            if right { "board" } else { "conflicts" }
        );
        if !right {
            let plan = late_preview(&a.doc, "s0").unwrap();
            resolve(&mut a.doc, &plan, false).unwrap();
        }
        assert_eq!(state(&a.doc, "k1"), "purged");
    }
}

#[test]
fn a_previously_unseen_replica_reopens_conflict_but_replays_do_not() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut old = store.read("B").unwrap().doc.fork();
    let mut b = store.read("B").unwrap();
    let initial = b.doc.get_changes(&[]);
    choose(&mut a.doc, "s0", "trash");
    purge(&mut a.doc, "s0", &["s0"], false).unwrap();
    scenario::change(&mut b.doc, "k0", "text", "seen").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let plan = late_preview(&a.doc, "s0").unwrap();
    resolve(&mut a.doc, &plan, false).unwrap();
    let changes = a.doc.get_changes(&[]);
    let mut reloaded = Automerge::load(&a.doc.save()).unwrap();
    for batch in [initial, changes.clone()] {
        reloaded.apply_changes(batch.into_iter().rev()).unwrap();
    }
    assert_eq!(state(&reloaded, "k0"), "purged");
    scenario::change(&mut old, "k1", "text", "unseen").unwrap();
    reloaded.merge(&mut old).unwrap();
    assert_eq!(state(&reloaded, "k0"), "purged");
    assert_eq!(state(&reloaded, "k1"), "conflicts");
    let plan = late_preview(&reloaded, "s0").unwrap();
    assert_eq!(plan.targets, ["c0", "k1", "s0"]);
    resolve(&mut reloaded, &plan, true).unwrap();
    let mut rebuilt = Automerge::new();
    rebuilt
        .apply_changes(reloaded.get_changes(&[]).into_iter().rev())
        .unwrap();
    assert_eq!(snapshot(&rebuilt).unwrap(), snapshot(&reloaded).unwrap());
}

#[test]
fn resolving_purge_does_not_resolve_independent_text_or_place_conflicts() {
    for restore in [false, true] {
        let (_dir, store) = prepared();
        let mut a = store.read("A").unwrap();
        let mut b = store.read("B").unwrap();
        let mut c = b.doc.fork();
        choose(&mut a.doc, "s0", "trash");
        purge(&mut a.doc, "s0", &["s0"], false).unwrap();
        scenario::change(&mut b.doc, "k0", "text", "left").unwrap();
        scenario::change(&mut b.doc, "k0", "place", "c0:100").unwrap();
        scenario::change(&mut c, "k0", "text", "right").unwrap();
        scenario::change(&mut c, "k0", "place", "c0:200").unwrap();
        a.doc.merge(&mut b.doc).unwrap();
        a.doc.merge(&mut c).unwrap();
        let before = view(&a.doc).unwrap()["k0"].fields.clone();
        let plan = late_preview(&a.doc, "s0").unwrap();
        resolve(&mut a.doc, &plan, restore).unwrap();
        let after = view(&a.doc).unwrap();
        assert_eq!(after["k0"].fields, before);
        assert!(after["k0"].blocked());
        assert_eq!(
            location(&after, "k0"),
            if restore { "conflicts" } else { "purged" }
        );
        reject(&mut a.doc, |d| {
            scenario::change(d, "k0", "completed", "true")
        });
    }
}

#[test]
fn sqlite_abort_rolls_back_purge_and_both_late_decisions_including_actor_peer() {
    for operation in ["purge", "restore", "confirm"] {
        let (dir, mut store) = prepared();
        let mut a = store.read("A").unwrap();
        choose(&mut a.doc, "s0", "trash");
        if operation != "purge" {
            purge(&mut a.doc, "s0", &["s0"], false).unwrap();
            let mut b = store.read("B").unwrap();
            scenario::change(&mut b.doc, "k0", "text", "late").unwrap();
            a.doc.merge(&mut b.doc).unwrap();
        }
        store.write("A", &a).unwrap();
        let before = [
            store.read("A").unwrap().fingerprint().unwrap(),
            store.read("B").unwrap().fingerprint().unwrap(),
        ];
        let count = a.doc.get_changes(&[]).len();
        if operation == "purge" {
            purge(&mut a.doc, "s0", &["s0"], false).unwrap();
        } else {
            let plan = late_preview(&a.doc, "s0").unwrap();
            resolve(&mut a.doc, &plan, operation == "restore").unwrap();
        }
        assert_eq!(a.doc.get_changes(&[]).len(), count + 1);
        store.0.execute_batch("CREATE TRIGGER fail AFTER UPDATE ON replicas BEGIN SELECT RAISE(ABORT, 'injected'); END;").unwrap();
        assert!(store.write("A", &a).is_err());
        drop(store);
        let mut reopened = Store::open(&dir.path().join("purge.sqlite3")).unwrap();
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
fn three_cli_phases_reopen_the_sqlite_source_without_loading_data_from_json() {
    let dir = tempfile::tempdir().unwrap();
    for phase in ["purge-prepare", "purge-resolve", "purge-resume"] {
        run(dir.path(), phase).unwrap();
    }
}

#[test]
fn a_late_creation_stays_in_conflicts_even_after_its_ancestors_were_restored() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    let mut old = b.doc.fork();
    choose(&mut a.doc, "s0", "trash");
    purge(&mut a.doc, "s0", &["s0"], false).unwrap();
    scenario::change(&mut b.doc, "k0", "text", "first late edit").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let plan = late_preview(&a.doc, "s0").unwrap();
    resolve(&mut a.doc, &plan, true).unwrap();
    // The old replica has never seen the purge, unlike a legitimate new creation.
    create(&mut old, "new-b", "c0").unwrap();
    a.doc.merge(&mut old).unwrap();
    assert_eq!(state(&a.doc, "s0"), "board");
    assert_eq!(state(&a.doc, "c0"), "board");
    assert_eq!(state(&a.doc, "new-b"), "conflicts");
    let plan = late_preview(&a.doc, "s0").unwrap();
    assert_eq!(plan.targets, ["new-b"]);
    resolve(&mut a.doc, &plan, true).unwrap();
    assert_eq!(state(&a.doc, "new-b"), "board");
    create(&mut a.doc, "new-a", "c0").unwrap();
    assert_eq!(state(&a.doc, "new-a"), "board");
    assert_eq!(state(&a.doc, "k1"), "purged");
    let mut reloaded = Automerge::load(&a.doc.save()).unwrap();
    reloaded
        .apply_changes(old.get_changes(&[]).into_iter().rev())
        .unwrap();
    assert_eq!(state(&reloaded, "new-b"), "board");
}

#[test]
fn late_preview_becomes_stale_and_repeated_purge_cycles_keep_their_boundaries() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    choose(&mut a.doc, "s0", "trash");
    purge(&mut a.doc, "s0", &["s0"], false).unwrap();
    scenario::change(&mut b.doc, "k0", "text", "late").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let stale = late_preview(&a.doc, "s0").unwrap();
    create(&mut b.doc, "new-a", "c0").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    reject(&mut a.doc, |d| resolve(d, &stale, true));
    let plan = late_preview(&a.doc, "s0").unwrap();
    resolve(&mut a.doc, &plan, true).unwrap();
    choose(&mut a.doc, "s0", "trash");
    purge(&mut a.doc, "s0", &["s0"], false).unwrap();
    assert_eq!(view(&a.doc).unwrap()["s0"].barriers.len(), 2);
    a.doc.merge(&mut b.doc).unwrap();
    assert_eq!(state(&a.doc, "k0"), "purged");
    assert_eq!(state(&a.doc, "new-a"), "purged");
}

#[test]
fn confirming_a_late_container_purge_records_a_boundary_for_future_late_creations() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    choose(&mut a.doc, "s0", "trash");
    purge(&mut a.doc, "s0", &["s0"], false).unwrap();
    create(&mut b.doc, "new-col", "s0").unwrap();
    create(&mut b.doc, "new-a", "new-col").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    let mut restored = a.doc.fork();
    let plan = late_preview(&a.doc, "s0").unwrap();
    resolve(&mut a.doc, &plan, false).unwrap();
    resolve(&mut restored, &plan, true).unwrap();
    let mut old = restored.fork();
    a.doc.merge(&mut restored).unwrap();
    let plan = late_preview(&a.doc, "s0").unwrap();
    resolve(&mut a.doc, &plan, true).unwrap();
    create(&mut old, "new-b", "new-col").unwrap();
    a.doc.merge(&mut old).unwrap();
    assert_eq!(state(&a.doc, "new-col"), "board");
    assert_eq!(state(&a.doc, "new-b"), "conflicts");
    let plan = late_preview(&a.doc, "s0").unwrap();
    assert_eq!(plan.targets, ["new-b"]);
    resolve(&mut a.doc, &plan, false).unwrap();
    assert_eq!(state(&a.doc, "new-b"), "purged");
}
