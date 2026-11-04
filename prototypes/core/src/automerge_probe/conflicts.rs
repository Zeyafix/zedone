//! Whole-field registers with explicit resolution; one synthetic card per document.
use super::*;

#[derive(Clone, Copy)]
pub(super) enum Field {
    Title,
    Text,
}

impl Field {
    fn key(self) -> &'static str {
        match self {
            Self::Title => "title",
            Self::Text => "text",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub(super) struct Variant {
    operation: String,
    pub(super) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CardView {
    heads: Vec<automerge::ChangeHash>,
    title: Vec<Variant>,
    text: Vec<Variant>,
}

pub(super) fn conflicting(variants: &[Variant]) -> bool {
    variants.iter().any(|v| v.value != variants[0].value)
}

impl Replica {
    pub(super) fn variants(&self, field: Field) -> Result<Vec<Variant>> {
        read_assignments(&self.doc, &ROOT, field.key())
    }

    fn view(&self) -> Result<CardView> {
        let mut heads = self.doc.get_heads();
        heads.sort();
        Ok(CardView {
            heads,
            title: self.variants(Field::Title)?,
            text: self.variants(Field::Text)?,
        })
    }

    fn edit(&mut self, field: Field, value: &str) -> Result<()> {
        let view = self.view()?;
        if conflicting(&view.title) || conflicting(&view.text) {
            return Err("Ordinary editing is blocked until all card conflicts are resolved".into());
        }
        self.put_field(field, value)
    }

    fn resolve(&mut self, observed: &CardView, field: Field, value: &str) -> Result<()> {
        if self.view()? != *observed {
            return Err("Stale resolution: read the current card and choose again".into());
        }
        if !conflicting(match field {
            Field::Title => &observed.title,
            Field::Text => &observed.text,
        }) {
            return Err("Selected field has no different competing values".into());
        }
        // A new assignment supersedes precisely the operations visible here.
        // An unseen concurrent edit/decision remains a variant when it arrives.
        self.put_field(field, value)
    }

    pub(super) fn put_field(&mut self, field: Field, value: &str) -> Result<()> {
        write_assignment(&mut self.doc, &ROOT, field.key(), value)
    }
}

pub(super) fn read_assignments(
    doc: &Automerge,
    object: &automerge::ObjId,
    key: &str,
) -> Result<Vec<Variant>> {
    let mut variants = doc
        .get_all(object, key)?
        .into_iter()
        .map(|(kind, operation)| {
            if kind != automerge::Value::Object(automerge::ObjType::Map) {
                return Err("Expected immutable field assignment map".into());
            }
            let value = doc
                .get(&operation, "value")?
                .ok_or("Missing field value")?
                .0;
            Ok(Variant {
                operation: operation.to_string(),
                value: value.as_str().ok_or("Expected scalar string")?.to_owned(),
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if variants.is_empty() {
        return Err("Missing synthetic field".into());
    }
    variants.sort_by(|a, b| a.operation.cmp(&b.operation));
    Ok(variants)
}

pub(super) fn write_assignment(
    doc: &mut Automerge,
    object: &automerge::ObjId,
    key: &str,
    value: &str,
) -> Result<()> {
    let mut tx = doc.transaction();
    // Scalar put(existing winner) may only delete competing values in 0.12.0.
    // A fresh immutable map gives *every* decision a distinct assignment ID,
    // including a choice of an existing text. Never mutate this map later.
    let assignment = tx.put_object(object, key, automerge::ObjType::Map)?;
    tx.put(assignment, "value", value)?;
    tx.commit();
    Ok(())
}

const LEFT: &str = "# Версия A\nПривет, ёж 😀\n**Первый** абзац";
const RIGHT: &str = "# Версия B\nДругой текст cafe\u{301}\n[Ссылка](https://example.invalid)";
const COMBINED: &str = "# Ручное объединение\nВерсии A и B сохранены явно 😀";

fn seed(store: &mut Store) -> Result<()> {
    let mut a = Replica::new();
    a.set_title(ORIGINAL)?;
    store.write("A", &a)?;
    store.write("B", &Replica::new())?;
    exchange(store)
}

fn expect(replica: &Replica, field: Field, expected: &[&str]) -> Result<()> {
    let mut actual: Vec<_> = replica
        .variants(field)?
        .into_iter()
        .map(|v| v.value)
        .collect();
    actual.sort();
    actual.dedup();
    let mut expected: Vec<_> = expected.iter().map(|v| (*v).to_owned()).collect();
    expected.sort();
    assert_eq!(actual, expected);
    Ok(())
}

pub(super) fn checkpoint(store: &Store, path: &Path, verify: bool) -> Result<()> {
    let current = json!({
        "A": store.read("A")?.fingerprint()?,
        "B": store.read("B")?.fingerprint()?,
    });
    if verify {
        let expected: Value = serde_json::from_slice(&std::fs::read(path)?)?;
        assert_eq!(current, expected);
    } else {
        std::fs::write(path, serde_json::to_vec(&current)?)?;
    }
    Ok(())
}

pub(super) fn run(directory: &Path, phase: &str) -> Result<()> {
    let mut store = Store::open(&directory.join("automerge-conflicts.sqlite3"))?;
    let checkpoint_path = directory.join("conflicts-expected.json");
    match phase {
        "conflicts-prepare" => {
            let count: i64 = store
                .0
                .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
            if count != 0 || checkpoint_path.exists() {
                return Err("Conflicts prepare requires an empty disposable store".into());
            }
            seed(&mut store)?;
            let mut a = store.read("A")?;
            let mut b = store.read("B")?;
            a.edit(Field::Title, "Новый заголовок")?;
            b.edit(Field::Text, "Новый текст без конфликта")?;
            store.write("A", &a)?;
            store.write("B", &b)?;
            exchange(&mut store)?;
            for id in ["A", "B"] {
                let replica = store.read(id)?;
                expect(&replica, Field::Title, &["Новый заголовок"])?;
                expect(&replica, Field::Text, &["Новый текст без конфликта"])?;
            }
            for (id, title, text) in [("A", "Заголовок A", LEFT), ("B", "Заголовок B", RIGHT)]
            {
                let mut replica = store.read(id)?;
                replica.edit(Field::Title, title)?;
                replica.edit(Field::Text, text)?;
                store.write(id, &replica)?;
            }
            exchange(&mut store)?;
            checkpoint(&store, &checkpoint_path, false)?;
        }
        "conflicts-resolve" => {
            checkpoint(&store, &checkpoint_path, true)?;
            for id in ["A", "B"] {
                let mut replica = store.read(id)?;
                expect(&replica, Field::Title, &["Заголовок A", "Заголовок B"])?;
                expect(&replica, Field::Text, &[LEFT, RIGHT])?;
                assert!(replica.edit(Field::Title, "Must be blocked").is_err());
                let text = if id == "A" { LEFT } else { COMBINED };
                let title = if id == "A" {
                    "Заголовок A"
                } else {
                    "Решение B"
                };
                replica.resolve(&replica.view()?, Field::Text, text)?;
                assert!(replica.edit(Field::Text, "Still blocked").is_err());
                replica.resolve(&replica.view()?, Field::Title, title)?;
                store.write(id, &replica)?;
            }
            // Both different decisions remain offline until the next process.
            checkpoint(&store, &checkpoint_path, false)?;
        }
        "conflicts-resume" => {
            checkpoint(&store, &checkpoint_path, true)?;
            exchange(&mut store)?;
            for id in ["A", "B"] {
                let replica = store.read(id)?;
                expect(&replica, Field::Title, &["Заголовок A", "Решение B"])?;
                expect(&replica, Field::Text, &[LEFT, COMBINED])?;
            }
            let mut b = store.read("B")?;
            b.resolve(&b.view()?, Field::Title, "Итоговый заголовок")?;
            b.resolve(&b.view()?, Field::Text, COMBINED)?;
            store.write("B", &b)?;
            exchange(&mut store)?;
            for text in [RIGHT, COMBINED] {
                let mut a = store.read("A")?;
                a.edit(Field::Text, text)?;
                store.write("A", &a)?;
                exchange(&mut store)?;
                for id in ["A", "B"] {
                    let replica = store.read(id)?;
                    expect(&replica, Field::Title, &["Итоговый заголовок"])?;
                    expect(&replica, Field::Text, &[text])?;
                }
            }
        }
        _ => {
            return Err("Expected conflicts-prepare, conflicts-resolve or conflicts-resume".into());
        }
    }
    println!("Automerge whole-field conflicts: {phase} passed");
    Ok(())
}

#[cfg(test)]
mod tests;
