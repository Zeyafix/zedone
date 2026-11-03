use super::*;

pub(super) fn add(store: &mut Store, peer: &str, name: &str) -> Result<ObjId> {
    let mut replica = store.read(peer)?;
    let home = object(&replica.doc, "s0")?.to_string();
    let id = create(&mut replica.doc, &home, name)?;
    for card in ["k0", "k1"] {
        assign(&mut replica.doc, card, &id, true)?;
    }
    store.write(peer, &replica)?;
    Ok(id)
}

pub(super) fn prepare(store: &mut Store, against_creation: bool) -> Result<(ObjId, ObjId, ObjId)> {
    seed(store)?;
    let left = add(store, "A", "Работа")?;
    let other = add(store, "A", "Другой")?;
    let right = if against_creation {
        None
    } else {
        Some(add(store, "A", "Личное")?)
    };
    exchange(store)?;
    let mut a = store.read("A")?;
    rename(&mut a.doc, &left, "Важное")?;
    edit(&mut a.doc, "k0", "title", "Правка заголовка")?;
    edit(&mut a.doc, "k0", "completed", "true")?;
    store.write("A", &a)?;
    let right = if let Some(right) = right {
        let mut b = store.read("B")?;
        rename(&mut b.doc, &right, " ВАЖНОЕ ")?;
        store.write("B", &b)?;
        right
    } else {
        add(store, "B", " ВАЖНОЕ ")?
    };
    let mut b = store.read("B")?;
    edit(&mut b.doc, "k0", "text", "Правка описания")?;
    let column = object(&b.doc, "c1")?.to_string();
    edit(&mut b.doc, "k0", "column", &column)?;
    store.write("B", &b)?;
    exchange(store)?;
    Ok((left, right, other))
}

pub(super) fn verify_card(doc: &Automerge, left: &ObjId, right: &ObjId) -> Result<()> {
    let card = object(doc, "k0")?;
    for (field, value) in [
        ("title", "Правка заголовка".to_owned()),
        ("text", "Правка описания".to_owned()),
        ("completed", "true".to_owned()),
        ("column", object(doc, "c1")?.to_string()),
    ] {
        let variants = read_assignments(doc, &card, field)?;
        assert!(variants.iter().all(|v| v.value == value));
    }
    for id in [left, right] {
        for card in ["k0", "k1"] {
            let values = links(doc, &object(doc, card)?, &tag(doc, id)?)?;
            assert!(!values.is_empty());
            assert!(values.iter().all(|v| v.value == "present"));
        }
    }
    Ok(())
}

pub(in crate::automerge_probe) fn run(directory: &Path, phase: &str) -> Result<()> {
    for against_creation in [false, true] {
        let suffix = if against_creation {
            "creation"
        } else {
            "renames"
        };
        let mut store = Store::open(&directory.join(format!("automerge-names-{suffix}.sqlite3")))?;
        let checkpoint = directory.join(format!("names-{suffix}-expected.json"));
        if phase == "names-prepare" {
            let count: i64 = store
                .0
                .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
            if count != 0 || checkpoint.exists() {
                return Err("Names prepare requires an empty store".into());
            }
            let (left, right, _) = prepare(&mut store, against_creation)?;
            verify_card(&store.read("A")?.doc, &left, &right)?;
        } else {
            let expected: Value = serde_json::from_slice(&std::fs::read(&checkpoint)?)?;
            for peer in ["A", "B"] {
                assert_eq!(store.read(peer)?.fingerprint()?, expected[peer]);
            }
            if phase == "names-resume" {
                exchange(&mut store)?;
            }
            let catalog = tags(&store.read("A")?.doc)?;
            let disputed: Vec<_> = catalog.iter().filter(|v| v.conflicting()).collect();
            assert_eq!(disputed.len(), 2);
            let (left, right) = (disputed[0].ids[0].clone(), disputed[1].ids[0].clone());
            match phase {
                "names-resolve" => {
                    for (peer, names) in [("A", ["Все", "Всё"]), ("B", ["Планы", "Архив"])]
                    {
                        let mut replica = store.read(peer)?;
                        let before = replica.fingerprint()?;
                        for id in [&left, &right] {
                            for card in ["k0", "k1"] {
                                for present in [false, true] {
                                    assert!(assign(&mut replica.doc, card, id, present).is_err());
                                }
                            }
                        }
                        let observed = view(&replica.doc, &left)?;
                        assert!(
                            resolve(
                                &mut replica.doc,
                                &left,
                                &observed,
                                &[(left.clone(), "Всё"), (right.clone(), " ВСЁ ")]
                            )
                            .is_err()
                        );
                        assert_eq!(replica.fingerprint()?, before);
                        resolve(
                            &mut replica.doc,
                            &left,
                            &observed,
                            &[(left.clone(), names[0]), (right.clone(), names[1])],
                        )?;
                        verify_card(&replica.doc, &left, &right)?;
                        store.write(peer, &replica)?;
                    }
                }
                "names-resume" => {
                    let old = store.read("B")?.doc.get_changes(&[]);
                    let mut a = store.read("A")?;
                    assert!(tag(&a.doc, &left)?.conflicting());
                    assert!(tag(&a.doc, &right)?.conflicting());
                    for (id, name) in [(&left, "Все"), (&right, "Всё")] {
                        let observed = view(&a.doc, id)?;
                        resolve(&mut a.doc, id, &observed, &[(id.clone(), name)])?;
                    }
                    a.doc.apply_changes(old)?;
                    verify_card(&a.doc, &left, &right)?;
                    for id in [&left, &right] {
                        assert!(!tag(&a.doc, id)?.conflicting());
                        assign(&mut a.doc, "k0", id, false)?;
                        assign(&mut a.doc, "k0", id, true)?;
                    }
                    rename(&mut a.doc, &left, "Обратное переименование")?;
                    rename(&mut a.doc, &left, "Все")?;
                    store.write("A", &a)?;
                    exchange(&mut store)?;
                    assert_eq!(snapshot(&a.doc)?, snapshot(&store.read("B")?.doc)?);
                }
                _ => return Err("Unknown names phase".into()),
            }
        }
        std::fs::write(
            checkpoint,
            serde_json::to_vec(&json!({
                "A": store.read("A")?.fingerprint()?, "B": store.read("B")?.fingerprint()?,
            }))?,
        )?;
    }
    println!("Automerge tag names: {phase} passed");
    Ok(())
}
