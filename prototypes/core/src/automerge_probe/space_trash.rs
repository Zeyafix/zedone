//! P02.05: isolated hierarchy, not a product format or permissions API.
use super::{conflicts::*, *};
use automerge::{ObjId, ObjType, transaction::Transaction};
use std::collections::BTreeMap;

const CARD_FIELDS: [(&str, &str); 5] = [
    ("title", "Карточка"),
    ("text", "# Ёж 😀"),
    ("completed", "false"),
    ("place", "c0:100"),
    ("tag", "present"),
];

fn put(tx: &mut Transaction<'_>, id: &ObjId, field: &str, value: &str) -> Result<ObjId> {
    let assignment = tx.put_object(id, field, ObjType::Map)?;
    tx.put(&assignment, "value", value)?;
    Ok(assignment)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Node {
    id: ObjId,
    kind: String,
    parent: String,
    fields: BTreeMap<String, Vec<Variant>>,
    life: Vec<Variant>,
    cycles: Vec<Variant>,
    space_group: Vec<Variant>,
    column_group: Vec<Variant>,
    barriers: BTreeMap<String, Vec<String>>,
    acknowledged: Vec<String>,
}

impl Node {
    fn blocked(&self) -> bool {
        conflicting(&self.life) || self.fields.values().any(|v| conflicting(v))
    }

    fn parents(&self) -> Vec<&str> {
        if self.kind == "card" {
            self.fields["place"]
                .iter()
                .map(|v| v.value.split(':').next().unwrap())
                .collect()
        } else if self.parent.is_empty() {
            vec![]
        } else {
            vec![&self.parent]
        }
    }
}

type View = BTreeMap<String, Node>;

fn view(doc: &Automerge) -> Result<View> {
    let root = doc.get(ROOT, "nodes")?.ok_or("Missing fixture nodes")?.1;
    let mut nodes = BTreeMap::new();
    for key in doc.keys(&root) {
        let ids = doc.get_all(&root, &key)?;
        if ids.len() != 1 {
            return Err("Expected one stable fixture object".into());
        }
        let id = ids[0].1.clone();
        let scalar = |field| -> Result<String> {
            Ok(doc
                .get(&id, field)?
                .ok_or("Missing metadata")?
                .0
                .as_str()
                .ok_or("Invalid metadata")?
                .into())
        };
        let kind = scalar("kind")?;
        let barriers = doc.get(&id, "barriers")?.ok_or("Missing barriers")?.1;
        let mut boundaries = BTreeMap::new();
        for index in 0..doc.length(&barriers) {
            let event = doc.get(&barriers, index)?.ok_or("Missing boundary")?.1;
            let known = doc
                .get(&event, "known")?
                .ok_or("Missing boundary members")?
                .0;
            boundaries.insert(
                event.to_string(),
                serde_json::from_str(known.as_str().ok_or("Invalid members")?)?,
            );
        }
        let ack = doc
            .get(&id, "acknowledged")?
            .ok_or("Missing acknowledgments")?
            .1;
        let names: Vec<_> = match kind.as_str() {
            "space" | "tag" => vec!["name"],
            "column" => vec!["name", "order"],
            "card" => CARD_FIELDS.iter().map(|(k, _)| *k).collect(),
            _ => return Err("Unknown fixture kind".into()),
        };
        nodes.insert(
            key,
            Node {
                parent: scalar("parent")?,
                kind,
                fields: names
                    .into_iter()
                    .map(|f| Ok((f.into(), read_assignments(doc, &id, f)?)))
                    .collect::<Result<_>>()?,
                life: read_assignments(doc, &id, "life")?,
                cycles: read_assignments(doc, &id, "cycles")?,
                space_group: read_assignments(doc, &id, "space_group")?,
                column_group: read_assignments(doc, &id, "column_group")?,
                id,
                barriers: boundaries,
                acknowledged: doc.keys(&ack).collect(),
            },
        );
    }
    Ok(nodes)
}

fn node<'a>(nodes: &'a View, key: &str) -> Result<&'a Node> {
    nodes
        .get(key)
        .ok_or_else(|| "Unknown fixture object".into())
}

fn within(nodes: &View, key: &str, ancestor: &str) -> bool {
    key == ancestor
        || nodes[key]
            .parents()
            .iter()
            .any(|p| within(nodes, p, ancestor))
}

