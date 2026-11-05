//! Disposable topology and whole-position registers; no product commands.
use super::{conflicts::*, *};
use automerge::{ObjId, ObjType, Value as AmValue};
use serde::{Deserialize, Serialize};

mod ranks;
use ranks::Rank;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Space,
    Column,
    Card,
}

// key, kind, immutable home space, initial parent, initial rank/name.
const FIXTURE: &[(&str, Kind, &str, &str, u32, &str)] = &[
    ("s", Kind::Space, "", "", 0, "М"),
    ("a", Kind::Space, "", "", 0, "А"),
    ("z", Kind::Space, "", "", 0, "Я"),
    ("t", Kind::Space, "", "", 0, "М"),
    ("c0", Kind::Column, "s", "s", 100, "Колонка"),
    ("c1", Kind::Column, "s", "s", 200, "Колонка"),
    ("c2", Kind::Column, "s", "s", 300, "Колонка"),
    ("c3", Kind::Column, "s", "s", 400, "Колонка"),
    ("foreign", Kind::Column, "a", "a", 100, "Колонка"),
    ("k0", Kind::Card, "s", "c0", 100, "Карточка"),
    ("k1", Kind::Card, "s", "c0", 200, "Карточка"),
    ("k2", Kind::Card, "s", "c0", 300, "Карточка"),
    ("k3", Kind::Card, "s", "c0", 400, "Карточка"),
];

fn kind(key: &str) -> Result<Kind> {
    FIXTURE
        .iter()
        .find(|v| v.0 == key)
        .map(|v| v.1)
        .ok_or("Unknown fixture object".into())
}

