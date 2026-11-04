use super::*;

fn prepared() -> (tempfile::TempDir, Store, ObjId) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("deletion.sqlite3")).unwrap();
    let id = scenario::prepare(&mut store).unwrap();
    (directory, store, id)
}

fn choose(doc: &mut Automerge, id: &ObjId, keep: bool) {
    let observed = observe(doc, id).unwrap();
    decide(doc, id, &observed, keep).unwrap();
}

#[test]
fn deletion_conflict_blocks_affected_cards_but_preserves_fields_and_both_choices() {
    for keep in [false, true] {
        let (_directory, store, id) = prepared();
        let mut a = store.read("A").unwrap();
        let before = a.fingerprint().unwrap();
        let group = tag(&a.doc, &id).unwrap();
        assert!(group.deletion_conflict());
        assert!(group.life.iter().any(|v| v.value == "alive"));
        assert!(group.life.iter().any(|v| v.value == "deleted"));
        let other = tags(&a.doc)
            .unwrap()
            .into_iter()
            .find(|v| v.home == group.home && v.ids.len() == 1)
            .unwrap();
        for card in ["k0", "k1"] {
            for field in resolution::CARD_FIELDS {
                assert!(resolution::edit(&mut a.doc, card, field, "blocked").is_err());
            }
            for alias in group.ids.iter().chain(&other.ids) {
                for present in [false, true] {
                    assert!(assign(&mut a.doc, card, alias, present).is_err());
                }
            }
        }
        for alias in &group.ids {
            assert!(rename(&mut a.doc, alias, "Новое имя").is_err());
        }
        let foreign = tags(&a.doc)
            .unwrap()
            .into_iter()
            .find(|v| v.home != group.home)
            .unwrap();
        let observed = observe(&a.doc, &id).unwrap();
        assert!(decide(&mut a.doc, &foreign.ids[0], &observed, keep).is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
        resolution::edit(&mut a.doc, "k2", "text", "Другое пространство").unwrap();
        resolution::edit(&mut a.doc, "k2", "text", "Исходный текст").unwrap();
        choose(&mut a.doc, &group.ids[1], keep);
        scenario::verify(&a.doc, keep).unwrap();
        if keep {
            for alias in &group.ids {
                assign(&mut a.doc, "k0", alias, false).unwrap();
                assign(&mut a.doc, "k0", alias, true).unwrap();
            }
            for field in resolution::CARD_FIELDS {
                let card = object(&a.doc, "k0").unwrap();
                let original = read_assignments(&a.doc, &card, field).unwrap()[0]
                    .value
                    .clone();
                let temporary = match field {
                    "column" => object(&a.doc, "c1").unwrap().to_string(),
                    "completed" => "true".to_owned(),
                    _ => "Временная правка".to_owned(),
                };
                resolution::edit(&mut a.doc, "k0", field, &temporary).unwrap();
                resolution::edit(&mut a.doc, "k0", field, &original).unwrap();
            }
        }
    }
}

#[test]
fn ordinary_deletion_and_recreation_never_restore_old_alias_memberships() {
    let (_directory, mut store, id) = prepared();
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, &id, true);
    let group = tag(&a.doc, &id).unwrap();
    let home = group.home.clone();
    let original = snapshot(&a.doc).unwrap()["cards"].clone();
    choose(&mut a.doc, &group.ids[1], false);
    scenario::verify(&a.doc, false).unwrap();
    for alias in &group.ids {
        let observed = observe(&a.doc, alias).unwrap();
        let before = a.fingerprint().unwrap();
        assert!(decide(&mut a.doc, alias, &observed, true).is_err());
        assert!(decide(&mut a.doc, alias, &observed, false).is_err());
        assert!(rename(&mut a.doc, alias, "Ещё имя").is_err());
        assert!(assign(&mut a.doc, "k0", alias, true).is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
    }
    let recreated = create(&mut a.doc, &home, " работа ").unwrap();
    assert!(!group.ids.contains(&recreated));
    for card in ["k0", "k1"] {
        assert!(
            links(
                &a.doc,
                &object(&a.doc, card).unwrap(),
                &tag(&a.doc, &recreated).unwrap()
            )
            .unwrap()
            .is_empty()
        );
        assign(&mut a.doc, card, &recreated, true).unwrap();
    }
    choose(&mut a.doc, &recreated, false);
    let fresh = create(&mut a.doc, &home, "Работа").unwrap();
    assert_ne!(fresh, recreated);
    assert_eq!(tag(&a.doc, &fresh).unwrap().ids, vec![fresh.clone()]);
    for card in ["k0", "k1"] {
        assert!(
            links(
                &a.doc,
                &object(&a.doc, card).unwrap(),
                &tag(&a.doc, &fresh).unwrap()
            )
            .unwrap()
            .is_empty()
        );
    }
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    assert_eq!(
        snapshot(&a.doc).unwrap(),
        snapshot(&store.read("B").unwrap().doc).unwrap()
    );
    for (before, after) in original
        .as_array()
        .unwrap()
        .iter()
        .zip(snapshot(&a.doc).unwrap()["cards"].as_array().unwrap())
    {
        for field in resolution::CARD_FIELDS {
            assert_eq!(before[field], after[field]);
        }
    }
}

#[test]
fn stale_and_competing_deletion_choices_preserve_variants_under_reordered_delivery() {
    for same in [false, true] {
        let (_directory, mut store, id) = prepared();
        let mut a = store.read("A").unwrap();
        let mut late = store.read("B").unwrap();
        let observed = observe(&a.doc, &id).unwrap();
        choose(&mut a.doc, &id, true);
        // A resolved and continued editing before the other decision arrives.
        resolution::edit(&mut a.doc, "k0", "title", "Поздняя правка").unwrap();
        choose(&mut late.doc, &id, same);
        a.doc.merge(&mut late.doc).unwrap();
        assert_eq!(tag(&a.doc, &id).unwrap().deletion_conflict(), !same);
        let before = a.fingerprint().unwrap();
        assert!(decide(&mut a.doc, &id, &observed, false).is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
        if !same {
            assert!(resolution::edit(&mut a.doc, "k0", "text", "blocked").is_err());
            choose(&mut a.doc, &id, true);
        }
        let card = object(&a.doc, "k0").unwrap();
        assert_eq!(
            read_assignments(&a.doc, &card, "title").unwrap()[0].value,
            "Поздняя правка"
        );
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
}

#[test]
fn resolving_tag_deletion_does_not_clear_a_cards_own_text_conflict() {
    let (_directory, store, id) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    for replica in [&mut a, &mut b] {
        let card = object(&replica.doc, "k0").unwrap();
        let text = replica.doc.get_actor().to_string();
        write_assignment(&mut replica.doc, &card, "text", &text).unwrap();
    }
    a.doc.merge(&mut b.doc).unwrap();
    choose(&mut a.doc, &id, false);
    let card = object(&a.doc, "k0").unwrap();
    assert_eq!(read_assignments(&a.doc, &card, "text").unwrap().len(), 2);
    assert!(resolution::edit(&mut a.doc, "k0", "title", "blocked").is_err());
}

#[test]
fn sqlite_abort_leaves_both_replicas_unchanged_for_deletion_and_each_resolution() {
    for keep in [None, Some(false), Some(true)] {
        let (directory, mut store, id) = prepared();
        if keep.is_none() {
            let mut a = store.read("A").unwrap();
            choose(&mut a.doc, &id, true);
            store.write("A", &a).unwrap();
            exchange(&mut store).unwrap();
        }
        let before = [
            store.read("A").unwrap().fingerprint().unwrap(),
            store.read("B").unwrap().fingerprint().unwrap(),
        ];
        let mut a = store.read("A").unwrap();
        choose(&mut a.doc, &id, keep.unwrap_or(false));
        store.0.execute_batch("CREATE TRIGGER reject_deletion AFTER UPDATE ON replicas BEGIN SELECT RAISE(ABORT, 'synthetic deletion failure'); END;").unwrap();
        assert!(store.write("A", &a).is_err());
        drop(store);
        let store = Store::open(&directory.path().join("deletion.sqlite3")).unwrap();
        for (peer, before) in ["A", "B"].into_iter().zip(before) {
            assert_eq!(store.read(peer).unwrap().fingerprint().unwrap(), before);
        }
    }
}

#[test]
fn native_deletion_phases_preserve_both_choices_between_processes() {
    let directory = tempfile::tempdir().unwrap();
    for phase in ["deletion-prepare", "deletion-resolve", "deletion-resume"] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "deletion-prepare").is_err());
}

#[test]
fn ordinary_delete_observes_memberships_but_unrelated_tags_do_not_stale_it() {
    let (_directory, store, id) = prepared();
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, &id, true);
    let observed = observe(&a.doc, &id).unwrap();
    let before = a.fingerprint().unwrap();
    assert!(decide(&mut a.doc, &id, &observed, true).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    assign(&mut a.doc, "k1", &id, false).unwrap();
    let changed = a.fingerprint().unwrap();
    assert!(decide(&mut a.doc, &id, &observed, false).is_err());
    assert_eq!(a.fingerprint().unwrap(), changed);
    let observed = observe(&a.doc, &id).unwrap();
    let other = create(&mut a.doc, &observed.tag.home, "Новый тег").unwrap();
    assign(&mut a.doc, "k0", &other, true).unwrap();
    decide(&mut a.doc, &id, &observed, false).unwrap();
    assert!(tag(&a.doc, &id).unwrap().deleted());
    assert!(!tag(&a.doc, &other).unwrap().deleted());
}

#[test]
fn ordinary_delete_cannot_bypass_an_affected_cards_own_conflict() {
    let (_directory, store, id) = prepared();
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, &id, true);
    let mut b = a.doc.fork();
    let card = object(&a.doc, "k0").unwrap();
    write_assignment(&mut a.doc, &card, "title", "A").unwrap();
    write_assignment(&mut b, &card, "title", "B").unwrap();
    a.doc.merge(&mut b).unwrap();
    let observed = observe(&a.doc, &id).unwrap();
    let before = a.fingerprint().unwrap();
    assert!(decide(&mut a.doc, &id, &observed, false).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
}

#[test]
fn late_creation_after_delete_and_recreate_never_requires_renaming_a_tombstone() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("late.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    let home = object(&a.doc, "s0").unwrap().to_string();
    let old = create(&mut a.doc, &home, "Работа").unwrap();
    assign(&mut a.doc, "k0", &old, true).unwrap();
    choose(&mut a.doc, &old, false);
    let new = create(&mut a.doc, &home, "Работа").unwrap();
    let late = create(&mut b.doc, &home, "Работа").unwrap();
    assign(&mut b.doc, "k1", &late, true).unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    assert!(tag(&a.doc, &old).unwrap().deleted());
    let observed = resolution::view(&a.doc, &new).unwrap();
    assert_eq!(observed.len(), 2);
    resolution::resolve(
        &mut a.doc,
        &new,
        &observed,
        &[(new.clone(), "Новый"), (late.clone(), "Поздний")],
    )
    .unwrap();
    for id in [&new, &late] {
        assert!(!tag(&a.doc, id).unwrap().conflicting());
        assign(&mut a.doc, "k0", id, true).unwrap();
    }
    assert!(tag(&a.doc, &old).unwrap().deleted());
    assert!(
        links(
            &a.doc,
            &object(&a.doc, "k0").unwrap(),
            &tag(&a.doc, &old).unwrap()
        )
        .unwrap()
        .is_empty()
    );
}
