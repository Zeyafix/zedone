//! Explicit names for a connected collision; existing identities never merge here.
use super::*;

pub(super) const CARD_FIELDS: [&str; 4] = ["title", "text", "completed", "column"];

pub(super) fn card_conflicting(doc: &Automerge, card: &ObjId) -> Result<bool> {
    for field in CARD_FIELDS {
        if conflicting(&read_assignments(doc, card, field)?) {
            return Ok(true);
        }
    }
    let home = scalar(doc, card, "home")?;
    for tag in tags(doc)?.iter().filter(|v| v.home == home) {
        let values = links(doc, card, tag)?;
        if values.iter().any(|v| v.value != values[0].value) {
            return Ok(true);
        }
    }
    Ok(false)
}

fn edit(doc: &mut Automerge, card: &str, field: &str, value: &str) -> Result<()> {
    if !["k0", "k1", "k2"].contains(&card) || !CARD_FIELDS.contains(&field) {
        return Err("Unknown fixture card or field".into());
    }
    let card = object(doc, card)?;
    if card_conflicting(doc, &card)? {
        return Err("Ordinary editing blocked by the card's own conflict".into());
    }
    if field == "completed" && !["false", "true"].contains(&value) {
        return Err("Expected a completion flag".into());
    }
    if field == "column" {
        let mut valid = false;
        for key in ["c0", "c1", "c2"] {
            let column = object(doc, key)?;
            valid |= column.to_string() == value
                && scalar(doc, &column, "home")? == scalar(doc, &card, "home")?;
        }
        if !valid {
            return Err("Unknown column or a different space".into());
        }
    }
    // A tag's name conflict alone never blocks independent card fields.
    write_assignment(doc, &card, field, value)
}

fn view(doc: &Automerge, anchor: &ObjId) -> Result<Vec<Tag>> {
    let home = tag(doc, anchor)?.home;
    Ok(tags(doc)?.into_iter().filter(|v| v.home == home).collect())
}

fn overlaps(left: &Tag, right: &Tag) -> bool {
    left.names.iter().any(|a| {
        right
            .names
            .iter()
            .any(|b| name_key(&a.value) == name_key(&b.value))
    })
}

fn resolve(
    doc: &mut Automerge,
    anchor: &ObjId,
    observed: &[Tag],
    choices: &[(ObjId, &str)],
) -> Result<()> {
    let current = view(doc, anchor)?;
    if current != observed {
        return Err("Stale names: read the current space catalog and choose again".into());
    }
    let start = current
        .iter()
        .position(|v| v.ids.contains(anchor))
        .ok_or("Unknown tag")?;
    if !current[start].conflicting() {
        return Err("Selected tag has no name conflict".into());
    }
    let mut group = vec![start];
    loop {
        let before = group.len();
        for i in 0..current.len() {
            if !group.contains(&i) && group.iter().any(|j| overlaps(&current[i], &current[*j])) {
                group.push(i);
            }
        }
        if group.len() == before {
            break;
        }
    }
    if group.iter().any(|i| current[*i].identity_ambiguous) {
        return Err("Ambiguous creation identities need a separate decision".into());
    }
    if group.len() != choices.len() {
        return Err("Choose one name for every tag in the collision".into());
    }
    let mut selected = Vec::new();
    let mut keys = Vec::new();
    for (id, name) in choices {
        let index = current
            .iter()
            .position(|v| v.ids.contains(id))
            .ok_or("Unknown choice ID")?;
        let key = name_key(name);
        if !group.contains(&index)
            || selected.contains(&index)
            || key.is_empty()
            || keys.contains(&key)
            || current.iter().enumerate().any(|(i, v)| {
                !group.contains(&i) && v.names.iter().any(|n| name_key(&n.value) == key)
            })
        {
            return Err("Invalid, duplicate, missing or occupied choice".into());
        }
        selected.push(index);
        keys.push(key);
    }
    let context = doc.get_heads();
    let mut tx = doc.transaction();
    for (id, name) in choices {
        put(&mut tx, id, "name", name, &context)?;
    }
    // All names form one Automerge change; SQLite persists the complete document.
    tx.commit();
    Ok(())
}

mod scenario;
pub(in super::super) use scenario::run;

#[cfg(test)]
mod tests;
