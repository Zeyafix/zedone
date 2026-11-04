use super::*;

pub(super) const EDITED_FIELDS: [&str; 5] = [
    "Правка без связи",
    "# Независимый Markdown 😀",
    "true",
    "c1:100",
    "absent",
];

pub(super) fn prepare(store: &mut Store) -> Result<()> {
    seed(store)?;
    let mut a = store.read("A")?;
    let mut b = store.read("B")?;
    set_life(&mut a.doc, "k0", &view(&b.doc, "k0")?, "trash")?;
    for (field, value) in FIELDS.iter().zip(EDITED_FIELDS) {
        let observed = view(&b.doc, "k0")?;
        edit(&mut b.doc, "k0", &observed, field, value, false)?;
    }
    store.write("A", &a)?;
    store.write("B", &b)?;
    exchange(store)
}

pub(super) fn expect(doc: &Automerge, location: &str) -> Result<()> {
    let card = view(doc, "k0")?;
    assert_eq!(card.location(), location);
    for (field, value) in FIELDS.iter().zip(EDITED_FIELDS) {
        assert!(!conflicting(&card.fields[*field]));
        assert_eq!(card.fields[*field][0].value, value);
    }
    assert_eq!(view(doc, "k1")?.location(), "board");
    Ok(())
}

pub(in super::super) fn run(directory: &Path, phase: &str) -> Result<()> {
    for state in ["active", "trash"] {
        let mut store = Store::open(&directory.join(format!("automerge-trash-{state}.sqlite3")))?;
        let checkpoint = directory.join(format!("trash-{state}-expected.json"));
        match phase {
            "trash-prepare" => {
                let count: i64 = store
                    .0
                    .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
                if count != 0 || checkpoint.exists() {
                    return Err("Trash prepare requires an empty disposable store".into());
                }
                prepare(&mut store)?;
                for id in ["A", "B"] {
                    expect(&store.read(id)?.doc, "conflicts")?;
                }
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "trash-resolve" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                // Equal choices must converge without a false conflict after restart.
                for id in ["A", "B"] {
                    let mut replica = store.read(id)?;
                    let observed = view(&replica.doc, "k0")?;
                    set_life(&mut replica.doc, "k0", &observed, state)?;
                    store.write(id, &replica)?;
                }
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "trash-resume" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                exchange(&mut store)?;
                let location = if state == "active" { "board" } else { "trash" };
                for id in ["A", "B"] {
                    expect(&store.read(id)?.doc, location)?;
                }
                let mut a = store.read("A")?;
                let before = view(&a.doc, "k0")?;
                // Normal trash/restore are inverse operations preserving identity/data.
                for next in [if state == "active" { "trash" } else { "active" }, state] {
                    let observed = view(&a.doc, "k0")?;
                    set_life(&mut a.doc, "k0", &observed, next)?;
                }
                assert_eq!(view(&a.doc, "k0")?.id, before.id);
                assert_eq!(view(&a.doc, "k0")?.fields, before.fields);
                store.write("A", &a)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    expect(&store.read(id)?.doc, location)?;
                }
            }
            _ => return Err("Expected trash-prepare, trash-resolve or trash-resume".into()),
        }
    }
    println!("Automerge card trash: {phase} passed");
    Ok(())
}
