use super::*;

fn prepared(creation: bool) -> (tempfile::TempDir, Store, ObjId, ObjId, ObjId) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("names.sqlite3")).unwrap();
    let (left, right, other) = scenario::prepare(&mut store, creation).unwrap();
    (directory, store, left, right, other)
}

fn choose(doc: &mut Automerge, left: &ObjId, right: &ObjId, names: [&str; 2]) {
    let observed = view(doc, left).unwrap();
    resolve(
        doc,
        left,
        &observed,
        &[(left.clone(), names[0]), (right.clone(), names[1])],
    )
    .unwrap();
}

#[test]
fn both_collisions_preserve_all_links_and_allow_four_card_edits_and_reverses() {
    for creation in [false, true] {
        let (_directory, mut store, left, right, other) = prepared(creation);
        let mut a = store.read("A").unwrap();
        for id in [&left, &right] {
            assert_eq!(tag(&a.doc, id).unwrap().ids, std::slice::from_ref(id));
            assert!(tag(&a.doc, id).unwrap().conflicting());
            for card in ["k0", "k1"] {
                for present in [false, true] {
                    let before = a.fingerprint().unwrap();
                    assert!(assign(&mut a.doc, card, id, present).is_err());
                    assert_eq!(a.fingerprint().unwrap(), before);
                }
            }
        }
        let observed = view(&a.doc, &left).unwrap();
        for field in CARD_FIELDS {
            let card = object(&a.doc, "k0").unwrap();
            let value = read_assignments(&a.doc, &card, field).unwrap()[0]
                .value
                .clone();
            let temporary = match field {
                "column" => object(&a.doc, "c0").unwrap().to_string(),
                "completed" => "false".to_owned(),
                _ => "Временная правка".to_owned(),
            };
            edit(&mut a.doc, "k0", field, &temporary).unwrap();
            edit(&mut a.doc, "k0", field, &value).unwrap();
        }
        assign(&mut a.doc, "k0", &other, false).unwrap();
        assign(&mut a.doc, "k0", &other, true).unwrap();
        // Card edits/links are not part of the observed name catalog.
        resolve(
            &mut a.doc,
            &left,
            &observed,
            &[(left.clone(), "Все"), (right.clone(), "Всё")],
        )
        .unwrap();
        scenario::verify_card(&a.doc, &left, &right).unwrap();
        for id in [&left, &right] {
            assert!(!tag(&a.doc, id).unwrap().conflicting());
            assign(&mut a.doc, "k0", id, false).unwrap();
            assign(&mut a.doc, "k0", id, true).unwrap();
        }
        rename(&mut a.doc, &left, "Обратно").unwrap();
        rename(&mut a.doc, &left, "Все").unwrap();
        let before = a.fingerprint().unwrap();
        assert!(rename(&mut a.doc, &left, " ВСЁ ").is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
        store.write("A", &a).unwrap();
        exchange(&mut store).unwrap();
        assert_eq!(
            snapshot(&a.doc).unwrap(),
            snapshot(&store.read("B").unwrap().doc).unwrap()
        );
    }
}

#[test]
fn incomplete_duplicate_foreign_empty_and_occupied_names_never_write() {
    let (_directory, store, left, right, other) = prepared(false);
    let mut a = store.read("A").unwrap();
    let observed = view(&a.doc, &left).unwrap();
    let before = a.fingerprint().unwrap();
    for choices in [
        vec![],
        vec![(left.clone(), "Все")],
        vec![(left.clone(), "Все"), (left.clone(), "Всё")],
        vec![(left.clone(), "Всё"), (right.clone(), " ВСЁ ")],
        vec![(left.clone(), " \t\n"), (right.clone(), "Всё")],
        vec![(left.clone(), " Другой "), (right.clone(), "Всё")],
        vec![(left.clone(), "Все"), (other.clone(), "Всё")],
        vec![(ROOT, "Все"), (right.clone(), "Всё")],
    ] {
        assert!(resolve(&mut a.doc, &left, &observed, &choices).is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
    }
    let foreign = object(&a.doc, "s1").unwrap().to_string();
    let foreign_id = create(&mut a.doc, &foreign, "Все").unwrap();
    let before = a.fingerprint().unwrap();
    assert!(
        resolve(
            &mut a.doc,
            &left,
            &observed,
            &[(left.clone(), "Все"), (foreign_id, "Всё")]
        )
        .is_err()
    );
    assert_eq!(a.fingerprint().unwrap(), before);
    // Names in another space do not occupy the local catalog or stale its view.
    resolve(
        &mut a.doc,
        &left,
        &observed,
        &[(left.clone(), "Все"), (right.clone(), "Всё")],
    )
    .unwrap();
    let before = a.fingerprint().unwrap();
    let observed = view(&a.doc, &left).unwrap();
    assert!(resolve(&mut a.doc, &left, &observed, &[]).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
}

#[test]
fn changed_operation_ids_or_a_new_name_in_the_same_space_make_a_choice_stale() {
    for new_tag in [false, true] {
        let (_directory, store, left, right, other) = prepared(false);
        let mut a = store.read("A").unwrap();
        let observed = view(&a.doc, &left).unwrap();
        if new_tag {
            let home = object(&a.doc, "s0").unwrap().to_string();
            create(&mut a.doc, &home, "Новое").unwrap();
        } else {
            // Same displayed value, new operation: a value-only check is unsafe.
            rename(&mut a.doc, &other, "Другой").unwrap();
        }
        let before = a.fingerprint().unwrap();
        assert!(
            resolve(
                &mut a.doc,
                &left,
                &observed,
                &[(left.clone(), "Все"), (right, "Всё")]
            )
            .is_err()
        );
        assert_eq!(a.fingerprint().unwrap(), before);
    }
}

#[test]
fn own_card_conflicts_survive_name_resolution_and_block_all_ordinary_edits() {
    for field in CARD_FIELDS.into_iter().chain(["membership"]) {
        let (_directory, mut store, left, right, other) = prepared(false);
        for (peer, value) in [("A", "Первый"), ("B", "Второй")] {
            let mut replica = store.read(peer).unwrap();
            let card = object(&replica.doc, "k0").unwrap();
            if field == "membership" {
                assign(&mut replica.doc, "k0", &other, false).unwrap();
                if peer == "B" {
                    assign(&mut replica.doc, "k0", &other, true).unwrap();
                }
            } else {
                let value = match field {
                    "completed" => (peer == "A").to_string(),
                    "column" => object(&replica.doc, if peer == "A" { "c0" } else { "c1" })
                        .unwrap()
                        .to_string(),
                    _ => value.to_owned(),
                };
                // Direct fixture setup isolates each own-field conflict.
                write_assignment(&mut replica.doc, &card, field, &value).unwrap();
            }
            store.write(peer, &replica).unwrap();
        }
        exchange(&mut store).unwrap();
        let mut a = store.read("A").unwrap();
        choose(&mut a.doc, &left, &right, ["Все", "Всё"]);
        let before = a.fingerprint().unwrap();
        for key in CARD_FIELDS {
            assert!(edit(&mut a.doc, "k0", key, "Blocked").is_err());
        }
        for id in [&left, &right, &other] {
            assert!(assign(&mut a.doc, "k0", id, false).is_err());
        }
        assert_eq!(a.fingerprint().unwrap(), before);
        edit(&mut a.doc, "k1", "text", "Другая карточка доступна").unwrap();
    }
}

#[test]
fn concurrent_identical_decisions_converge_and_different_decisions_remain_variants() {
    for same in [false, true] {
        let (_directory, mut store, left, right, _) = prepared(true);
        for peer in ["A", "B"] {
            let mut replica = store.read(peer).unwrap();
            choose(
                &mut replica.doc,
                &left,
                &right,
                if same || peer == "A" {
                    ["Все", "Всё"]
                } else {
                    ["Планы", "Архив"]
                },
            );
            store.write(peer, &replica).unwrap();
        }
        exchange(&mut store).unwrap();
        let mut a = store.read("A").unwrap();
        for id in [&left, &right] {
            assert_eq!(tag(&a.doc, id).unwrap().conflicting(), !same);
            assert_eq!(tag(&a.doc, id).unwrap().names.len(), 2);
        }
        if !same {
            let observed = view(&a.doc, &left).unwrap();
            resolve(&mut a.doc, &left, &observed, &[(left.clone(), "Все")]).unwrap();
            assert!(tag(&a.doc, &right).unwrap().conflicting());
            let observed = view(&a.doc, &right).unwrap();
            resolve(&mut a.doc, &right, &observed, &[(right.clone(), "Всё")]).unwrap();
        }
        scenario::verify_card(&a.doc, &left, &right).unwrap();
    }
}

#[test]
fn foreign_column_invalid_status_and_unknown_card_field_are_rejected_without_writes() {
    let (_directory, store, _, _, _) = prepared(false);
    let mut a = store.read("A").unwrap();
    let before = a.fingerprint().unwrap();
    let foreign = object(&a.doc, "c2").unwrap().to_string();
    for (card, field, value) in [
        ("k0", "column", foreign.as_str()),
        ("k0", "column", "unknown"),
        ("k0", "completed", "maybe"),
        ("unknown", "text", "no"),
        ("k0", "home", "no"),
    ] {
        assert!(edit(&mut a.doc, card, field, value).is_err());
        assert_eq!(a.fingerprint().unwrap(), before);
    }
}

#[test]
fn native_names_phases_preserve_unresolved_and_competing_decisions_between_processes() {
    let directory = tempfile::tempdir().unwrap();
    for phase in ["names-prepare", "names-resolve", "names-resume"] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "names-prepare").is_err());
}

#[test]
fn a_late_autonomous_decision_is_preserved_after_local_resolution_and_rename() {
    let (_directory, store, left, right, _) = prepared(true);
    let mut a = store.read("A").unwrap();
    let mut late = store.read("B").unwrap();
    choose(&mut a.doc, &left, &right, ["Все", "Всё"]);
    rename(&mut a.doc, &left, "Планы").unwrap();
    choose(&mut late.doc, &left, &right, ["Архив", "Будущее"]);
    a.doc.merge(&mut late.doc).unwrap();
    let names: Vec<_> = tag(&a.doc, &left)
        .unwrap()
        .names
        .into_iter()
        .map(|v| v.value)
        .collect();
    assert_eq!(names.len(), 2);
    assert!(names.contains(&"Планы".to_owned()));
    assert!(names.contains(&"Архив".to_owned()));
    for (id, name) in [(&left, "Все"), (&right, "Всё")] {
        let observed = view(&a.doc, id).unwrap();
        resolve(&mut a.doc, id, &observed, &[(id.clone(), name)]).unwrap();
    }
    scenario::verify_card(&a.doc, &left, &right).unwrap();
}

#[test]
fn one_choice_covers_every_tag_in_the_name_collision() {
    let (_directory, mut store, left, right, third) = prepared(false);
    let mut late = store.read("B").unwrap();
    let initial = store.read("A").unwrap();
    // Fork the common state known immediately before the first offline rename.
    let context = tag(&initial.doc, &left).unwrap().names[0].context.clone();
    let mut third_doc = initial.doc.fork_at(&context).unwrap();
    third_doc.set_actor(automerge::ActorId::random());
    rename(&mut third_doc, &third, "Важное").unwrap();
    late.doc.merge(&mut third_doc).unwrap();
    store.write("B", &late).unwrap();
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let observed = view(&a.doc, &left).unwrap();
    assert!(
        resolve(
            &mut a.doc,
            &left,
            &observed,
            &[(left.clone(), "Все"), (right.clone(), "Всё")]
        )
        .is_err()
    );
    resolve(
        &mut a.doc,
        &left,
        &observed,
        &[
            (left.clone(), "Все"),
            (right.clone(), "Всё"),
            (third.clone(), "Другое"),
        ],
    )
    .unwrap();
    for id in [&left, &right, &third] {
        assert!(!tag(&a.doc, id).unwrap().conflicting());
    }
}

#[test]
fn sqlite_failure_keeps_both_replicas_and_all_names_unchanged_after_reopen() {
    let (directory, mut store, left, right, _) = prepared(true);
    let before_a = store.read("A").unwrap().fingerprint().unwrap();
    let before_b = store.read("B").unwrap().fingerprint().unwrap();
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, &left, &right, ["Все", "Всё"]);
    edit(&mut a.doc, "k1", "text", "Не должна сохраниться частично").unwrap();
    store
        .0
        .execute_batch(
            "CREATE TRIGGER reject_names AFTER UPDATE ON replicas
        BEGIN SELECT RAISE(ABORT, 'synthetic names failure'); END;",
        )
        .unwrap();
    assert!(store.write("A", &a).is_err());
    drop(store);
    let store = Store::open(&directory.path().join("names.sqlite3")).unwrap();
    assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), before_a);
    assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), before_b);
}

