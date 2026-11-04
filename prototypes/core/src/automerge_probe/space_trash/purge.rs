//! Logical purge boundaries. Automerge's physical change graph is measured in P02.06.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct Plan {
    space: String,
    roots: Vec<String>,
    all: bool,
    observed: View,
    targets: Vec<String>,
    conflicts: Vec<String>,
}

fn preview(doc: &Automerge, space: &str, selected: &[&str], all: bool) -> Result<Plan> {
    let nodes = view(doc)?;
    if node(&nodes, space)?.kind != "space" || (all && !selected.is_empty()) {
        return Err("Expected one space and selected objects or its whole trash".into());
    }
    let mut roots: Vec<String> = if all {
        nodes
            .keys()
            .filter(|k| {
                within(&nodes, k, space)
                    && (location(&nodes, k) == "trash"
                        || (location(&nodes, k) == "conflicts"
                            && nodes[*k].life.iter().any(|v| v.value == "trash")))
            })
            .cloned()
            .collect()
    } else {
        selected.iter().map(|s| (*s).into()).collect()
    };
    roots.sort();
    roots.dedup();
    for key in &roots {
        node(&nodes, key)?;
        if !within(&nodes, key, space) || !["trash", "conflicts"].contains(&location(&nodes, key)) {
            return Err("Selection must belong to this space's trash".into());
        }
    }
    let included = |key: &str| roots.iter().any(|r| within(&nodes, key, r));
    let targets = nodes
        .keys()
        .filter(|k| included(k) && location(&nodes, k) == "trash")
        .cloned()
        .collect();
    let conflicts = nodes
        .iter()
        .filter(|(k, n)| included(k) && (n.blocked() || location(&nodes, k) == "conflicts"))
        .map(|(k, _)| k.clone())
        .collect();
    Ok(Plan {
        space: space.into(),
        roots,
        all,
        observed: scope(&nodes, space),
        targets,
        conflicts,
    })
}

fn apply(doc: &mut Automerge, plan: &Plan) -> Result<()> {
    let keys: Vec<_> = plan.roots.iter().map(String::as_str).collect();
    let current = preview(
        doc,
        &plan.space,
        if plan.all { &[] } else { &keys },
        plan.all,
    )?;
    if current != *plan {
        return Err("Stale purge scope: inspect and explicitly request again".into());
    }
    if !plan.conflicts.is_empty() {
        return Err(format!("Purge blocked by conflicts: {}", plan.conflicts.join(", ")).into());
    }
    if plan.targets.is_empty() {
        return Err("No trash selected".into());
    }
    let slots = history::purge_slots(doc, &plan.targets)?;
    let barriers = boundary_slots(doc, plan)?;
    let mut tx = doc.transaction();
    append_boundaries(&mut tx, plan, barriers)?;
    for key in &plan.targets {
        put(&mut tx, &plan.observed[key].id, "life", "purged")?;
    }
    history::purge_known(&mut tx, slots)?;
    tx.commit();
    Ok(())
}

fn boundary_slots(doc: &Automerge, plan: &Plan) -> Result<Vec<(ObjId, usize)>> {
    plan.targets
        .iter()
        .filter(|k| ["space", "column"].contains(&plan.observed[*k].kind.as_str()))
        .map(|key| {
            let list = doc
                .get(&plan.observed[key].id, "barriers")?
                .ok_or("Missing barriers")?
                .1;
            let index = doc.length(&list);
            Ok((list, index))
        })
        .collect()
}

fn append_boundaries(
    tx: &mut Transaction<'_>,
    plan: &Plan,
    slots: Vec<(ObjId, usize)>,
) -> Result<()> {
    let known = serde_json::to_string(
        &plan
            .observed
            .values()
            .map(|n| n.id.to_string())
            .collect::<Vec<_>>(),
    )?;
    for (list, index) in slots {
        let event = tx.insert_object(&list, index, ObjType::Map)?;
        tx.put(event, "known", known.as_str())?;
    }
    Ok(())
}

fn affected(nodes: &View, key: &str) -> bool {
    unseen_boundary(nodes, key)
        || nodes[key].life.iter().any(|v| v.value == "purged")
        || nodes[key].parents().iter().any(|p| affected(nodes, p))
}

pub(super) fn late_preview(doc: &Automerge, space: &str) -> Result<Plan> {
    let nodes = view(doc)?;
    if node(&nodes, space)?.kind != "space" {
        return Err("Expected a space".into());
    }
    let observed = scope(&nodes, space);
    let targets = observed
        .keys()
        .filter(|k| location(&nodes, k) == "conflicts" && affected(&nodes, k))
        .cloned()
        .collect();
    Ok(Plan {
        space: space.into(),
        roots: vec![],
        all: false,
        observed,
        targets,
        conflicts: vec![],
    })
}

