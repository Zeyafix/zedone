//! P02.05.4: causal deletion of known visible versions, not autosave grouping.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct Version {
    pub(super) id: ObjId,
    pub(super) card: String,
    pub(super) title: String,
    pub(super) text: String,
    pub(super) title_op: String,
    pub(super) text_op: String,
}

pub(super) fn versions(doc: &Automerge) -> Result<Vec<Version>> {
    let Some((_, root)) = doc.get(ROOT, "history")? else {
        return Ok(vec![]);
    };
    let mut entries = Vec::new();
    for index in 0..doc.length(&root) {
        let id = doc.get(&root, index)?.ok_or("Missing history entry")?.1;
        let scalar = |field| -> Result<String> {
            Ok(doc
                .get(&id, field)?
                .ok_or("Missing version field")?
                .0
                .as_str()
                .ok_or("Expected version string")?
                .into())
        };
        entries.push(Version {
            card: scalar("card")?,
            title: scalar("title")?,
            text: scalar("text")?,
            title_op: scalar("title_op")?,
            text_op: scalar("text_op")?,
            id,
        });
    }
    Ok(entries)
}

// One discrete synthetic command = one saved pair and one visible version.
// Participant sessions, autosave series and product permission checks are P03/P04.
fn record(doc: &mut Automerge, key: &str, observed: &Node, title: &str, text: &str) -> Result<()> {
    let nodes = view(doc)?;
    let n = node(&nodes, key)?;
    if n != observed
        || n.kind != "card"
        || n.blocked()
        || location(&nodes, key) != "board"
        || title.trim().is_empty()
    {
        return Err("Stale, blocked or invalid history edit".into());
    }
    let ancestors = active_ancestors(&nodes, key)?;
    let history = doc
        .get(ROOT, "history")?
        .ok_or("Missing shared history list")?
        .1;
    let index = doc.length(&history);
    let mut tx = doc.transaction();
    let title_op = put(&mut tx, &n.id, "title", title)?;
    let text_op = put(&mut tx, &n.id, "text", text)?;
    put(&mut tx, &n.id, "life", "active")?;
    endorse(&mut tx, &ancestors)?;
    let entry = tx.insert_object(&history, index, ObjType::Map)?;
    for (field, value) in [
        ("card", key),
        ("title", title),
        ("text", text),
        ("title_op", &title_op.to_string()),
        ("text_op", &text_op.to_string()),
    ] {
        tx.put(&entry, field, value)?;
    }
    tx.commit();
    Ok(())
}