fn scope(nodes: &View, key: &str) -> View {
    nodes
        .iter()
        .filter(|(k, _)| within(nodes, k, key))
        .map(|(k, n)| (k.clone(), n.clone()))
        .collect()
}

fn deletion_scope(mut observed: View, key: &str) -> View {
    // Column deletion concerns its active cards; space deletion has the stricter
    // whole-space scope, including catalog and earlier trash (ADR-0014).
    if observed[key].kind == "column" {
        observed.retain(|k, n| k == key || n.life.iter().any(|v| v.value == "active"));
    }
    observed
}

fn location(nodes: &View, key: &str) -> &'static str {
    let n = &nodes[key];
    if !conflicting(&n.life) && n.life[0].value == "purged" {
        return "purged";
    }
    if n.kind == "card" && conflicting(&n.fields["place"]) {
        return "conflicts";
    }
    if conflicting(&n.life) {
        return "conflicts";
    }
    if unseen_boundary(nodes, key) {
        return "conflicts";
    }
    if n.life[0].value == "trash" {
        return "trash";
    }
    // Earlier independent trash wins over an ancestor's later deletion conflict.
    for parent in n.parents() {
        let state = location(nodes, parent);
        if state == "purged" {
            // An unseen child is a late creation, never an automatic resurrection.
            return "conflicts";
        }
        if state != "board" {
            return state;
        }
    }
    "board"
}

fn unseen_boundary(nodes: &View, key: &str) -> bool {
    fn missing(nodes: &View, key: &str, ancestor: &str) -> bool {
        let n = &nodes[key];
        nodes[ancestor].barriers.iter().any(|(event, known)| {
            !known.contains(&n.id.to_string()) && !n.acknowledged.contains(event)
        }) || nodes[ancestor]
            .parents()
            .iter()
            .any(|p| missing(nodes, key, p))
    }
    nodes[key].parents().iter().any(|p| missing(nodes, key, p))
}

fn acknowledge(tx: &mut Transaction<'_>, id: &ObjId, nodes: &View) -> Result<()> {
    let ack = tx
        .get(id, "acknowledged")?
        .ok_or("Missing acknowledgments")?
        .1;
    for event in nodes.values().flat_map(|n| n.barriers.keys()) {
        tx.put(&ack, event, true)?;
    }
    Ok(())
}

fn joins_group(nodes: &View, key: &str, container: &str) -> bool {
    key == container
        || (nodes[key].life.iter().any(|v| v.value == "active")
            && nodes[key]
                .parents()
                .iter()
                .any(|p| joins_group(nodes, p, container)))
}

fn active_ancestors(nodes: &View, key: &str) -> Result<Vec<ObjId>> {
    let mut ids = Vec::new();
    for parent in node(nodes, key)?.parents() {
        if location(nodes, parent) != "board" {
            return Err(
                "Explicitly restore the space/column and resolve its deletion first".into(),
            );
        }
        ids.push(nodes[parent].id.clone());
        ids.extend(active_ancestors(nodes, parent)?);
    }
    Ok(ids)
}

fn endorse(tx: &mut Transaction<'_>, ids: &[ObjId]) -> Result<()> {
    for id in ids {
        put(tx, id, "life", "active")?;
    }
    Ok(())
}

fn edit(
    doc: &mut Automerge,
    key: &str,
    observed: &Node,
    field: &str,
    value: &str,
    resolve: bool,
) -> Result<()> {
    let nodes = view(doc)?;
    let current = node(&nodes, key)?;
    if current != observed || !current.fields.contains_key(field) {
        return Err("Stale object or unknown field: read again".into());
    }
    if (["title", "name"].contains(&field) && value.trim().is_empty())
        || (field == "completed" && !["true", "false"].contains(&value))
        || (field == "tag" && !["present", "absent"].contains(&value))
        || (field == "order" && !["100", "200"].contains(&value))
        || (field == "place"
            && !current
                .parents()
                .iter()
                .all(|p| value == format!("{p}:100") || value == format!("{p}:200")))
    {
        return Err("Invalid fixture field value".into());
    }
    let ids = if resolve {
        if !conflicting(&current.fields[field]) {
            return Err("No field conflict".into());
        }
        // Choosing a field never implicitly restores or resolves an ancestor.
        if location(&nodes, key) == "board" {
            active_ancestors(&nodes, key)?
        } else {
            vec![]
        }
    } else {
        if current.blocked() || location(&nodes, key) != "board" {
            return Err("Ordinary editing is blocked".into());
        }
        active_ancestors(&nodes, key)?
    };
    let mut tx = doc.transaction();
    put(&mut tx, &current.id, field, value)?;
    if !resolve || location(&nodes, key) == "board" {
        put(&mut tx, &current.id, "life", "active")?;
    }
    endorse(&mut tx, &ids)?;
    tx.commit();
    Ok(())
}

