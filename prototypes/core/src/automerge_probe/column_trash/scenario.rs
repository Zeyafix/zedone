use super::*;

pub(super) const EDITED: [&str; 5] = [
    "Новая карточка",
    "Правка без связи 😀",
    "true",
    "c0:200",
    "absent",
];

pub(super) fn prepare(store: &mut Store) -> Result<()> {
    seed(store)?;
    let mut a = store.read("A")?;
    let mut b = store.read("B")?;
    let observed = column(&a.doc, "c0")?;
    set_column(&mut a.doc, "c0", &observed, "trash")?;
    for (field, value) in FIELDS.into_iter().zip(EDITED) {
        let observed = card(&b.doc, "k0")?;
        edit(&mut b.doc, "k0", &observed, field, value, false)?;
    }
    create(&mut b.doc, "new-a", "c0")?;
    store.write("A", &a)?;
    store.write("B", &b)?;
    exchange(store)
}

fn expect(doc: &Automerge, state: &str) -> Result<()> {
    let current = column(doc, "c0")?;
    assert_eq!(conflicting(&current.life), state == "conflicts");
    for key in ["k0", "k1", "new-a"] {
        assert_eq!(current.location(&current.cards[key]), state);
        if state == "trash" {
            assert!(current.member(&current.cards[key]));
        }
    }
    assert_eq!(current.location(&current.cards["old"]), "trash");
    assert!(!current.member(&current.cards["old"]));
    for (field, value) in FIELDS.into_iter().zip(EDITED) {
        assert_eq!(current.cards["k0"].fields[field][0].value, value);
    }
    let other = column(doc, "c1")?;
    assert_eq!(other.location(&other.cards["other"]), "board");
    Ok(())
}

pub(in super::super) fn run(directory: &Path, phase: &str) -> Result<()> {
    for state in ["active", "trash"] {
        let mut store =
            Store::open(&directory.join(format!("automerge-column-trash-{state}.sqlite3")))?;
        let checkpoint = directory.join(format!("column-trash-{state}-expected.json"));
        match phase {
            "column-trash-prepare" => {
                let count: i64 = store
                    .0
                    .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
                if count != 0 || checkpoint.exists() {
                    return Err("Column trash prepare requires an empty disposable store".into());
                }
                prepare(&mut store)?;
                for id in ["A", "B"] {
                    expect(&store.read(id)?.doc, "conflicts")?;
                }
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "column-trash-resolve" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                for id in ["A", "B"] {
                    let mut replica = store.read(id)?;
                    let observed = column(&replica.doc, "c0")?;
                    set_column(&mut replica.doc, "c0", &observed, state)?;
                    store.write(id, &replica)?;
                }
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "column-trash-resume" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                exchange(&mut store)?;
                let location = if state == "active" { "board" } else { "trash" };
                for id in ["A", "B"] {
                    expect(&store.read(id)?.doc, location)?;
                }
                let mut a = store.read("A")?;
                let before = column(&a.doc, "c0")?;
                for next in [if state == "active" { "trash" } else { "active" }, state] {
                    let observed = column(&a.doc, "c0")?;
                    set_column(&mut a.doc, "c0", &observed, next)?;
                }
                let after = column(&a.doc, "c0")?;
                assert_eq!(before.id, after.id);
                for key in before.cards.keys() {
                    assert_eq!(before.cards[key].id, after.cards[key].id);
                    assert_eq!(before.cards[key].fields, after.cards[key].fields);
                }
                store.write("A", &a)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    expect(&store.read(id)?.doc, location)?;
                }
                // Restore an earlier deletion alone into the other active column.
                let mut a = store.read("A")?;
                let old = card(&a.doc, "old")?;
                let source_life = column(&a.doc, "c0")?.life;
                for next in ["active", "trash", "active"] {
                    let observed = card(&a.doc, "old")?;
                    set_card(&mut a.doc, "old", &observed, next, "c1")?;
                }
                assert_eq!(card(&a.doc, "old")?.id, old.id);
                assert_eq!(column(&a.doc, "c0")?.life, source_life);
                store.write("A", &a)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    let replica = store.read(id)?;
                    let destination = column(&replica.doc, "c1")?;
                    assert_eq!(destination.location(&destination.cards["old"]), "board");
                    assert_eq!(destination.cards["old"].id, old.id);
                }
            }
            _ => return Err("Unknown column trash phase".into()),
        }
    }
    println!("Automerge column trash: {phase} passed");
    Ok(())
}