#[test]
fn repeated_and_reordered_changes_preserve_all_choices_and_later_reverses() {
    for creation in [false, true] {
        let (_directory, mut store, left, right, _) = prepared(creation);
        let old = store.read("B").unwrap().doc.get_changes(&[]);
        let mut a = store.read("A").unwrap();
        choose(&mut a.doc, &left, &right, ["Все", "Всё"]);
        rename(&mut a.doc, &left, "Новое").unwrap();
        assign(&mut a.doc, "k0", &right, false).unwrap();
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
}

#[test]
fn merged_tag_aliases_address_one_choice_and_cannot_be_chosen_twice() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("aliases.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    let left = scenario::add(&mut store, "A", "Работа").unwrap();
    let alias = scenario::add(&mut store, "B", " РАБОТА ").unwrap();
    exchange(&mut store).unwrap();
    let right = scenario::add(&mut store, "A", "Личное").unwrap();
    exchange(&mut store).unwrap();
    for (peer, id) in [("A", &left), ("B", &right)] {
        let mut replica = store.read(peer).unwrap();
        rename(&mut replica.doc, id, "Важное").unwrap();
        store.write(peer, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let observed = view(&a.doc, &left).unwrap();
    let before = a.fingerprint().unwrap();
    assert!(
        resolve(
            &mut a.doc,
            &alias,
            &observed,
            &[(left.clone(), "Все"), (alias.clone(), "Всё")]
        )
        .is_err()
    );
    assert_eq!(a.fingerprint().unwrap(), before);
    resolve(
        &mut a.doc,
        &alias,
        &observed,
        &[(alias.clone(), "Все"), (right.clone(), "Всё")],
    )
    .unwrap();
    assert_eq!(tag(&a.doc, &left).unwrap().name(), "Все");
    assert_eq!(tag(&a.doc, &alias).unwrap().ids.len(), 2);
    for id in [&left, &right] {
        for card in ["k0", "k1"] {
            let values = links(
                &a.doc,
                &object(&a.doc, card).unwrap(),
                &tag(&a.doc, id).unwrap(),
            )
            .unwrap();
            assert!(!values.is_empty());
            assert!(values.iter().all(|v| v.value == "present"));
        }
    }
    assign(&mut a.doc, "k0", &left, false).unwrap();
    let values = links(
        &a.doc,
        &object(&a.doc, "k0").unwrap(),
        &tag(&a.doc, &alias).unwrap(),
    )
    .unwrap();
    assert!(values.iter().all(|v| v.value == "absent"));
}

#[test]
fn name_choice_does_not_silently_decide_ambiguous_creation_identities() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("bridge.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    let left = scenario::add(&mut store, "A", "Работа").unwrap();
    let mut a = store.read("A").unwrap();
    rename(&mut a.doc, &left, "Личное").unwrap();
    store.write("A", &a).unwrap();
    let next = scenario::add(&mut store, "A", "Работа").unwrap();
    let late = scenario::add(&mut store, "B", "Работа").unwrap();
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let before = a.fingerprint().unwrap();
    let observed = view(&a.doc, &next).unwrap();
    assert!(
        resolve(
            &mut a.doc,
            &next,
            &observed,
            &[(next.clone(), "Все"), (late, "Всё")]
        )
        .is_err()
    );
    assert_eq!(a.fingerprint().unwrap(), before);
    assert!(tag(&a.doc, &left).unwrap().identity_ambiguous);
}