fn add(
    tx: &mut Transaction<'_>,
    root: &ObjId,
    key: &str,
    kind: &str,
    parent: &str,
    state: &str,
) -> Result<ObjId> {
    let id = tx.put_object(root, key, ObjType::Map)?;
    tx.put(&id, "kind", kind)?;
    tx.put(&id, "parent", parent)?;
    tx.put_object(&id, "barriers", ObjType::List)?;
    tx.put_object(&id, "acknowledged", ObjType::Map)?;
    for (field, value) in [
        ("life", state),
        ("cycles", ""),
        ("space_group", ""),
        ("column_group", ""),
    ] {
        put(tx, &id, field, value)?;
    }
    if kind == "card" {
        let place = format!("{parent}:100");
        for (field, value) in CARD_FIELDS {
            put(
                tx,
                &id,
                field,
                if field == "place" { &place } else { value },
            )?;
        }
    } else {
        put(tx, &id, "name", key)?;
        if kind == "column" {
            put(tx, &id, "order", "100")?;
        }
    }
    Ok(id)
}

fn create(doc: &mut Automerge, key: &str, parent: &str) -> Result<()> {
    let nodes = view(doc)?;
    let target = node(&nodes, parent)?;
    let kind = match (key, target.kind.as_str()) {
        ("new-col" | "late-col", "space") => "column",
        ("new-a" | "new-b", "column") => "card",
        _ => return Err("Unknown fixture creation".into()),
    };
    if nodes.contains_key(key) || location(&nodes, parent) != "board" {
        return Err("Creation requires a new ID and active parent".into());
    }
    let mut ids = active_ancestors(&nodes, parent)?;
    ids.push(target.id.clone());
    let root = doc.get(ROOT, "nodes")?.ok_or("Missing nodes")?.1;
    let mut tx = doc.transaction();
    let id = add(&mut tx, &root, key, kind, parent, "active")?;
    acknowledge(&mut tx, &id, &nodes)?;
    endorse(&mut tx, &ids)?;
    tx.commit();
    Ok(())
}

fn set_container(doc: &mut Automerge, key: &str, observed: &View, state: &str) -> Result<()> {
    let nodes = view(doc)?;
    let current = node(&nodes, key)?;
    let resolving = conflicting(&current.life);
    let ordinary_deletion = !resolving && current.life[0].value == "active" && state == "trash";
    let fresh = if ordinary_deletion && observed.contains_key(key) {
        deletion_scope(scope(&nodes, key), key) == deletion_scope(observed.clone(), key)
    } else {
        scope(&nodes, key) == *observed
    };
    if !["space", "column"].contains(&current.kind.as_str())
        || current.life.iter().any(|v| v.value == "purged")
        || unseen_boundary(&nodes, key)
        || !["active", "trash"].contains(&state)
        || !fresh
    {
        return Err("Stale container scope or invalid command: read again".into());
    }
    let ids = active_ancestors(&nodes, key)?;
    if !resolving
        && (current.life[0].value == state
            || current.blocked()
            || (state == "trash"
                && observed
                    .iter()
                    .any(|(k, n)| location(&nodes, k) != "trash" && n.blocked())))
    {
        return Err("Ordinary container action is blocked or already applied".into());
    }
    let mut tx = doc.transaction();
    let group = if state == "trash" && !resolving {
        let cycle = tx.put_object(&current.id, "cycles", ObjType::Map)?;
        let group = cycle.to_string();
        tx.put(cycle, "value", group.as_str())?;
        group
    } else {
        current
            .cycles
            .iter()
            .map(|v| v.value.as_str())
            .filter(|v| !v.is_empty())
            .min()
            .unwrap_or("")
            .to_owned()
    };
    if state == "trash" {
        if group.is_empty() {
            return Err("Deletion requires its causal group".into());
        }
        let field = if current.kind == "space" {
            "space_group"
        } else {
            "column_group"
        };
        for (k, n) in observed {
            // Keep independently deleted columns/cards and all their nested groups intact.
            if k != key && n.kind != "tag" && joins_group(&nodes, k, key) {
                put(&mut tx, &n.id, field, &group)?;
            }
        }
    }
    put(&mut tx, &current.id, "life", state)?;
    endorse(&mut tx, &ids)?;
    tx.commit();
    Ok(())
}

