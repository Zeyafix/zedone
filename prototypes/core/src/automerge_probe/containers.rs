//! Fixed synthetic topology; names never identify or replace a container.
use super::{conflicts::*, *};
use automerge::{ObjId, ObjType, Value as AmValue};

#[derive(Clone, Copy)]
enum Node {
    Space,
    Left,
    Right,
    Card,
}

const NODES: [Node; 4] = [Node::Space, Node::Left, Node::Right, Node::Card];

impl Node {
    fn key(self) -> &'static str {
        match self {
            Self::Space => "space",
            Self::Left => "left",
            Self::Right => "right",
            Self::Card => "card",
        }
    }

    fn object(self, doc: &Automerge) -> Result<ObjId> {
        match doc.get_all(ROOT, self.key())?.as_slice() {
            [(AmValue::Object(ObjType::Map), id)] => Ok(id.clone()),
            _ => Err("Expected exactly one stable fixture object".into()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
struct NodeView {
    id: String,
    parent: Option<String>,
    name: Vec<Variant>,
    text: Option<Vec<Variant>>,
}

impl NodeView {
    fn blocked(&self) -> bool {
        conflicting(&self.name) || self.text.as_ref().is_some_and(|v| conflicting(v))
    }
}

fn view(doc: &Automerge, node: Node) -> Result<NodeView> {
    let object = node.object(doc)?;
    let parent = match doc.get_all(&object, "parent")?.as_slice() {
        [] if matches!(node, Node::Space) => None,
        [(value, _)] if !matches!(node, Node::Space) => Some(
            value
                .as_str()
                .ok_or("Expected immutable parent ID")?
                .to_owned(),
        ),
        _ => return Err("Missing or ambiguous fixture parent".into()),
    };
    Ok(NodeView {
        id: object.to_string(),
        parent,
        name: read_assignments(doc, &object, "name")?,
        text: if matches!(node, Node::Card) {
            Some(read_assignments(doc, &object, "text")?)
        } else {
            None
        },
    })
}

pub(super) fn snapshot(doc: &Automerge) -> Result<Value> {
    Ok(serde_json::to_value(
        NODES
            .iter()
            .map(|n| view(doc, *n))
            .collect::<Result<Vec<_>>>()?,
    )?)
}

fn change(
    doc: &mut Automerge,
    node: Node,
    key: &str,
    value: &str,
    observed: Option<&NodeView>,
) -> Result<()> {
    let current = view(doc, node)?;
    let selected = match key {
        "name" => &current.name,
        "text" => current
            .text
            .as_ref()
            .ok_or("Only the fixture card has text")?,
        _ => return Err("Unknown fixture field".into()),
    };
    if let Some(observed) = observed {
        // Child edits neither unlock this object nor invalidate an unchanged choice.
        if current != *observed || !conflicting(selected) {
            return Err("Stale resolution or selected field has no conflict".into());
        }
    } else if current.blocked() {
        return Err("Ordinary changes of this object are blocked by its own conflict".into());
    }
    let object = node.object(doc)?;
    write_assignment(doc, &object, key, value)
}

fn seed(store: &mut Store) -> Result<()> {
    let mut a = Replica::new();
    // Construct only a fixed fixture, atomically. No product creation endpoint.
    let mut tx = a.doc.transaction();
    let mut parents = Vec::new();
    for node in NODES {
        let object = tx.put_object(ROOT, node.key(), ObjType::Map)?;
        if !matches!(node, Node::Space) {
            let parent = if matches!(node, Node::Card) { 1 } else { 0 };
            tx.put(&object, "parent", &parents[parent])?;
        }
        let assignment = tx.put_object(&object, "name", ObjType::Map)?;
        tx.put(assignment, "value", "Одинаковое имя")?;
        if matches!(node, Node::Card) {
            let text = tx.put_object(&object, "text", ObjType::Map)?;
            tx.put(text, "value", "Исходный текст")?;
        }
        parents.push(object.to_string());
    }
    tx.commit();
    store.write("A", &a)?;
    store.write("B", &Replica::new())?;
    exchange(store)
}

fn expect(doc: &Automerge, node: Node, names: &[&str]) -> Result<()> {
    let mut actual: Vec<_> = view(doc, node)?.name.into_iter().map(|v| v.value).collect();
    actual.sort();
    actual.dedup();
    let mut names: Vec<_> = names.iter().map(|s| s.to_string()).collect();
    names.sort();
    assert_eq!(actual, names);
    Ok(())
}

pub(super) fn run(directory: &Path, phase: &str) -> Result<()> {
    let mut store = Store::open(&directory.join("automerge-containers.sqlite3"))?;
    let checkpoint = directory.join("containers-expected.json");
    if phase == "containers-prepare" {
        let count: i64 = store
            .0
            .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
        if count != 0 || checkpoint.exists() {
            return Err("Containers prepare requires an empty disposable store".into());
        }
        seed(&mut store)?;
        for (id, name) in [("A", "Название A"), ("B", "Название B")] {
            let mut replica = store.read(id)?;
            for node in [Node::Space, Node::Left] {
                change(&mut replica.doc, node, "name", name, None)?;
            }
            store.write(id, &replica)?;
        }
        exchange(&mut store)?;
    } else {
        let expected: Value = serde_json::from_slice(&std::fs::read(&checkpoint)?)?;
        for id in ["A", "B"] {
            assert_eq!(store.read(id)?.fingerprint()?, expected[id]);
        }
        match phase {
            "containers-resolve" => {
                for id in ["A", "B"] {
                    let mut replica = store.read(id)?;
                    for node in [Node::Space, Node::Left] {
                        expect(&replica.doc, node, &["Название A", "Название B"])?;
                        let observed = view(&replica.doc, node)?;
                        assert!(change(&mut replica.doc, node, "name", "Blocked", None).is_err());
                        // The parent's name conflict does not block the card.
                        change(
                            &mut replica.doc,
                            Node::Card,
                            "text",
                            "Доступное содержимое",
                            None,
                        )?;
                        let choice = if id == "A" {
                            "Название A"
                        } else {
                            "Название B"
                        };
                        change(&mut replica.doc, node, "name", choice, Some(&observed))?;
                        assert_eq!(view(&replica.doc, node)?.id, observed.id);
                    }
                    store.write(id, &replica)?;
                }
            }
            "containers-resume" => {
                exchange(&mut store)?;
                let mut a = store.read("A")?;
                for node in [Node::Space, Node::Left] {
                    expect(&a.doc, node, &["Название A", "Название B"])?;
                    let observed = view(&a.doc, node)?;
                    change(&mut a.doc, node, "name", "Итог", Some(&observed))?;
                    change(&mut a.doc, node, "name", "Обратная правка", None)?;
                    change(&mut a.doc, node, "name", "Итог", None)?;
                    assert_eq!(view(&a.doc, node)?.id, observed.id);
                }
                change(&mut a.doc, Node::Card, "text", "Исходный текст", None)?;
                store.write("A", &a)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    let replica = store.read(id)?;
                    for node in [Node::Space, Node::Left] {
                        expect(&replica.doc, node, &["Итог"])?;
                    }
                    expect(&replica.doc, Node::Right, &["Одинаковое имя"])?;
                    assert_eq!(snapshot(&replica.doc)?, snapshot(&a.doc)?);
                }
            }
            _ => return Err("Unknown containers phase".into()),
        }
    }
    std::fs::write(
        checkpoint,
        serde_json::to_vec(&json!({
            "A": store.read("A")?.fingerprint()?, "B": store.read("B")?.fingerprint()?,
        }))?,
    )?;
    println!("Automerge container identity/names: {phase} passed");
    Ok(())
}

#[cfg(test)]
mod tests;
