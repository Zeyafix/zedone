use super::*;

fn present(doc: &Automerge, card: &str, tag: &Tag) -> Result<bool> {
    let values = links(doc, &object(doc, card)?, tag)?;
    assert!(values.iter().all(|v| v.value == values[0].value));
    Ok(values.first().is_some_and(|v| v.value == "present"))
}

pub(in super::super) fn run(directory: &Path, phase: &str) -> Result<()> {
    let mut store = Store::open(&directory.join("automerge-tag-identity.sqlite3"))?;
    let checkpoint = directory.join("identity-expected.json");
    if phase == "identity-prepare" {
        let count: i64 = store
            .0
            .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
        if count != 0 || checkpoint.exists() {
            return Err("Identity prepare requires an empty disposable store".into());
        }
        seed(&mut store)?;
        for (id, name, card) in [("A", "РАБОТА", "k0"), ("B", "Работа", "k1")] {
            let mut replica = store.read(id)?;
            let home = object(&replica.doc, "s0")?.to_string();
            let tag = create(&mut replica.doc, &home, name)?;
            assign(&mut replica.doc, card, &tag, true)?;
            store.write(id, &replica)?;
        }
    } else {
        let expected: Value = serde_json::from_slice(&std::fs::read(&checkpoint)?)?;
        for id in ["A", "B"] {
            assert_eq!(store.read(id)?.fingerprint()?, expected[id]);
        }
        match phase {
            "identity-rename" => {
                exchange(&mut store)?;
                let mut a = store.read("A")?;
                let group = tags(&a.doc)?.remove(0);
                assert_eq!(tags(&a.doc)?.len(), 1);
                assert_eq!(group.ids.len(), 2);
                assert!(["РАБОТА", "Работа"].contains(&group.name()));
                assert!(!group.conflicting());
                assert!(present(&a.doc, "k0", &group)?);
                assert!(present(&a.doc, "k1", &group)?);
                rename(&mut a.doc, &group.ids[1], "ЛИЧНОЕ")?;
                assign(&mut a.doc, "k0", &group.ids[0], false)?;
                store.write("A", &a)?;
                // The peer remains at the merged, original spelling until restart.
            }
            "identity-resume" => {
                let old = store.read("B")?.doc.get_changes(&[]);
                exchange(&mut store)?;
                let mut a = store.read("A")?;
                a.doc.apply_changes(old)?;
                let group = tags(&a.doc)?.remove(0);
                assert_eq!(group.name(), "ЛИЧНОЕ");
                assert!(!present(&a.doc, "k0", &group)?);
                assert!(present(&a.doc, "k1", &group)?);
                assert_eq!(group.ids.len(), 2);
                rename(&mut a.doc, &group.ids[0], "РАБОТА")?;
                assign(&mut a.doc, "k0", &group.ids[1], true)?;
                store.write("A", &a)?;
                exchange(&mut store)?;
                assert_eq!(snapshot(&a.doc)?, snapshot(&store.read("B")?.doc)?);
                let group = tags(&a.doc)?.remove(0);
                assert_eq!(group.name(), "РАБОТА");
                assert!(present(&a.doc, "k0", &group)?);
            }
            _ => return Err("Unknown identity phase".into()),
        }
    }
    std::fs::write(
        checkpoint,
        serde_json::to_vec(&json!({
            "A": store.read("A")?.fingerprint()?, "B": store.read("B")?.fingerprint()?,
        }))?,
    )?;
    println!("Automerge tag identities: {phase} passed");
    Ok(())
}
