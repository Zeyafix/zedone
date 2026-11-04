use super::*;

fn prepared() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("bridge.sqlite3")).unwrap();
    prepare(&mut store).unwrap();
    (directory, store)
}

#[test]
fn creation_cohort_requires_three_valid_names_and_preserves_independent_card_edits() {
    let (_directory, store) = prepared();
    let mut a = store.read("A").unwrap();
    let ids = ids(&a.doc).unwrap();
    let observed = view(&a.doc, &ids[0]).unwrap();
    let before = a.fingerprint().unwrap();
    for id in &ids {
        for card in ["k0", "k1"] {
            for present in [false, true] {
                assert!(assign(&mut a.doc, card, id, present).is_err());
            }
        }
        for names in [["Все", "Всё"], ["Всё", " ВСЁ "]] {
            assert!(
                resolve(
                    &mut a.doc,
                    id,
                    &observed,
                    &[(ids[0].clone(), names[0]), (ids[1].clone(), names[1])]
                )
                .is_err()
            );
        }
    }
    for names in [
        ["Все", "Всё", " ВСЁ "],
        ["Все", "Всё", " "],
        ["Все", "Всё", "Другой"],
    ] {
        assert!(choose(&mut a.doc, names).is_err());
    }
    assert_eq!(a.fingerprint().unwrap(), before);
    for field in CARD_FIELDS {
        let card = object(&a.doc, "k0").unwrap();
        let original = read_assignments(&a.doc, &card, field).unwrap()[0]
            .value
            .clone();
        let temporary = match field {
            "column" => object(&a.doc, "c0").unwrap().to_string(),
            "completed" => "false".to_owned(),
            _ => "Временная правка".to_owned(),
        };
        edit(&mut a.doc, "k0", field, &temporary).unwrap();
        edit(&mut a.doc, "k0", field, &original).unwrap();
    }
    let choices: Vec<_> = ids.iter().cloned().zip(["Все", "Всё", "Планы"]).collect();
    resolve(&mut a.doc, &ids[2], &observed, &choices).unwrap();
    verify(&a.doc).unwrap();
    assert!(tags(&a.doc).unwrap().iter().all(|v| !v.conflicting()));
    let before = a.fingerprint().unwrap();
    assert!(resolve(&mut a.doc, &ids[0], &observed, &choices).is_err());
    assert!(rename(&mut a.doc, &ids[0], " ВСЁ ").is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
}

#[test]
fn equal_and_late_competing_decisions_preserve_three_ids_and_all_name_variants() {
    for same in [false, true] {
        let (_directory, mut store) = prepared();
        let mut a = store.read("A").unwrap();
        let mut late = store.read("B").unwrap();
        choose(&mut a.doc, ["Все", "Всё", "Планы"]).unwrap();
        let ids = ids(&a.doc).unwrap();
        if !same {
            rename(&mut a.doc, &ids[0], "Будущее").unwrap();
        }
        choose(
            &mut late.doc,
            if same {
                ["Все", "Всё", "Планы"]
            } else {
                ["Дом", "Дела", "Архив"]
            },
        )
        .unwrap();
        a.doc.merge(&mut late.doc).unwrap();
        for (index, (id, name)) in ids.iter().zip(["Все", "Всё", "Планы"]).enumerate() {
            let current = tag(&a.doc, id).unwrap();
            assert!(current.creation_conflict.is_empty());
            assert_eq!(current.conflicting(), !same);
            assert_eq!(current.names.len(), 2);
            let local = if !same && index == 0 {
                "Будущее"
            } else {
                name
            };
            let remote = if same {
                name
            } else {
                ["Дом", "Дела", "Архив"][index]
            };
            for expected in [local, remote] {
                assert!(current.names.iter().any(|v| v.value == expected));
            }
            if !same {
                let observed = view(&a.doc, id).unwrap();
                resolve(&mut a.doc, id, &observed, &[(id.clone(), name)]).unwrap();
            }
        }
        verify(&a.doc).unwrap();
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
fn sqlite_abort_preserves_the_whole_unresolved_cohort_and_both_replicas() {
    let (directory, mut store) = prepared();
    let before = [
        store.read("A").unwrap().fingerprint().unwrap(),
        store.read("B").unwrap().fingerprint().unwrap(),
    ];
    let mut a = store.read("A").unwrap();
    choose(&mut a.doc, ["Все", "Всё", "Планы"]).unwrap();
    store
        .0
        .execute_batch(
            "CREATE TRIGGER reject_bridge AFTER UPDATE ON replicas
        BEGIN SELECT RAISE(ABORT, 'synthetic bridge failure'); END;",
        )
        .unwrap();
    assert!(store.write("A", &a).is_err());
    drop(store);
    let store = Store::open(&directory.path().join("bridge.sqlite3")).unwrap();
    for (peer, before) in ["A", "B"].into_iter().zip(before) {
        assert_eq!(store.read(peer).unwrap().fingerprint().unwrap(), before);
    }
}

#[test]
fn native_bridge_phases_preserve_the_cohort_and_resolutions_between_processes() {
    let directory = tempfile::tempdir().unwrap();
    for phase in ["bridge-prepare", "bridge-resolve", "bridge-resume"] {
        run(directory.path(), phase).unwrap();
    }
    assert!(run(directory.path(), "bridge-prepare").is_err());
}
