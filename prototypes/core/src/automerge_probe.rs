//! Isolated P02.01 probe, not a product API or storage format.

use std::{error::Error, path::Path, time::Duration};

use automerge::{
    Automerge, ROOT, ReadDoc,
    sync::{State, SyncDoc},
    transaction::Transactable,
};
use rusqlite::{Connection, TransactionBehavior, params};
use serde_json::{Value, json};

type Result<T> = std::result::Result<T, Box<dyn Error>>;
const ORIGINAL: &str = "Пространство — ёж, cafe\u{301}, 😀";
const EDITED: &str = "Изменено без связи\nВторая строка";

struct Replica {
    doc: Automerge,
    peer: State,
}

impl Replica {
    fn new() -> Self {
        Self {
            doc: Automerge::new(),
            peer: State::new(),
        }
    }

    fn title(&self) -> Result<String> {
        let values = self.doc.get_all(ROOT, "title")?;
        if values.len() != 1 {
            return Err("Expected one synthetic title; conflict rules belong to P02.02".into());
        }
        Ok(values[0]
            .0
            .as_str()
            .ok_or("Expected scalar string")?
            .to_owned())
    }

    fn set_title(&mut self, title: &str) -> Result<()> {
        let mut tx = self.doc.transaction();
        tx.put(ROOT, "title", title)?;
        // Markdown is a scalar field, not a character-wise merged Text object.
        tx.put(ROOT, "text", "# Заметка\nСинтетический **Markdown**")?;
        tx.commit();
        Ok(())
    }

    fn fingerprint(&self) -> Result<Value> {
        Ok(json!({
            "actor": self.doc.get_actor().to_bytes(),
            "heads": self.doc.get_heads().iter().map(ToString::to_string).collect::<Vec<_>>(),
            "changes": self.doc.get_changes(&[]).len(),
            "peer": self.peer.encode(),
            "title": self.title()?,
            "text": self.doc.get(ROOT, "text")?.ok_or("Missing text")?.0.as_str(),
        }))
    }
}

struct Store(Connection);

impl Store {
    fn open(path: &Path) -> Result<Self> {
        let db = Connection::open(path)?;
        db.busy_timeout(Duration::from_secs(2))?;
        db.execute_batch(
            "CREATE TABLE IF NOT EXISTS replicas (
            id TEXT PRIMARY KEY CHECK(id IN ('A', 'B')),
            document BLOB NOT NULL, actor BLOB NOT NULL, peer BLOB NOT NULL
        );",
        )?;
        Ok(Self(db))
    }

    fn read(&self, id: &str) -> Result<Replica> {
        let (document, actor, peer): (Vec<u8>, Vec<u8>, Vec<u8>) = self.0.query_row(
            "SELECT document, actor, peer FROM replicas WHERE id = ?1",
            [id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let mut doc = Automerge::load(&document)?;
        // load() chooses a new actor; preserve this local sequential writer explicitly.
        doc.set_actor(actor.into());
        Ok(Replica {
            doc,
            peer: State::decode(&peer)?,
        })
    }

    fn write(&mut self, id: &str, replica: &Replica) -> Result<()> {
        let tx = self
            .0
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        tx.execute(
            "INSERT INTO replicas(id, document, actor, peer) VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(id) DO UPDATE SET document=excluded.document,
                actor=excluded.actor, peer=excluded.peer",
            params![
                id,
                replica.doc.save(),
                replica.doc.get_actor().to_bytes(),
                replica.peer.encode()
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn delete(&mut self, id: &str) -> Result<()> {
        self.0.execute("DELETE FROM replicas WHERE id = ?1", [id])?;
        Ok(())
    }
}

fn exchange(store: &mut Store) -> Result<()> {
    let mut a = store.read("A")?;
    let mut b = store.read("B")?;
    // One connection session, reliable ordered delivery. Restart resets transient state.
    for _ in 0..16 {
        let mut sent = false;
        if let Some(message) = a.doc.generate_sync_message(&mut a.peer) {
            store.write("A", &a)?; // Never deliver before the sender's commit.
            b.doc.receive_sync_message(&mut b.peer, message)?;
            store.write("B", &b)?;
            sent = true;
        }
        if let Some(message) = b.doc.generate_sync_message(&mut b.peer) {
            store.write("B", &b)?;
            a.doc.receive_sync_message(&mut a.peer, message)?;
            store.write("A", &a)?;
            sent = true;
        }
        if !sent {
            if a.doc.get_heads() != b.doc.get_heads()
                || a.peer.shared_heads != a.doc.get_heads()
                || b.peer.shared_heads != b.doc.get_heads()
            {
                return Err("Sync stopped without matching document and shared heads".into());
            }
            return Ok(());
        }
    }
    Err("Synthetic sync did not settle within 16 rounds".into())
}

/// Both humans and agents run this same probe; no direct SQLite editing is required.
/// The caller owns an otherwise empty disposable directory. No product data is used.
pub fn run(directory: &Path, phase: &str) -> Result<()> {
    let mut store = Store::open(&directory.join("automerge-probe.sqlite3"))?;
    let expected_path = directory.join("expected.json");
    match phase {
        "prepare" => {
            let count: i64 = store
                .0
                .query_row("SELECT count(*) FROM replicas", [], |r| r.get(0))?;
            if count != 0 || expected_path.exists() {
                return Err("Prepare requires an empty disposable probe directory".into());
            }
            let mut a = Replica::new();
            a.set_title(ORIGINAL)?;
            store.write("A", &a)?;
            store.write("B", &Replica::new())?;
            exchange(&mut store)?;
            let mut a = store.read("A")?;
            a.set_title(EDITED)?;
            // Lose a message after commit. The next process must start a new session.
            assert!(a.doc.generate_sync_message(&mut a.peer).is_some());
            store.write("A", &a)?;
            std::fs::write(
                expected_path,
                serde_json::to_vec(&json!({
                    "A": a.fingerprint()?, "B": store.read("B")?.fingerprint()?
                }))?,
            )?;
        }
        "resume" => {
            let expected: Value = serde_json::from_slice(&std::fs::read(expected_path)?)?;
            for id in ["A", "B"] {
                let replica = store.read(id)?;
                assert_eq!(replica.fingerprint()?, expected[id]);
                assert!(!replica.peer.in_flight);
            }
            exchange(&mut store)?;
            assert_eq!(store.read("B")?.title()?, EDITED);
            let mut a = store.read("A")?;
            a.set_title(ORIGINAL)?;
            store.write("A", &a)?;
            exchange(&mut store)?;
            assert_eq!(store.read("B")?.title()?, ORIGINAL);
            let snapshot = store.read("A")?;
            store.delete("A")?;
            assert!(store.read("A").is_err());
            assert_eq!(store.read("B")?.title()?, ORIGINAL);
            store.write("A", &snapshot)?;
            assert_eq!(store.read("A")?.fingerprint()?, snapshot.fingerprint()?);
        }
        _ => return Err("Expected prepare or resume phase".into()),
    }
    println!("Automerge persistence: {phase} passed");
    Ok(())
}

#[cfg(test)]
mod tests;
