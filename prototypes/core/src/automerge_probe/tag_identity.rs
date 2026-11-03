//! P02.04.2.1: isolated creation identities and causal registers across aliases.
use super::{conflicts::*, *};
use automerge::{ChangeHash, ObjId, ObjType, Value as AmValue, transaction::Transaction};

fn object(doc: &Automerge, key: &str) -> Result<ObjId> {
    match doc.get_all(ROOT, key)?.as_slice() {
        [(AmValue::Object(_), id)] => Ok(id.clone()),
        _ => Err("Expected one fixture object".into()),
    }
}

fn scalar(doc: &Automerge, id: &ObjId, key: &str) -> Result<String> {
    match doc.get_all(id, key)?.as_slice() {
        [(value, _)] => Ok(value.as_str().ok_or("Expected string")?.to_owned()),
        _ => Err("Expected immutable scalar".into()),
    }
}

// A bounded probe policy, not the final product Unicode contract.
fn name_key(name: &str) -> String {
    name.trim().to_lowercase()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Candidate {
    operation: ObjId,
    value: String,
    context: Vec<ChangeHash>,
}

fn candidates(doc: &Automerge, id: &ObjId, key: &str) -> Result<Vec<Candidate>> {
    doc.get_all(id, key)?
        .into_iter()
        .map(|(kind, operation)| {
            if kind != AmValue::Object(ObjType::Map) {
                return Err("Expected whole-value register".into());
            }
            Ok(Candidate {
                value: scalar(doc, &operation, "value")?,
                context: serde_json::from_str(&scalar(doc, &operation, "context")?)?,
                operation,
            })
        })
        .collect()
}

fn put(
    tx: &mut Transaction<'_>,
    id: &ObjId,
    key: &str,
    value: &str,
    context: &[ChangeHash],
) -> Result<()> {
    let assignment = tx.put_object(id, key, ObjType::Map)?;
    tx.put(&assignment, "value", value)?;
    tx.put(&assignment, "context", serde_json::to_string(context)?)?;
    Ok(())
}

// Registers belonging to different creation IDs still share causal precedence.
// Never pick a winner by arrival order or treat 'absent' as deleting a register.
fn frontier(doc: &Automerge, values: Vec<Candidate>) -> Result<Vec<Candidate>> {
    let mut result = Vec::new();
    for value in &values {
        let mut superseded = false;
        for other in &values {
            if doc
                .get_at(&value.operation, "value", &other.context)?
                .is_some()
            {
                superseded = true;
                break;
            }
        }
        if !superseded {
            result.push(value.clone());
        }
    }
    result.sort_by_key(|v| v.operation.to_string());
    Ok(result)
}

#[derive(Clone)]
struct Creation {
    id: ObjId,
    home: String,
    birth: String,
    context: Vec<ChangeHash>,
}

fn concurrent(doc: &Automerge, left: &Creation, right: &Creation) -> Result<bool> {
    Ok(doc.get_at(&left.id, "home", &right.context)?.is_none()
        && doc.get_at(&right.id, "home", &left.context)?.is_none())
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Tag {
    ids: Vec<ObjId>,
    home: String,
    names: Vec<Candidate>,
    blocked: bool,
}

impl Tag {
    fn conflicting(&self) -> bool {
        self.blocked
            || self
                .names
                .iter()
                .any(|v| name_key(&v.value) != name_key(&self.names[0].value))
    }

    fn name(&self) -> &str {
        // Sorted by assignment ID: one original spelling, never forced lowercase.
        &self.names[0].value
    }
}

fn tags(doc: &Automerge) -> Result<Vec<Tag>> {
    let catalog = object(doc, "catalog")?;
    let mut creations = Vec::new();
    for index in 0..doc.length(&catalog) {
        let (_, id) = doc.get(&catalog, index)?.ok_or("Missing creation")?;
        creations.push(Creation {
            home: scalar(doc, &id, "home")?,
            birth: scalar(doc, &id, "birth")?,
            context: serde_json::from_str(&scalar(doc, &id, "context")?)?,
            id,
        });
    }
    creations.sort_by_key(|v| v.id.to_string());
    let mut groups: Vec<usize> = (0..creations.len()).collect();
    for (i, left) in creations.iter().enumerate() {
        for (j, right) in creations.iter().enumerate().skip(i + 1) {
            if left.home == right.home
                && name_key(&left.birth) == name_key(&right.birth)
                && concurrent(doc, left, right)?
            {
                let (from, to) = (groups[j].max(groups[i]), groups[j].min(groups[i]));
                for group in &mut groups {
                    if *group == from {
                        *group = to;
                    }
                }
            }
        }
    }
    // A late creation must not bridge two tags that were already known distinct.
    // Preserve each raw identity and refuse ordinary actions in that unsupported
    // cohort; the next slice owns explicit collision resolution, not this probe.
    let mut ambiguous = vec![false; creations.len()];
    for (i, left) in creations.iter().enumerate() {
        for (j, right) in creations.iter().enumerate().skip(i + 1) {
            if groups[i] == groups[j] && !concurrent(doc, left, right)? {
                ambiguous[groups[i]] = true;
            }
        }
    }
    let mut result = Vec::new();
    for i in 0..creations.len() {
        if ambiguous[groups[i]] {
            result.push(Tag {
                ids: vec![creations[i].id.clone()],
                home: creations[i].home.clone(),
                names: candidates(doc, &creations[i].id, "name")?,
                blocked: true,
            });
            continue;
        }
        if groups[i] != i {
            continue;
        }
        let mut ids = Vec::new();
        let mut names = Vec::new();
        for (j, creation) in creations.iter().enumerate() {
            if groups[j] == i {
                ids.push(creation.id.clone());
                names.extend(candidates(doc, &creation.id, "name")?);
            }
        }
        result.push(Tag {
            ids,
            home: creations[i].home.clone(),
            names: frontier(doc, names)?,
            blocked: false,
        });
    }
    // Detect but do not resolve rename/name collisions before P02.04.2.2.
    for i in 0..result.len() {
        for j in i + 1..result.len() {
            if result[i].home == result[j].home
                && result[i].names.iter().any(|left| {
                    result[j]
                        .names
                        .iter()
                        .any(|right| name_key(&left.value) == name_key(&right.value))
                })
            {
                result[i].blocked = true;
                result[j].blocked = true;
            }
        }
    }
    result.sort_by_key(|v| v.ids[0].to_string());
    Ok(result)
}

fn tag(doc: &Automerge, id: &ObjId) -> Result<Tag> {
    tags(doc)?
        .into_iter()
        .find(|tag| tag.ids.contains(id))
        .ok_or_else(|| "Unknown creation ID".into())
}

fn validate_name(tags: &[Tag], home: &str, name: &str, except: Option<&ObjId>) -> Result<()> {
    let key = name_key(name);
    if key.is_empty()
        || tags.iter().any(|tag| {
            tag.home == home
                && !except.is_some_and(|id| tag.ids.contains(id))
                && tag.names.iter().any(|v| name_key(&v.value) == key)
        })
    {
        return Err("Empty or occupied tag name".into());
    }
    Ok(())
}

fn create(doc: &mut Automerge, home: &str, name: &str) -> Result<ObjId> {
    let homes = [
        object(doc, "s0")?.to_string(),
        object(doc, "s1")?.to_string(),
    ];
    if !homes.iter().any(|v| v == home) {
        return Err("Unknown space".into());
    }
    validate_name(&tags(doc)?, home, name, None)?;
    let catalog = object(doc, "catalog")?;
    let index = doc.length(&catalog);
    let context = doc.get_heads();
    let mut tx = doc.transaction();
    let id = tx.insert_object(&catalog, index, ObjType::Map)?;
    tx.put(&id, "home", home)?;
    tx.put(&id, "birth", name)?;
    tx.put(&id, "context", serde_json::to_string(&context)?)?;
    put(&mut tx, &id, "name", name, &context)?;
    tx.commit();
    Ok(id)
}

fn rename(doc: &mut Automerge, id: &ObjId, name: &str) -> Result<()> {
    let current = tag(doc, id)?;
    if current.conflicting() {
        return Err("Resolve conflicting names before ordinary rename".into());
    }
    validate_name(&tags(doc)?, &current.home, name, Some(id))?;
    let context = doc.get_heads();
    let mut tx = doc.transaction();
    put(&mut tx, id, "name", name, &context)?;
    tx.commit();
    Ok(())
}

fn links(doc: &Automerge, card: &ObjId, tag: &Tag) -> Result<Vec<Candidate>> {
    let mut values = Vec::new();
    for id in &tag.ids {
        values.extend(candidates(doc, card, &format!("tag:{id}"))?);
    }
    frontier(doc, values)
}

fn assign(doc: &mut Automerge, card: &str, id: &ObjId, present: bool) -> Result<()> {
    if !["k0", "k1", "k2"].contains(&card) {
        return Err("Unknown fixture card".into());
    }
    let card = object(doc, card)?;
    let tag = tag(doc, id)?;
    let values = links(doc, &card, &tag)?;
    let mut own_conflict = false;
    for other in tags(doc)?.iter().filter(|v| v.home == tag.home) {
        let other_values = links(doc, &card, other)?;
        own_conflict |= other_values
            .iter()
            .any(|v| v.value != other_values[0].value);
    }
    if tag.home != scalar(doc, &card, "home")?
        || tag.conflicting()
        || own_conflict
        || conflicting(&read_assignments(doc, &card, "text")?)
    {
        return Err("Foreign tag or own membership/text/name conflict".into());
    }
    let value = if present { "present" } else { "absent" };
    if values.first().map_or(!present, |v| v.value == value) {
        return Ok(());
    }
    let context = doc.get_heads();
    let mut tx = doc.transaction();
    put(&mut tx, &card, &format!("tag:{id}"), value, &context)?;
    tx.commit();
    Ok(())
}

pub(super) fn snapshot(doc: &Automerge) -> Result<Value> {
    let tags = tags(doc)?;
    let mut cards = Vec::new();
    for key in ["k0", "k1", "k2"] {
        let id = object(doc, key)?;
        let home = scalar(doc, &id, "home")?;
        let mut memberships = Vec::new();
        for tag in tags.iter().filter(|v| v.home == home) {
            memberships.push(json!({"tag": tag.ids[0], "values": links(doc, &id, tag)?}));
        }
        cards.push(json!({"id": id, "home": home, "links": memberships,
            "text": read_assignments(doc, &id, "text")?}));
    }
    Ok(json!({"tags": tags, "cards": cards}))
}

fn seed(store: &mut Store) -> Result<()> {
    let mut a = Replica::new();
    let mut tx = a.doc.transaction();
    tx.put(ROOT, "identity_fixture", true)?;
    tx.put_object(ROOT, "catalog", ObjType::List)?;
    let homes = [
        tx.put_object(ROOT, "s0", ObjType::Map)?,
        tx.put_object(ROOT, "s1", ObjType::Map)?,
    ];
    for (key, home) in [("k0", 0), ("k1", 0), ("k2", 1)] {
        let card = tx.put_object(ROOT, key, ObjType::Map)?;
        tx.put(&card, "home", homes[home].to_string())?;
        let text = tx.put_object(&card, "text", ObjType::Map)?;
        tx.put(text, "value", "Исходный текст")?;
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