fn set_card(
    doc: &mut Automerge,
    key: &str,
    observed: &Node,
    state: &str,
    target: &str,
) -> Result<()> {
    let nodes = view(doc)?;
    let current = node(&nodes, key)?;
    if current != observed
        || current.kind != "card"
        || current.blocked()
        || !["active", "trash"].contains(&state)
    {
        return Err("Stale, conflicting card or invalid state".into());
    }
    let parent = current.parents()[0];
    let space = &nodes[parent].parent;
    if location(&nodes, space) != "board" {
        return Err("Explicitly restore the entire space before individual recovery".into());
    }
    let old = location(&nodes, key);
    if !matches!((old, state), ("board", "trash") | ("trash", "active")) {
        return Err("Expected an explicit transition".into());
    }
    let ids = if state == "active" {
        let destination = node(&nodes, target)?;
        if destination.kind != "column"
            || &destination.parent != space
            || location(&nodes, target) != "board"
        {
            return Err("Choose an active column in the same restored space".into());
        }
        let mut ids = active_ancestors(&nodes, target)?;
        ids.push(destination.id.clone());
        ids
    } else {
        active_ancestors(&nodes, key)?
    };
    let mut tx = doc.transaction();
    put(&mut tx, &current.id, "life", state)?;
    for field in ["space_group", "column_group"] {
        put(&mut tx, &current.id, field, "")?;
    }
    if state == "active" {
        put(&mut tx, &current.id, "place", &format!("{target}:100"))?;
    }
    endorse(&mut tx, &ids)?;
    tx.commit();
    Ok(())
}

pub(super) fn snapshot(doc: &Automerge) -> Result<Value> {
    let nodes = view(doc)?;
    let locations: BTreeMap<_, _> = nodes.iter().map(|(k, n)| (k, json!({
        "location": location(&nodes, k), "blocked": n.blocked() || location(&nodes, k) != "board",
    }))).collect();
    Ok(json!({"nodes": nodes, "locations": locations}))
}

fn seed(store: &mut Store) -> Result<()> {
    let a = fixture(true)?;
    store.write("A", &a)?;
    store.write("B", &Replica::new())?;
    exchange(store)
}

fn fixture(include_foreign: bool) -> Result<Replica> {
    let mut a = Replica::new();
    let mut tx = a.doc.transaction();
    tx.put(ROOT, "space_trash_fixture", true)?;
    if !include_foreign {
        tx.put(ROOT, "single_space_fixture", true)?;
    }
    let root = tx.put_object(ROOT, "nodes", ObjType::Map)?;
    tx.put_object(ROOT, "history", ObjType::List)?;
    for (key, kind, parent) in [
        ("s0", "space", ""),
        ("s1", "space", ""),
        ("c0", "column", "s0"),
        ("c1", "column", "s0"),
        ("old-col", "column", "s0"),
        ("foreign-col", "column", "s1"),
        ("tag", "tag", "s0"),
        ("k0", "card", "c0"),
        ("k1", "card", "c0"),
        ("old", "card", "c0"),
        ("old-col-card", "card", "old-col"),
        ("other", "card", "c1"),
        ("foreign", "card", "foreign-col"),
    ] {
        if !include_foreign && ["s1", "foreign-col", "foreign"].contains(&key) {
            continue;
        }
        add(&mut tx, &root, key, kind, parent, "active")?;
    }
    tx.commit();
    // Real earlier actions supply a genuine independent nested column group.
    let nodes = view(&a.doc)?;
    set_card(&mut a.doc, "old", &nodes["old"], "trash", "")?;
    let nodes = view(&a.doc)?;
    set_container(&mut a.doc, "old-col", &scope(&nodes, "old-col"), "trash")?;
    Ok(a)
}

mod scenario;
pub(super) use scenario::run;
pub(super) mod history;
pub(super) mod lifecycle;
pub(super) mod purge;
#[cfg(test)]
mod tests;
