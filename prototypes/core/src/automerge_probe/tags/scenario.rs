use super::*;

fn expect(doc: &Automerge, card: &str, key: &str, expected: &[&str]) -> Result<()> {
    let tag = object(doc, key)?.to_string();
    let card = view(doc, card)?;
    let link = card.links.iter().find(|v| v.tag == tag).unwrap();
    let mut values: Vec<_> = link.variants.iter().map(|v| v.value.as_str()).collect();
    values.sort();
    values.dedup();
    assert_eq!(values, expected);
    Ok(())
}

pub(in super::super) fn run(directory: &Path, phase: &str) -> Result<()> {
    let mut store = Store::open(&directory.join("automerge-tags.sqlite3"))?;
    let checkpoint = directory.join("tags-expected.json");
    if phase == "tags-prepare" {
        let count: i64 = store
            .0
            .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
        if count != 0 || checkpoint.exists() {
            return Err("Tags prepare requires an empty disposable store".into());
        }
        seed(&mut store)?;
        for (id, key) in [("A", "t0"), ("B", "t1")] {
            let mut replica = store.read(id)?;
            let tag = object(&replica.doc, key)?.to_string();
            assign(&mut replica.doc, "k1", &tag, true, None)?;
            let tag = object(&replica.doc, "t0")?.to_string();
            assign(&mut replica.doc, "k0", &tag, false, None)?;
            if id == "B" {
                assign(&mut replica.doc, "k0", &tag, true, None)?;
            }
            store.write(id, &replica)?;
        }
        exchange(&mut store)?;
    } else {
        let expected: Value = serde_json::from_slice(&std::fs::read(&checkpoint)?)?;
        for id in ["A", "B"] {
            assert_eq!(store.read(id)?.fingerprint()?, expected[id]);
        }
        match phase {
            "tags-resolve" => {
                for (id, choice) in [("A", true), ("B", false)] {
                    let mut replica = store.read(id)?;
                    expect(&replica.doc, "k1", "t0", &["present"])?;
                    expect(&replica.doc, "k1", "t1", &["present"])?;
                    expect(&replica.doc, "k0", "t0", &["absent", "present"])?;
                    let card = view(&replica.doc, "k0")?;
                    let tag = object(&replica.doc, "t0")?.to_string();
                    assert!(edit(&mut replica.doc, "k0", Field::Text, "Blocked").is_err());
                    assert!(assign(&mut replica.doc, "k0", &tag, true, None).is_err());
                    edit(&mut replica.doc, "k1", Field::Text, "Доступная карточка")?;
                    assign(&mut replica.doc, "k0", &tag, choice, Some(&card))?;
                    store.write(id, &replica)?;
                }
            }
            "tags-resume" => {
                exchange(&mut store)?;
                let mut a = store.read("A")?;
                expect(&a.doc, "k0", "t0", &["absent", "present"])?;
                let card = view(&a.doc, "k0")?;
                let tag = object(&a.doc, "t0")?.to_string();
                assign(&mut a.doc, "k0", &tag, true, Some(&card))?;
                assign(&mut a.doc, "k0", &tag, false, None)?;
                assign(&mut a.doc, "k0", &tag, true, None)?;
                edit(&mut a.doc, "k0", Field::Text, "Исходный текст")?;
                expect(&a.doc, "k0", "t0", &["present"])?;
                store.write("A", &a)?;
                exchange(&mut store)?;
                assert_eq!(snapshot(&a.doc)?, snapshot(&store.read("B")?.doc)?);
                assert!(!view(&a.doc, "k0")?.blocked());
            }
            _ => return Err("Unknown tags phase".into()),
        }
    }
    std::fs::write(
        checkpoint,
        serde_json::to_vec(&json!({
            "A": store.read("A")?.fingerprint()?, "B": store.read("B")?.fingerprint()?,
        }))?,
    )?;
    println!("Automerge tag assignments: {phase} passed");
    Ok(())
}
