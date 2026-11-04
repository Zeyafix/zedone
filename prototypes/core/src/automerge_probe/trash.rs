//! P02.05.1: fixed cards, active containers, one tag; no purge or product API.
use super::{conflicts::*, *};
use automerge::{ObjId, ObjType, Value as AmValue, transaction::Transaction};
use std::collections::BTreeMap;

const FIELDS: [&str; 5] = ["title", "text", "completed", "place", "tag"];
const INITIAL: [&str; 5] = ["Карточка", "# Ёж 😀", "false", "c0:100", "present"];

fn object(doc: &Automerge, key: &str) -> Result<ObjId> {
    if !["k0", "k1"].contains(&key) {
        return Err("Unknown fixture card".into());
    }
    match doc.get_all(ROOT, key)?.as_slice() {
        [(AmValue::Object(ObjType::Map), id)] => Ok(id.clone()),
        _ => Err("Expected one stable card ID".into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Card {
    id: ObjId,
    fields: BTreeMap<String, Vec<Variant>>,
    life: Vec<Variant>,
}

impl Card {
    fn blocked(&self) -> bool {
        conflicting(&self.life) || self.fields.values().any(|v| conflicting(v))
    }

    fn location(&self) -> &str {
        if conflicting(&self.life) || conflicting(&self.fields["place"]) {
            "conflicts"
        } else if self.life[0].value == "trash" {
            "trash"
        } else {
            "board"
        }
    }
}

fn view(doc: &Automerge, key: &str) -> Result<Card> {
    let id = object(doc, key)?;
    Ok(Card {
        fields: FIELDS
            .iter()
            .map(|field| Ok(((*field).to_owned(), read_assignments(doc, &id, field)?)))
            .collect::<Result<_>>()?,
        life: read_assignments(doc, &id, "life")?,
        id,
    })
}

fn put(tx: &mut Transaction<'_>, id: &ObjId, key: &str, value: &str) -> Result<()> {
    let assignment = tx.put_object(id, key, ObjType::Map)?;
    tx.put(assignment, "value", value)?;
    Ok(())
}

fn edit(
    doc: &mut Automerge,
    key: &str,
    observed: &Card,
    field: &str,
    value: &str,
    resolve: bool,
) -> Result<()> {
    let current = view(doc, key)?;
    if current != *observed || !FIELDS.contains(&field) {
        return Err("Stale card or unknown field: read again".into());
    }
    if resolve {
        if !conflicting(&current.fields[field]) {
            return Err("Selected field has no conflict".into());
        }
    } else if current.blocked() || current.life[0].value != "active" {
        return Err("Ordinary editing requires an active card without conflicts".into());
    }
    if (field == "completed" && !["false", "true"].contains(&value))
        || (field == "tag" && !["absent", "present"].contains(&value))
        || (field == "place" && !["c0:100", "c0:200", "c1:100"].contains(&value))
        || (field == "title" && value.trim().is_empty())
    {
        return Err("Invalid fixture field value or foreign position".into());
    }
    let mut tx = doc.transaction();
    put(&mut tx, &current.id, field, value)?;
    // Resolving text while lifecycle is disputed/trashed never restores implicitly.
    // An ordinary edit (or active field resolution) is evidence against unseen trash.
    if !conflicting(&current.life) && current.life[0].value == "active" {
        put(&mut tx, &current.id, "life", "active")?;
    }
    tx.commit();
    Ok(())
}

fn set_life(doc: &mut Automerge, key: &str, observed: &Card, state: &str) -> Result<()> {
    let current = view(doc, key)?;
    if current != *observed || !["active", "trash"].contains(&state) {
        return Err("Stale card or invalid lifecycle choice".into());
    }
    if !conflicting(&current.life) && (current.blocked() || current.life[0].value == state) {
        return Err("Ordinary trash/restore requires a resolved card and a different state".into());
    }
    // A lifecycle choice does not resolve conflicting text/position/membership.
    write_assignment(doc, &current.id, "life", state)
}

pub(super) fn snapshot(doc: &Automerge) -> Result<Value> {
    let mut result = Vec::new();
    for key in ["k0", "k1"] {
        let card = view(doc, key)?;
        result.push(json!({"card": card, "location": card.location(), "blocked": card.blocked()}));
    }
    Ok(json!(result))
}

fn seed(store: &mut Store) -> Result<()> {
    let mut a = Replica::new();
    let mut tx = a.doc.transaction();
    tx.put(ROOT, "trash_fixture", true)?;
    for key in ["k0", "k1"] {
        let id = tx.put_object(ROOT, key, ObjType::Map)?;
        tx.put(&id, "home", "synthetic-space")?;
        for (field, value) in FIELDS.iter().zip(INITIAL) {
            put(&mut tx, &id, field, value)?;
        }
        put(&mut tx, &id, "life", "active")?;
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