pub(super) fn resolve(doc: &mut Automerge, plan: &Plan, restore: bool) -> Result<()> {
    if late_preview(doc, &plan.space)? != *plan || plan.targets.is_empty() {
        return Err("Stale or empty late-change selection".into());
    }
    // Every target is either an independently changed/created object or one of
    // its required original ancestors. Untouched purged siblings stay purged.
    let slots = if restore {
        None
    } else {
        history::purge_slots(doc, &plan.targets)?
    };
    let boundaries = if restore {
        vec![]
    } else {
        boundary_slots(doc, plan)?
    };
    let mut tx = doc.transaction();
    append_boundaries(&mut tx, plan, boundaries)?;
    for key in &plan.targets {
        let n = &plan.observed[key];
        put(
            &mut tx,
            &n.id,
            "life",
            if restore { "active" } else { "purged" },
        )?;
        if restore {
            acknowledge(&mut tx, &n.id, &plan.observed)?;
            for field in ["space_group", "column_group"] {
                put(&mut tx, &n.id, field, "")?;
            }
        }
    }
    history::purge_known(&mut tx, slots)?;
    tx.commit();
    Ok(())
}

pub(super) fn purge(doc: &mut Automerge, space: &str, selected: &[&str], all: bool) -> Result<()> {
    apply(doc, &preview(doc, space, selected, all)?)
}

pub(in super::super) fn snapshot(doc: &Automerge) -> Result<Value> {
    let nodes = view(doc)?;
    // This is the logical read model. Raw Automerge/SQLite bytes are not erased
    // by tombstones and must not be confused with secure physical deletion.
    let visible: View = nodes
        .iter()
        .filter(|(k, _)| location(&nodes, k) != "purged")
        .map(|(k, n)| (k.clone(), n.clone()))
        .collect();
    let locations: BTreeMap<_, _> = visible.keys().map(|k| (k, location(&nodes, k))).collect();
    Ok(json!({"visible": visible, "locations": locations}))
}

pub(in super::super) fn run(directory: &Path, phase: &str) -> Result<()> {
    for restore in [false, true] {
        let mut store = Store::open(&directory.join(format!("automerge-purge-{restore}.sqlite3")))?;
        let checkpoint = directory.join(format!("purge-{restore}-expected.json"));
        match phase {
            "purge-prepare" => {
                if store
                    .0
                    .query_row("SELECT count(*) FROM replicas", [], |r| r.get::<_, i64>(0))?
                    != 0
                    || checkpoint.exists()
                {
                    return Err("Purge prepare requires an empty disposable store".into());
                }
                seed(&mut store)?;
                let mut a = store.read("A")?;
                scenario::choose(&mut a.doc, "s0", "trash")?;
                purge(&mut a.doc, "s0", &["s0"], false)?;
                store.write("A", &a)?;
                let mut b = store.read("B")?;
                scenario::change(&mut b.doc, "k0", "text", "Поздняя правка 😀")?;
                create(&mut b.doc, "new-col", "s0")?;
                create(&mut b.doc, "new-a", "new-col")?;
                store.write("B", &b)?;
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "purge-resolve" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    let mut replica = store.read(id)?;
                    let plan = late_preview(&replica.doc, "s0")?;
                    assert_eq!(plan.targets, ["c0", "k0", "new-a", "new-col", "s0"]);
                    resolve(&mut replica.doc, &plan, restore)?;
                    store.write(id, &replica)?;
                }
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "purge-resume" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                exchange(&mut store)?;
                let a = store.read("A")?;
                let b = store.read("B")?;
                assert_eq!(snapshot(&a.doc)?, snapshot(&b.doc)?);
                let nodes = view(&a.doc)?;
                for key in ["s0", "c0", "k0", "new-col", "new-a"] {
                    assert_eq!(
                        location(&nodes, key),
                        if restore { "board" } else { "purged" }
                    );
                }
                for key in ["k1", "old", "old-col", "old-col-card", "c1", "other", "tag"] {
                    assert_eq!(location(&nodes, key), "purged");
                }
                assert_eq!(location(&nodes, "foreign"), "board");
                assert_eq!(nodes["k0"].fields["text"][0].value, "Поздняя правка 😀");
            }
            _ => return Err("Unknown purge phase".into()),
        }
    }
    println!("Automerge purge: {phase} passed");
    Ok(())
}

#[cfg(test)]
mod tests;
