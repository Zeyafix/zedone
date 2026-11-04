//! The late creation connects two causally distinct tags; all three stay separate.
use super::*;

fn ids(doc: &Automerge) -> Result<Vec<ObjId>> {
    let mut ids = Vec::new();
    for tag in tags(doc)? {
        if name_key(&scalar(doc, &tag.ids[0], "birth")?) == "работа" {
            ids.push(tag.ids[0].clone());
        }
    }
    Ok(ids)
}

fn prepare(store: &mut Store) -> Result<()> {
    seed(store)?;
    let left = scenario::add(store, "A", "Работа")?;
    scenario::add(store, "A", "Другой")?;
    let mut a = store.read("A")?;
    rename(&mut a.doc, &left, "Личное")?;
    edit(&mut a.doc, "k0", "title", "Правка заголовка")?;
    edit(&mut a.doc, "k0", "completed", "true")?;
    store.write("A", &a)?;
    scenario::add(store, "A", "Работа")?;
    // B has seen neither A creation. Its birth cannot alias both distinct tags.
    scenario::add(store, "B", " работа ")?;
    let mut b = store.read("B")?;
    edit(&mut b.doc, "k0", "text", "Правка описания")?;
    let column = object(&b.doc, "c1")?.to_string();
    edit(&mut b.doc, "k0", "column", &column)?;
    store.write("B", &b)?;
    exchange(store)
}

fn choose(doc: &mut Automerge, names: [&str; 3]) -> Result<()> {
    let ids = ids(doc)?;
    let observed = view(doc, &ids[0])?;
    let choices: Vec<_> = ids.iter().cloned().zip(names).collect();
    resolve(doc, &ids[0], &observed, &choices)
}

fn verify(doc: &Automerge) -> Result<()> {
    let ids = ids(doc)?;
    assert_eq!(ids.len(), 3);
    scenario::verify_card(doc, &ids[0], &ids[1])?;
    for id in &ids {
        assert_eq!(tag(doc, id)?.ids, std::slice::from_ref(id));
        for card in ["k0", "k1"] {
            let values = links(doc, &object(doc, card)?, &tag(doc, id)?)?;
            assert!(!values.is_empty());
            assert!(values.iter().all(|v| v.value == "present"));
        }
    }
    Ok(())
}

pub(in crate::automerge_probe) fn run(directory: &Path, phase: &str) -> Result<()> {
    let mut store = Store::open(&directory.join("automerge-bridge.sqlite3"))?;
    let checkpoint = directory.join("bridge-expected.json");
    if phase == "bridge-prepare" {
        let count: i64 = store
            .0
            .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
        if count != 0 || checkpoint.exists() {
            return Err("Bridge prepare requires an empty store".into());
        }
        prepare(&mut store)?;
        for peer in ["A", "B"] {
            let replica = store.read(peer)?;
            verify(&replica.doc)?;
            for id in ids(&replica.doc)? {
                assert_eq!(tag(&replica.doc, &id)?.creation_conflict.len(), 3);
            }
        }
    } else {
        let expected: Value = serde_json::from_slice(&std::fs::read(&checkpoint)?)?;
        for peer in ["A", "B"] {
            assert_eq!(store.read(peer)?.fingerprint()?, expected[peer]);
        }
        match phase {
            "bridge-resolve" => {
                for (peer, names) in [
                    ("A", ["Все", "Всё", "Планы"]),
                    ("B", ["Дом", "Дела", "Архив"]),
                ] {
                    let mut replica = store.read(peer)?;
                    choose(&mut replica.doc, names)?;
                    verify(&replica.doc)?;
                    assert!(tags(&replica.doc)?.iter().all(|v| !v.conflicting()));
                    store.write(peer, &replica)?;
                }
            }
            "bridge-resume" => {
                exchange(&mut store)?;
                let mut a = store.read("A")?;
                let old = a.doc.get_changes(&[]);
                for (id, name) in ids(&a.doc)?.iter().zip(["Все", "Всё", "Планы"]) {
                    let observed = view(&a.doc, id)?;
                    assert!(tag(&a.doc, id)?.conflicting());
                    assert!(tag(&a.doc, id)?.creation_conflict.is_empty());
                    resolve(&mut a.doc, id, &observed, &[(id.clone(), name)])?;
                    assign(&mut a.doc, "k0", id, false)?;
                    assign(&mut a.doc, "k0", id, true)?;
                    rename(&mut a.doc, id, "Обратное имя")?;
                    rename(&mut a.doc, id, name)?;
                }
                a.doc.apply_changes(old)?;
                verify(&a.doc)?;
                store.write("A", &a)?;
                exchange(&mut store)?;
                assert_eq!(snapshot(&a.doc)?, snapshot(&store.read("B")?.doc)?);
            }
            _ => return Err("Unknown bridge phase".into()),
        }
    }
    std::fs::write(
        checkpoint,
        serde_json::to_vec(&json!({
            "A": store.read("A")?.fingerprint()?, "B": store.read("B")?.fingerprint()?,
        }))?,
    )?;
    println!("Automerge late tag creation: {phase} passed");
    Ok(())
}

#[cfg(test)]
mod tests;
