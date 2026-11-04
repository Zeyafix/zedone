//! P02.05.2.1: one active space, column deletion groups; no product API or purge.
use super::{conflicts::*, *};
use automerge::{ObjId, ObjType, Value as AmValue, transaction::Transaction};
use std::collections::BTreeMap;

const FIELDS: [&str; 5] = ["title", "text", "completed", "place", "tag"];
const INITIAL: [&str; 5] = ["Карточка", "# Ёж 😀", "false", "c0:100", "present"];

fn object(doc: &Automerge, collection: &str, key: &str) -> Result<ObjId> {
    let root = doc
        .get(ROOT, collection)?
        .ok_or("Missing fixture collection")?
        .1;
    match doc.get_all(root, key)?.as_slice() {
        [(AmValue::Object(ObjType::Map), id)] => Ok(id.clone()),
        _ => Err("Expected one stable fixture object".into()),
    }
}

fn put(tx: &mut Transaction<'_>, id: &ObjId, field: &str, value: &str) -> Result<ObjId> {
    let assignment = tx.put_object(id, field, ObjType::Map)?;
    tx.put(&assignment, "value", value)?;
    Ok(assignment)
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Card {
    id: ObjId,
    fields: BTreeMap<String, Vec<Variant>>,
    life: Vec<Variant>,
    group: Vec<Variant>,
}

impl Card {
    fn blocked(&self) -> bool {
        conflicting(&self.life) || self.fields.values().any(|v| conflicting(v))
    }

    fn in_column(&self, key: &str) -> bool {
        self.fields["place"]
            .iter()
            .any(|v| v.value.split(':').next() == Some(key))
    }
}

fn card(doc: &Automerge, key: &str) -> Result<Card> {
    let id = object(doc, "cards", key)?;
    Ok(Card {
        fields: FIELDS
            .iter()
            .map(|f| Ok(((*f).into(), read_assignments(doc, &id, f)?)))
            .collect::<Result<_>>()?,
        life: read_assignments(doc, &id, "life")?,
        group: read_assignments(doc, &id, "group")?,
        id,
    })
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Column {
    id: ObjId,
    name: Vec<Variant>,
    order: Vec<Variant>,
    life: Vec<Variant>,
    cycles: Vec<Variant>,
    cards: BTreeMap<String, Card>,
}

impl Column {
    fn deletion_scope(&self) -> Self {
        let mut scope = self.clone();
        scope
            .cards
            .retain(|_, card| card.life.iter().any(|v| v.value == "active"));
        scope
    }

    fn member(&self, card: &Card) -> bool {
        card.group
            .iter()
            .any(|g| !g.value.is_empty() && self.cycles.iter().any(|cycle| cycle.value == g.value))
    }

    fn location(&self, card: &Card) -> &'static str {
        if conflicting(&card.life) || conflicting(&card.fields["place"]) {
            "conflicts"
        } else if card.life[0].value == "trash" {
            "trash"
        } else if conflicting(&self.life) {
            "conflicts"
        } else if self.life[0].value == "trash" {
            "trash"
        } else {
            "board"
        }
    }
}

fn column(doc: &Automerge, key: &str) -> Result<Column> {
    let id = object(doc, "columns", key)?;
    let root = doc.get(ROOT, "cards")?.ok_or("Missing cards")?.1;
    let mut cards = BTreeMap::new();
    for key in doc.keys(root) {
        let current = card(doc, &key)?;
        cards.insert(key, current);
    }
    cards.retain(|_, card| card.in_column(key));
    Ok(Column {
        name: read_assignments(doc, &id, "name")?,
        order: read_assignments(doc, &id, "order")?,
        life: read_assignments(doc, &id, "life")?,
        cycles: read_assignments(doc, &id, "cycles")?,
        id,
        cards,
    })
}

fn active_parent(doc: &Automerge, card: &Card) -> Result<Column> {
    if conflicting(&card.fields["place"]) {
        return Err("Resolve the card position first".into());
    }
    let key = card.fields["place"][0]
        .value
        .split(':')
        .next()
        .ok_or("Missing column")?;
    let parent = column(doc, key)?;
    if conflicting(&parent.life) || parent.life[0].value != "active" {
        return Err("Parent column must be active and its deletion resolved".into());
    }
    Ok(parent)
}

fn edit(
    doc: &mut Automerge,
    key: &str,
    observed: &Card,
    field: &str,
    value: &str,
    resolve: bool,
) -> Result<()> {
    let current = card(doc, key)?;
    if current != *observed || !FIELDS.contains(&field) {
        return Err("Stale card or unknown field: read again".into());
    }
    if (field == "title" && value.trim().is_empty())
        || (field == "completed" && !["false", "true"].contains(&value))
        || (field == "tag" && !["present", "absent"].contains(&value))
        || (field == "place" && !["c0:100", "c0:200", "c1:100", "c1:200"].contains(&value))
    {
        return Err("Invalid fixture field".into());
    }
    let parent = if resolve {
        if !conflicting(&current.fields[field]) {
            return Err("No field conflict".into());
        }
        if !conflicting(&current.life) && current.life[0].value == "active" {
            active_parent(doc, &current).ok()
        } else {
            None
        }
    } else {
        if current.blocked() || current.life[0].value != "active" {
            return Err("Ordinary card editing is blocked".into());
        }
        Some(active_parent(doc, &current)?)
    };
    // This slice checks order within the parent. Cross-column movement is EV-029.
    if field == "place"
        && !current.fields["place"]
            .iter()
            .all(|v| v.value.split(':').next() == value.split(':').next())
    {
        return Err("Use the ordering probe for ordinary cross-column moves".into());
    }
    let mut tx = doc.transaction();
    put(&mut tx, &current.id, field, value)?;
    if let Some(parent) = parent {
        put(&mut tx, &current.id, "life", "active")?;
        put(&mut tx, &parent.id, "life", "active")?;
    }
    tx.commit();
    Ok(())
}

fn create(doc: &mut Automerge, key: &str, parent: &str) -> Result<()> {
    let current = column(doc, parent)?;
    if conflicting(&current.life) || current.life[0].value != "active" {
        return Err("Creation requires an active parent".into());
    }
    let root = doc.get(ROOT, "cards")?.ok_or("Missing cards")?.1;
    if !["new-a", "new-b"].contains(&key) || doc.get(&root, key)?.is_some() {
        return Err("Unknown or existing fixture card".into());
    }
    let place = format!("{parent}:100");
    let mut tx = doc.transaction();
    let id = tx.put_object(root, key, ObjType::Map)?;
    for (field, value) in FIELDS.into_iter().zip(INITIAL) {
        put(
            &mut tx,
            &id,
            field,
            if field == "place" { &place } else { value },
        )?;
    }
    put(&mut tx, &id, "life", "active")?;
    put(&mut tx, &id, "group", "")?;
    put(&mut tx, &current.id, "life", "active")?;
    tx.commit();
    Ok(())
}

fn set_column(doc: &mut Automerge, key: &str, observed: &Column, state: &str) -> Result<()> {
    let current = column(doc, key)?;
    let resolving = conflicting(&current.life);
    let ordinary_deletion = !resolving && current.life[0].value == "active" && state == "trash";
    let fresh = if ordinary_deletion {
        current.deletion_scope() == observed.deletion_scope()
    } else {
        current == *observed
    };
    if !fresh || !["active", "trash"].contains(&state) {
        return Err("Stale column scope or invalid state: read again".into());
    }
    if !resolving
        && (current.life[0].value == state
            || conflicting(&current.name)
            || conflicting(&current.order)
            || (state == "trash" && current.deletion_scope().cards.values().any(Card::blocked)))
    {
        return Err("Ordinary column action is blocked or already applied".into());
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
            return Err("Deletion requires an existing group".into());
        }
        for card in current.cards.values() {
            // An independent earlier/own deletion is not converted into this group.
            if card.life.iter().any(|v| v.value == "active") {
                put(&mut tx, &card.id, "group", &group)?;
            }
        }
    }
    put(&mut tx, &current.id, "life", state)?;
    // Restoration changes the parent, not independent card lifecycle registers.
    tx.commit();
    Ok(())
}