fn protected(nodes: &View, version: &Version) -> bool {
    let n = &nodes[&version.card];
    [("title", &version.title_op), ("text", &version.text_op)]
        .iter()
        .any(|(field, op)| {
            conflicting(&n.fields[*field]) && n.fields[*field].iter().any(|v| &v.operation == *op)
        })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct ClearPlan {
    space: String,
    card: Option<String>,
    observed: View,
    versions: Vec<Version>,
    removed: Vec<ObjId>,
    retained_conflicts: Vec<ObjId>,
}

pub(super) fn preview(doc: &Automerge, space: &str, card: Option<&str>) -> Result<ClearPlan> {
    let nodes = view(doc)?;
    if node(&nodes, space)?.kind != "space" {
        return Err("Expected a space".into());
    }
    if let Some(key) = card
        && (node(&nodes, key)?.kind != "card" || !within(&nodes, key, space))
    {
        return Err("Expected a card in the selected space".into());
    }
    let observed = match card {
        Some(key) => scope(&nodes, key),
        None => scope(&nodes, space),
    };
    let versions: Vec<_> = versions(doc)?
        .into_iter()
        .filter(|v| observed.contains_key(&v.card))
        .collect();
    let removed = versions
        .iter()
        .filter(|v| !protected(&nodes, v))
        .map(|v| v.id.clone())
        .collect();
    let retained_conflicts = versions
        .iter()
        .filter(|v| protected(&nodes, v))
        .map(|v| v.id.clone())
        .collect();
    Ok(ClearPlan {
        space: space.into(),
        card: card.map(str::to_owned),
        observed,
        versions,
        removed,
        retained_conflicts,
    })
}

pub(super) fn clear(doc: &mut Automerge, plan: &ClearPlan) -> Result<()> {
    if preview(doc, &plan.space, plan.card.as_deref())? != *plan {
        return Err("Stale history scope: inspect again".into());
    }
    if plan.removed.is_empty() {
        return Err("No known removable history versions".into());
    }
    let entries = versions(doc)?;
    let root = doc.get(ROOT, "history")?.ok_or("Missing history")?.1;
    let mut tx = doc.transaction();
    // List deletion addresses the observed element's operation. Concurrent inserts
    // and versions absent from this replica cannot be accidentally deleted.
    for (index, v) in entries.iter().enumerate().rev() {
        if plan.removed.contains(&v.id) {
            tx.delete(&root, index)?;
        }
    }
    tx.commit();
    Ok(())
}

// Purging contents also removes their known user-visible history in the SAME
// Automerge change. Concurrent versions are not in these observed list slots.
pub(super) fn purge_slots(doc: &Automerge, keys: &[String]) -> Result<Option<(ObjId, Vec<usize>)>> {
    let Some((_, root)) = doc.get(ROOT, "history")? else {
        return Ok(None);
    };
    let indices = versions(doc)?
        .iter()
        .enumerate()
        .filter(|(_, v)| keys.contains(&v.card))
        .map(|(i, _)| i)
        .collect();
    Ok(Some((root, indices)))
}

pub(super) fn purge_known(
    tx: &mut Transaction<'_>,
    slots: Option<(ObjId, Vec<usize>)>,
) -> Result<()> {
    if let Some((root, indices)) = slots {
        for index in indices.into_iter().rev() {
            tx.delete(&root, index)?;
        }
    }
    Ok(())
}

pub(in super::super) fn snapshot(doc: &Automerge) -> Result<Value> {
    // List order is deterministic but is not a wall-clock ordering of offline edits.
    Ok(serde_json::to_value(versions(doc)?)?)
}

pub(super) fn save_pair(doc: &mut Automerge, key: &str, title: &str, text: &str) -> Result<()> {
    record(doc, key, &view(doc)?[key], title, text)
}

fn seed_history(store: &mut Store) -> Result<()> {
    seed(store)?;
    let mut a = store.read("A")?;
    for (key, title, text) in [
        ("k0", "Версия 1", "Один"),
        ("k0", "Версия 2", "Два"),
        ("k1", "Сосед", "Три"),
        ("foreign", "Другое", "Четыре"),
    ] {
        save_pair(&mut a.doc, key, title, text)?;
    }
    store.write("A", &a)?;
    exchange(store)
}

pub(in super::super) fn run(directory: &Path, phase: &str) -> Result<()> {
    for whole_space in [false, true] {
        let mut store =
            Store::open(&directory.join(format!("automerge-history-{whole_space}.sqlite3")))?;
        let checkpoint = directory.join(format!("history-{whole_space}-expected.json"));
        match phase {
            "history-prepare" => {
                if store
                    .0
                    .query_row("SELECT count(*) FROM replicas", [], |r| r.get::<_, i64>(0))?
                    != 0
                    || checkpoint.exists()
                {
                    return Err("History prepare requires an empty disposable store".into());
                }
                seed_history(&mut store)?;
                let mut a = store.read("A")?;
                let mut b = store.read("B")?;
                let before = view(&a.doc)?;
                let plan = preview(&a.doc, "s0", if whole_space { None } else { Some("k0") })?;
                assert_eq!(plan.removed.len(), if whole_space { 3 } else { 2 });
                clear(&mut a.doc, &plan)?;
                assert_eq!(view(&a.doc)?, before);
                save_pair(&mut b.doc, "k0", "Офлайн", "Новая независимая версия 😀")?;
                store.write("A", &a)?;
                store.write("B", &b)?;
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "history-clear" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    let r = store.read(id)?;
                    let entries = versions(&r.doc)?;
                    assert_eq!(entries.len(), if whole_space { 2 } else { 3 });
                    assert_eq!(entries.iter().filter(|v| v.card == "k0").count(), 1);
                    assert!(
                        entries
                            .iter()
                            .any(|v| v.text == "Новая независимая версия 😀")
                    );
                    assert_eq!(view(&r.doc)?["k0"].fields["title"][0].value, "Офлайн");
                }
                conflicts::checkpoint(&store, &checkpoint, false)?;
            }
            "history-resume" => {
                conflicts::checkpoint(&store, &checkpoint, true)?;
                let mut a = store.read("A")?;
                save_pair(&mut a.doc, "k0", "После очистки", "История продолжается")?;
                store.write("A", &a)?;
                exchange(&mut store)?;
                assert_eq!(
                    snapshot(&store.read("A")?.doc)?,
                    snapshot(&store.read("B")?.doc)?
                );
                assert_eq!(
                    versions(&store.read("B")?.doc)?.len(),
                    if whole_space { 3 } else { 4 }
                );
            }
            _ => return Err("Unknown history phase".into()),
        }
    }
    println!("Automerge history: {phase} passed");
    Ok(())
}

#[cfg(test)]
mod tests;
