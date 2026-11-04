use super::*;

pub(super) const EDITED: [&str; 5] = ["Новое", "Правка без связи 😀", "true", "c0:200", "absent"];

pub(super) fn choose(doc: &mut Automerge, key: &str, state: &str) -> Result<()> {
    set_container(doc, key, &scope(&view(doc)?, key), state)
}

pub(super) fn change(doc: &mut Automerge, key: &str, field: &str, value: &str) -> Result<()> {
    edit(doc, key, &view(doc)?[key], field, value, false)
}

pub(super) fn prepare(store: &mut Store) -> Result<()> {
    seed(store)?;
    let mut a = store.read("A")?;
    let mut b = store.read("B")?;
    choose(&mut a.doc, "s0", "trash")?;
    for ((field, _), value) in CARD_FIELDS.into_iter().zip(EDITED) {
        change(&mut b.doc, "k0", field, value)?;
    }
    change(&mut b.doc, "c0", "name", "Новая колонка")?;
    create(&mut b.doc, "new-col", "s0")?;
    create(&mut b.doc, "new-a", "new-col")?;
    create(&mut b.doc, "new-b", "c0")?;
    store.write("A", &a)?;
    store.write("B", &b)?;
    exchange(store)
}

pub(super) fn expect(doc: &Automerge, state: &str) -> Result<()> {
    let nodes = view(doc)?;
    assert_eq!(conflicting(&nodes["s0"].life), state == "conflicts");
    for key in [
        "s0", "c0", "c1", "k0", "k1", "other", "new-col", "new-a", "new-b",
    ] {
        assert_eq!(location(&nodes, key), state, "{key}");
        if state == "trash" && key != "s0" {
            assert!(nodes[key].space_group.iter().any(|g| {
                nodes["s0"]
                    .cycles
                    .iter()
                    .any(|c| !g.value.is_empty() && c.value == g.value)
            }));
        }
    }
    for key in ["old", "old-col", "old-col-card"] {
        assert_eq!(location(&nodes, key), "trash");
        assert_eq!(nodes[key].space_group[0].value, "");
    }
    assert!(!nodes["old-col-card"].column_group[0].value.is_empty());
    for key in ["s1", "foreign-col", "foreign"] {
        assert_eq!(location(&nodes, key), "board");
    }
    for ((field, _), value) in CARD_FIELDS.into_iter().zip(EDITED) {
        assert_eq!(nodes["k0"].fields[field][0].value, value);
    }
    Ok(())
}

pub(in super::super) fn run(directory: &Path, phase: &str) -> Result<()> {
    for state in ["active", "trash"] {
        let mut store =
            Store::open(&directory.join(format!("automerge-space-trash-{state}.sqlite3")))?;
        let checkpoint = directory.join(format!("space-trash-{state}-expected.json"));
        match phase {
            "space-trash-prepare" => {
                let count: i64 = store
                    .0
                    .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
                if count != 0 || checkpoint.exists() {
                    return Err("Space trash prepare requires an empty disposable store".into());
                }
                prepare(&mut store)?;
                for id in ["A", "B"] {
                    expect(&store.read(id)?.doc, "conflicts")?;
                }
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "space-trash-resolve" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                for id in ["A", "B"] {
                    let mut replica = store.read(id)?;
                    choose(&mut replica.doc, "s0", state)?;
                    store.write(id, &replica)?;
                }
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "space-trash-resume" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    expect(
                        &store.read(id)?.doc,
                        if state == "active" { "board" } else { "trash" },
                    )?;
                }
                let mut a = store.read("A")?;
                let before = view(&a.doc)?;
                for next in [if state == "active" { "trash" } else { "active" }, state] {
                    choose(&mut a.doc, "s0", next)?;
                }
                let after = view(&a.doc)?;
                for key in before.keys() {
                    assert_eq!(before[key].id, after[key].id);
                    assert_eq!(before[key].fields, after[key].fields);
                }
                store.write("A", &a)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    expect(
                        &store.read(id)?.doc,
                        if state == "active" { "board" } else { "trash" },
                    )?;
                }
                // No partial space restoration: restore the space, then earlier trash.
                let mut a = store.read("A")?;
                if state == "trash" {
                    let old = view(&a.doc)?["old"].clone();
                    assert!(set_card(&mut a.doc, "old", &old, "active", "c1").is_err());
                    choose(&mut a.doc, "s0", "active")?;
                }
                let old = view(&a.doc)?["old"].clone();
                set_card(&mut a.doc, "old", &old, "active", "c1")?;
                choose(&mut a.doc, "old-col", "active")?;
                assert_eq!(view(&a.doc)?["old"].id, old.id);
                store.write("A", &a)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    let nodes = view(&store.read(id)?.doc)?;
                    for key in nodes.keys() {
                        assert_eq!(location(&nodes, key), "board");
                    }
                }
            }
            _ => return Err("Unknown space trash phase".into()),
        }
    }
    println!("Automerge space trash: {phase} passed");
    Ok(())
}