fn object(doc: &Automerge, key: &str) -> Result<ObjId> {
    kind(key)?;
    match doc.get_all(ROOT, key)?.as_slice() {
        [(AmValue::Object(ObjType::Map), id)] => Ok(id.clone()),
        _ => Err("Missing or ambiguous fixture object".into()),
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Place {
    parent: String,
    rank: Rank,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct Item {
    id: String,
    home: Option<String>,
    name: Vec<Variant>,
    text: Option<Vec<Variant>>,
    place: Option<Vec<Variant>>,
}

impl Item {
    fn blocked(&self) -> bool {
        conflicting(&self.name)
            || self.text.as_ref().is_some_and(|v| conflicting(v))
            || self.place.as_ref().is_some_and(|v| conflicting(v))
    }

    fn position(&self) -> Result<Option<Place>> {
        match &self.place {
            Some(v) if !conflicting(v) => Ok(Some(serde_json::from_str(&v[0].value)?)),
            _ => Ok(None),
        }
    }
}

fn view(doc: &Automerge, key: &str) -> Result<Item> {
    let id = object(doc, key)?;
    let home = doc
        .get(&id, "home")?
        .map(|(v, _)| v.as_str().map(str::to_owned).ok_or("Invalid home space"))
        .transpose()?;
    Ok(Item {
        id: id.to_string(),
        home,
        name: read_assignments(doc, &id, "name")?,
        text: (kind(key)? == Kind::Card)
            .then(|| read_assignments(doc, &id, "text"))
            .transpose()?,
        place: (kind(key)? != Kind::Space)
            .then(|| read_assignments(doc, &id, "place"))
            .transpose()?,
    })
}

fn edit(
    doc: &mut Automerge,
    key: &str,
    field: &str,
    value: &str,
    observed: Option<&Item>,
) -> Result<()> {
    let current = view(doc, key)?;
    let variants = match field {
        "name" => &current.name,
        "text" => current.text.as_ref().ok_or("Only cards have text")?,
        _ => return Err("Unknown editable field".into()),
    };
    check_change(&current, variants, observed)?;
    write_assignment(doc, &object(doc, key)?, field, value)
}

fn check_change(current: &Item, variants: &[Variant], observed: Option<&Item>) -> Result<()> {
    if let Some(observed) = observed {
        if current != observed || !conflicting(variants) {
            return Err("Stale choice or selected field has no conflict".into());
        }
    } else if current.blocked() {
        return Err("Ordinary changes blocked by this object's own conflict".into());
    }
    Ok(())
}

fn ordered(doc: &Automerge, parent: &str) -> Result<Vec<String>> {
    let parent = object(doc, parent)?.to_string();
    let mut children = Vec::new();
    for (key, _, _, _, _, _) in FIXTURE {
        let item = view(doc, key)?;
        if let Some(place) = item.position()?
            && place.parent == parent
        {
            children.push((place.rank, item.id, key.to_string()));
        }
    }
    children.sort();
    Ok(children.into_iter().map(|(_, _, key)| key).collect())
}

// The caller chooses a gap in the current list with the moved object removed.
// Parent and rank are committed as one register, never merged independently.
fn relocate(
    doc: &mut Automerge,
    key: &str,
    parent: &str,
    left: Option<&str>,
    right: Option<&str>,
    observed: Option<&Item>,
) -> Result<()> {
    let item = view(doc, key)?;
    check_change(
        &item,
        item.place.as_ref().ok_or("Spaces have automatic order")?,
        observed,
    )?;
    let destination = view(doc, parent)?;
    match (kind(key)?, kind(parent)?) {
        (Kind::Column, Kind::Space) if item.home.as_ref() == Some(&destination.id) => {}
        (Kind::Card, Kind::Column) if item.home == destination.home => {}
        _ => return Err("Invalid destination or cross-space move".into()),
    }
    let children: Vec<_> = ordered(doc, parent)?
        .into_iter()
        .filter(|v| v != key)
        .collect();
    let index = match left {
        Some(left) => {
            children
                .iter()
                .position(|v| v == left)
                .ok_or("Missing left neighbour")?
                + 1
        }
        None => 0,
    };
    if children.get(index).map(String::as_str) != right {
        return Err("Neighbours no longer describe an adjacent gap".into());
    }
    let rank = |key: &str| -> Result<Rank> {
        Ok(view(doc, key)?
            .position()?
            .ok_or("Conflicting neighbour")?
            .rank)
    };
    let left = left.map(rank).transpose()?;
    let right = right.map(rank).transpose()?;
    let place = Place {
        parent: destination.id,
        rank: ranks::between(left.as_ref(), right.as_ref(), &item.id)?,
    };
    write_assignment(
        doc,
        &object(doc, key)?,
        "place",
        &serde_json::to_string(&place)?,
    )
}

pub(super) fn snapshot(doc: &Automerge) -> Result<Value> {
    let items: Vec<_> = FIXTURE
        .iter()
        .map(|v| view(doc, v.0))
        .collect::<Result<_>>()?;
    let mut spaces = Vec::new();
    let mut groups = Vec::new();
    for (key, kind, _, _, _, _) in FIXTURE {
        let item = view(doc, key)?;
        if *kind == Kind::Space {
            let conflict = conflicting(&item.name);
            // Exact Unicode scalar order, ID tie-break; conflicts ordered by ID.
            spaces.push((
                !conflict,
                if conflict {
                    String::new()
                } else {
                    item.name[0].value.clone()
                },
                item.id.clone(),
            ));
        }
        if *kind != Kind::Card {
            let conflicts: Vec<_> = items
                .iter()
                .filter(|v| {
                    v.home.as_ref() == Some(&item.id)
                        && v.place.as_ref().is_some_and(|p| conflicting(p))
                })
                .map(|v| &v.id)
                .collect();
            let children: Vec<_> = ordered(doc, key)?
                .iter()
                .map(|k| object(doc, k).map(|id| id.to_string()))
                .collect::<Result<_>>()?;
            groups.push(json!({"parent": item.id, "conflicts": conflicts, "ordinary": children}));
        }
    }
    spaces.sort();
    Ok(json!({"items": items, "spaces": spaces, "groups": groups}))
}

fn seed(store: &mut Store) -> Result<()> {
    let mut a = Replica::new();
    let mut tx = a.doc.transaction();
    tx.put(ROOT, "order_fixture", true)?;
    let mut ids: std::collections::BTreeMap<&str, String> = std::collections::BTreeMap::new();
    for (key, kind, home, parent, rank, name) in FIXTURE {
        let id = tx.put_object(ROOT, *key, ObjType::Map)?;
        let assignment = tx.put_object(&id, "name", ObjType::Map)?;
        tx.put(assignment, "value", *name)?;
        if *kind != Kind::Space {
            tx.put(&id, "home", &ids[home])?;
            let place = Place {
                parent: ids[parent].clone(),
                rank: Rank::initial(*rank, &id.to_string()),
            };
            let assignment = tx.put_object(&id, "place", ObjType::Map)?;
            tx.put(assignment, "value", serde_json::to_string(&place)?)?;
        }
        if *kind == Kind::Card {
            let assignment = tx.put_object(&id, "text", ObjType::Map)?;
            tx.put(assignment, "value", "Исходный текст")?;
        }
        ids.insert(*key, id.to_string());
    }
    tx.commit();
    store.write("A", &a)?;
    store.write("B", &Replica::new())?;
    exchange(store)
}

pub(super) fn run(directory: &Path, phase: &str) -> Result<()> {
    let mut store = Store::open(&directory.join("automerge-order.sqlite3"))?;
    let checkpoint = directory.join("order-expected.json");
    if phase == "order-prepare" {
        let count: i64 = store
            .0
            .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
        if count != 0 || checkpoint.exists() {
            return Err("Order prepare requires an empty disposable store".into());
        }
        seed(&mut store)?;
        for (id, parent, left, right) in
            [("A", "c1", None, Some("c0")), ("B", "c2", Some("c3"), None)]
        {
            let mut replica = store.read(id)?;
            relocate(&mut replica.doc, "k1", parent, None, None, None)?;
            relocate(&mut replica.doc, "c1", "s", left, right, None)?;
            edit(&mut replica.doc, "s", "name", id, None)?;
            // Different cards concurrently enter the same empty gap of c3.
            relocate(
                &mut replica.doc,
                if id == "A" { "k2" } else { "k3" },
                "c3",
                None,
                None,
                None,
            )?;
            store.write(id, &replica)?;
        }
        exchange(&mut store)?;
    } else {
        let expected: Value = serde_json::from_slice(&std::fs::read(&checkpoint)?)?;
        for id in ["A", "B"] {
            assert_eq!(store.read(id)?.fingerprint()?, expected[id]);
        }
        match phase {
            "order-resolve" => {
                for (id, parent) in [("A", "c1"), ("B", "c2")] {
                    let mut replica = store.read(id)?;
                    let card = view(&replica.doc, "k1")?;
                    assert!(card.blocked());
                    assert!(edit(&mut replica.doc, "k1", "text", "Blocked", None).is_err());
                    // Column conflicts do not block its unconflicted contents.
                    edit(&mut replica.doc, "k0", "text", "Доступное содержимое", None)?;
                    relocate(&mut replica.doc, "k1", parent, None, None, Some(&card))?;
                    let column = view(&replica.doc, "c1")?;
                    let (left, right) = if id == "A" {
                        (None, Some("c0"))
                    } else {
                        (Some("c3"), None)
                    };
                    relocate(&mut replica.doc, "c1", "s", left, right, Some(&column))?;
                    store.write(id, &replica)?;
                }
            }
            "order-resume" => {
                exchange(&mut store)?;
                let mut a = store.read("A")?;
                let observed = view(&a.doc, "k1")?;
                assert!(observed.blocked());
                relocate(&mut a.doc, "k1", "c1", None, None, Some(&observed))?;
                // Return through the same operation; no identity or history replacement.
                relocate(&mut a.doc, "k1", "c0", Some("k0"), None, None)?;
                let column = view(&a.doc, "c1")?;
                assert!(column.blocked());
                relocate(&mut a.doc, "c1", "s", Some("c0"), Some("c2"), Some(&column))?;
                let space = view(&a.doc, "s")?;
                edit(&mut a.doc, "s", "name", "М", Some(&space))?;
                store.write("A", &a)?;
                exchange(&mut store)?;
                assert_eq!(
                    snapshot(&store.read("A")?.doc)?,
                    snapshot(&store.read("B")?.doc)?
                );
            }
            _ => return Err("Unknown order phase".into()),
        }
    }
    std::fs::write(
        checkpoint,
        serde_json::to_vec(
            &json!({"A":store.read("A")?.fingerprint()?, "B":store.read("B")?.fingerprint()?}),
        )?,
    )?;
    println!("Automerge order: {phase} passed");
    Ok(())
}

#[cfg(test)]
mod tests;

pub(super) fn measure(directory: &Path) -> Result<Value> {
    use super::space_trash::lifecycle::measurements::{stats, timed};
    let mut doc = Automerge::new();
    let mut right = Rank::initial(u32::MAX / 2, "fixed-right");
    let mut stages = vec![];
    let mut samples = vec![];
    for i in 0..4096 {
        let mut tx = doc.transaction();
        let id = tx.put_object(ROOT, format!("rank-{i}"), ObjType::Map)?;
        let (rank, ms) = timed(|| ranks::between(None, Some(&right), &id.to_string()))?;
        let encoded = serde_json::to_string(&rank)?;
        tx.put(&id, "value", encoded.as_str())?;
        tx.commit();
        samples.push(ms);
        right = rank;
        if [1024, 4096].contains(&(i + 1)) {
            stages.push(json!({"insertions": i+1, "rank_components": serde_json::to_value(&right)?.as_array().unwrap().len(),
                "last_rank_utf8_bytes": encoded.len(), "packed_bytes": doc.save().len(), "allocation": stats(&samples)}));
        }
    }
    std::fs::write(
        directory.join("rank-graph.automerge"),
        doc.save_and_verify()?,
    )?;
    Ok(
        json!({"scope": "rank allocator with durable object-ID tie breaks; repeated insertion before same right neighbour", "stages": stages}),
    )
}
