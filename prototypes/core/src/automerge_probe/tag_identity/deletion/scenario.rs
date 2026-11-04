use super::*;

pub(super) fn selected(doc: &Automerge) -> Result<Tag> {
    tags(doc)?
        .into_iter()
        .find(|v| v.ids.len() == 2)
        .ok_or_else(|| "Expected the original aliased tag".into())
}

pub(super) fn prepare(store: &mut Store) -> Result<ObjId> {
    seed(store)?;
    for peer in ["A", "B"] {
        let mut replica = store.read(peer)?;
        let home = object(&replica.doc, "s0")?.to_string();
        let id = create(
            &mut replica.doc,
            &home,
            if peer == "A" {
                "Работа"
            } else {
                "РАБОТА"
            },
        )?;
        assign(&mut replica.doc, "k0", &id, true)?;
        if peer == "A" {
            let other_home = object(&replica.doc, "s1")?.to_string();
            let other = create(&mut replica.doc, &other_home, "Работа")?;
            assign(&mut replica.doc, "k2", &other, true)?;
            let other = create(&mut replica.doc, &home, "Другой")?;
            assign(&mut replica.doc, "k1", &other, true)?;
        }
        store.write(peer, &replica)?;
    }
    exchange(store)?;
    let mut a = store.read("A")?;
    let group = selected(&a.doc)?;
    let observed = observe(&a.doc, &group.ids[0])?;
    decide(&mut a.doc, &group.ids[0], &observed, false)?;
    store.write("A", &a)?;
    let mut b = store.read("B")?;
    resolution::edit(&mut b.doc, "k0", "text", "Сохранённая независимая правка")?;
    assign(&mut b.doc, "k1", &group.ids[1], true)?;
    store.write("B", &b)?;
    exchange(store)?;
    Ok(group.ids[0].clone())
}

pub(super) fn verify(doc: &Automerge, keep: bool) -> Result<()> {
    let group = selected(doc)?;
    assert!(!group.deletion_conflict());
    assert_eq!(group.deleted(), !keep);
    for key in ["k0", "k1", "k2"] {
        let card = object(doc, key)?;
        assert!(!resolution::card_conflicting(doc, &card)?);
        for field in resolution::CARD_FIELDS {
            let values = read_assignments(doc, &card, field)?;
            let expected = match field {
                "title" => "Карточка — ёж 😀".to_owned(),
                "text" if key == "k0" => "Сохранённая независимая правка".to_owned(),
                "text" => "Исходный текст".to_owned(),
                "completed" => "false".to_owned(),
                "column" => object(doc, if key == "k2" { "c2" } else { "c0" })?.to_string(),
                _ => unreachable!(),
            };
            assert!(values.iter().all(|v| v.value == expected));
        }
        let memberships = links(doc, &card, &group)?;
        assert_eq!(
            memberships.iter().any(|v| v.value == "present"),
            keep && key != "k2"
        );
        if !keep {
            assert!(memberships.is_empty());
        }
    }
    for tag in tags(doc)?.iter().filter(|v| v.ids.len() == 1) {
        assert!(!tag.deleted());
        let key = if tag.home == group.home { "k1" } else { "k2" };
        assert!(
            links(doc, &object(doc, key)?, tag)?
                .iter()
                .all(|v| v.value == "present")
        );
        assert!(!links(doc, &object(doc, key)?, tag)?.is_empty());
    }
    Ok(())
}

pub(in crate::automerge_probe) fn run(directory: &Path, phase: &str) -> Result<()> {
    for keep in [false, true] {
        let label = if keep { "keep" } else { "delete" };
        let mut store =
            Store::open(&directory.join(format!("automerge-deletion-{label}.sqlite3")))?;
        let checkpoint = directory.join(format!("deletion-{label}-expected.json"));
        if phase == "deletion-prepare" {
            let count: i64 = store
                .0
                .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
            if count != 0 || checkpoint.exists() {
                return Err("Deletion prepare requires an empty store".into());
            }
            let id = prepare(&mut store)?;
            assert!(tag(&store.read("A")?.doc, &id)?.deletion_conflict());
        } else {
            let expected: Value = serde_json::from_slice(&std::fs::read(&checkpoint)?)?;
            for peer in ["A", "B"] {
                assert_eq!(store.read(peer)?.fingerprint()?, expected[peer]);
            }
            match phase {
                "deletion-resolve" => {
                    let mut a = store.read("A")?;
                    let group = selected(&a.doc)?;
                    let observed = observe(&a.doc, &group.ids[1])?;
                    decide(&mut a.doc, &group.ids[1], &observed, keep)?;
                    verify(&a.doc, keep)?;
                    store.write("A", &a)?;
                }
                "deletion-resume" => {
                    let old = store.read("B")?.doc.get_changes(&[]);
                    exchange(&mut store)?;
                    let mut a = store.read("A")?;
                    let before = a.fingerprint()?;
                    a.doc.apply_changes(old.clone())?;
                    a.doc.apply_changes(old)?;
                    assert_eq!(a.fingerprint()?, before);
                    verify(&a.doc, keep)?;
                    assert_eq!(snapshot(&a.doc)?, snapshot(&store.read("B")?.doc)?);
                }
                _ => return Err("Unknown deletion phase".into()),
            }
        }
        std::fs::write(
            checkpoint,
            serde_json::to_vec(&json!({
                "A": store.read("A")?.fingerprint()?, "B": store.read("B")?.fingerprint()?,
            }))?,
        )?;
    }
    println!("Automerge tag deletion: {phase} passed");
    Ok(())
}