fn set_card(
    doc: &mut Automerge,
    key: &str,
    observed: &Card,
    state: &str,
    target: &str,
) -> Result<()> {
    let current = card(doc, key)?;
    if current != *observed || current.blocked() || !["active", "trash"].contains(&state) {
        return Err("Stale, conflicting card or invalid state".into());
    }
    let source = column(
        doc,
        current.fields["place"][0]
            .value
            .split(':')
            .next()
            .ok_or("Missing column")?,
    )?;
    let location = source.location(&current);
    if !matches!((location, state), ("board", "trash") | ("trash", "active")) {
        return Err("Card lifecycle action requires a fresh explicit transition".into());
    }
    let destination = if state == "active" {
        let destination = column(doc, target)?;
        if conflicting(&destination.life) || destination.life[0].value != "active" {
            return Err("Choose an active column in the same fixture space".into());
        }
        Some(destination)
    } else {
        None
    };
    let mut tx = doc.transaction();
    put(&mut tx, &current.id, "life", state)?;
    put(&mut tx, &current.id, "group", "")?;
    if let Some(destination) = destination {
        put(&mut tx, &current.id, "place", &format!("{target}:100"))?;
        put(&mut tx, &destination.id, "life", "active")?;
    }
    tx.commit();
    Ok(())
}

pub(super) fn snapshot(doc: &Automerge) -> Result<Value> {
    let mut result = BTreeMap::new();
    for key in ["c0", "c1"] {
        let current = column(doc, key)?;
        let locations: BTreeMap<_, _> = current
            .cards
            .iter()
            .map(|(k, c)| {
                (
                    k,
                    json!({
                        "location": current.location(c), "member": current.member(c),
                        "blocked": c.blocked() || current.location(c) != "board",
                    }),
                )
            })
            .collect();
        result.insert(key, json!({"column": current, "locations": locations}));
    }
    Ok(serde_json::to_value(result)?)
}

fn seed(store: &mut Store) -> Result<()> {
    let mut a = Replica::new();
    let mut tx = a.doc.transaction();
    tx.put(ROOT, "column_trash_fixture", true)?;
    let columns = tx.put_object(ROOT, "columns", ObjType::Map)?;
    for key in ["c0", "c1"] {
        let id = tx.put_object(&columns, key, ObjType::Map)?;
        for (field, value) in [
            ("name", key),
            ("order", "100"),
            ("life", "active"),
            ("cycles", ""),
        ] {
            put(&mut tx, &id, field, value)?;
        }
    }
    let cards = tx.put_object(ROOT, "cards", ObjType::Map)?;
    for key in ["k0", "k1", "old", "other"] {
        let id = tx.put_object(&cards, key, ObjType::Map)?;
        for (field, value) in FIELDS.into_iter().zip(INITIAL) {
            put(
                &mut tx,
                &id,
                field,
                if key == "other" && field == "place" {
                    "c1:100"
                } else {
                    value
                },
            )?;
        }
        put(
            &mut tx,
            &id,
            "life",
            if key == "old" { "trash" } else { "active" },
        )?;
        put(&mut tx, &id, "group", "")?;
    }
    tx.commit();
    store.write("A", &a)?;
    store.write("B", &Replica::new())?;
    exchange(store)
}

mod scenario;
pub(super) use scenario::run;
#[cfg(test)]
mod tests;
