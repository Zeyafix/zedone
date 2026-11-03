//! Fixed tags: one whole-value register per (card Object ID, tag Object ID).
use super::{conflicts::*, *};
use automerge::{ObjId, ObjType, Value as AmValue};

const TAGS: [(&str, &str, &str); 4] = [
    ("t0", "s0", "Работа"),
    ("t1", "s0", "Важно"),
    ("t2", "s1", "Работа"),
    ("t3", "s0", "Личное"),
];
const CARDS: [(&str, &str); 3] = [("k0", "s0"), ("k1", "s0"), ("k2", "s1")];

fn object(doc: &Automerge, key: &str) -> Result<ObjId> {
    match doc.get_all(ROOT, key)?.as_slice() {
        [(AmValue::Object(ObjType::Map), id)] => Ok(id.clone()),
        _ => Err("Expected exactly one fixture object".into()),
    }
}

fn scalar(doc: &Automerge, id: &ObjId, key: &str) -> Result<String> {
    match doc.get_all(id, key)?.as_slice() {
        [(value, _)] => Ok(value.as_str().ok_or("Expected fixture string")?.to_owned()),
        _ => Err("Expected immutable fixture string".into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Tag {
    id: String,
    home: String,
    name: String,
}

fn tags(doc: &Automerge) -> Result<Vec<Tag>> {
    TAGS.iter()
        .map(|(key, _, _)| {
            let id = object(doc, key)?;
            Ok(Tag {
                id: id.to_string(),
                home: scalar(doc, &id, "home")?,
                name: scalar(doc, &id, "name")?,
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Link {
    tag: String,
    variants: Vec<Variant>,
}

impl Link {
    fn present(&self) -> bool {
        self.variants.iter().any(|v| v.value == "present")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct Card {
    id: String,
    home: String,
    title: Vec<Variant>,
    text: Vec<Variant>,
    links: Vec<Link>,
}

impl Card {
    fn blocked(&self) -> bool {
        conflicting(&self.title)
            || conflicting(&self.text)
            || self.links.iter().any(|v| conflicting(&v.variants))
    }
}

fn view(doc: &Automerge, card: &str) -> Result<Card> {
    if !CARDS.iter().any(|v| v.0 == card) {
        return Err("Expected fixture card".into());
    }
    let id = object(doc, card)?;
    let home = scalar(doc, &id, "home")?;
    let mut links = Vec::new();
    for tag in tags(doc)?.into_iter().filter(|v| v.home == home) {
        let key = format!("tag:{}", tag.id);
        let variants = if doc.get_all(&id, &key)?.is_empty() {
            Vec::new() // No assignment yet; not a synthetic 'absent' operation.
        } else {
            read_assignments(doc, &id, &key)?
        };
        if variants
            .iter()
            .any(|v| v.value != "present" && v.value != "absent")
        {
            return Err("Invalid membership value".into());
        }
        links.push(Link {
            tag: tag.id,
            variants,
        });
    }
    Ok(Card {
        id: id.to_string(),
        home,
        title: read_assignments(doc, &id, "title")?,
        text: read_assignments(doc, &id, "text")?,
        links,
    })
}

fn assign(
    doc: &mut Automerge,
    card: &str,
    tag_id: &str,
    present: bool,
    observed: Option<&Card>,
) -> Result<()> {
    let current = view(doc, card)?;
    let link = current
        .links
        .iter()
        .find(|v| v.tag == tag_id)
        .ok_or("Unknown tag or tag belongs to another space")?;
    if let Some(observed) = observed {
        if current != *observed || !conflicting(&link.variants) {
            return Err("Stale choice or selected membership has no conflict".into());
        }
    } else {
        if current.blocked() {
            return Err("Ordinary card changes are blocked by its own conflict".into());
        }
        if link.present() == present {
            return Ok(());
        }
    }
    write_assignment(
        doc,
        &object(doc, card)?,
        &format!("tag:{tag_id}"),
        if present { "present" } else { "absent" },
    )
}

fn edit(doc: &mut Automerge, card: &str, field: Field, value: &str) -> Result<()> {
    if view(doc, card)?.blocked() {
        return Err("Ordinary card changes are blocked by its own conflict".into());
    }
    let key = match field {
        Field::Title => "title",
        Field::Text => "text",
    };
    write_assignment(doc, &object(doc, card)?, key, value)
}

pub(super) fn snapshot(doc: &Automerge) -> Result<Value> {
    Ok(json!({
        "tags": tags(doc)?,
        "cards": CARDS.iter().map(|v| view(doc, v.0)).collect::<Result<Vec<_>>>()?,
    }))
}

fn seed(store: &mut Store) -> Result<()> {
    let mut a = Replica::new();
    let mut tx = a.doc.transaction();
    tx.put(ROOT, "tag_fixture", true)?;
    let spaces = [
        tx.put_object(ROOT, "s0", ObjType::Map)?,
        tx.put_object(ROOT, "s1", ObjType::Map)?,
    ];
    for (key, home, name) in TAGS {
        let id = tx.put_object(ROOT, key, ObjType::Map)?;
        tx.put(&id, "home", spaces[usize::from(home == "s1")].to_string())?;
        tx.put(&id, "name", name)?;
    }
    for (key, home) in CARDS {
        let id = tx.put_object(ROOT, key, ObjType::Map)?;
        tx.put(&id, "home", spaces[usize::from(home == "s1")].to_string())?;
        for (field, value) in [("title", "Карточка — ёж 😀"), ("text", "Исходный текст")]
        {
            let assignment = tx.put_object(&id, field, ObjType::Map)?;
            tx.put(assignment, "value", value)?;
        }
    }
    tx.commit();
    // Construct only a fixed fixture, not a product creation operation.
    let tag = object(&a.doc, "t0")?.to_string();
    assign(&mut a.doc, "k0", &tag, true, None)?;
    let tag = object(&a.doc, "t3")?.to_string();
    assign(&mut a.doc, "k1", &tag, true, None)?;
    store.write("A", &a)?;
    store.write("B", &Replica::new())?;
    exchange(store)
}

mod scenario;
pub(super) use scenario::run;

#[cfg(test)]
mod tests;
