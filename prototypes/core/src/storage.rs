//! Disposable P01.05 data only. This is not the product schema or backup format.

use std::{path::PathBuf, sync::Arc, time::Duration};

use rusqlite::{Connection, OptionalExtension, TransactionBehavior, params};
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;
use utoipa::ToSchema;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
pub enum Space {
    A,
    B,
}

impl Space {
    fn key(self) -> &'static str {
        match self {
            Self::A => "A",
            Self::B => "B",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Card {
    pub id: u32,
    pub column: u32,
    pub title: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub space: Space,
    pub columns: Vec<String>,
    pub cards: Vec<Card>,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum StorageRequest {
    Seed { space: Space },
    Read { space: Space, query: String },
    Snapshot { space: Space },
    Restore { snapshot: Snapshot },
    Delete { space: Space },
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct StorageResponse {
    pub snapshot: Option<Snapshot>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct StorageError {
    pub message: String,
}

impl From<rusqlite::Error> for StorageError {
    fn from(_: rusqlite::Error) -> Self {
        Self {
            message: "SQLite operation failed; no success was confirmed".into(),
        }
    }
}

/// Cloneable handle; each operation owns its connection on a blocking worker.
#[derive(Clone)]
pub struct Storage {
    directory: PathBuf,
    slots: Arc<Semaphore>,
}

impl Storage {
    pub fn new(directory: PathBuf) -> Self {
        Self {
            directory,
            slots: Arc::new(Semaphore::new(4)),
        }
    }

    pub async fn execute(&self, request: StorageRequest) -> Result<StorageResponse, StorageError> {
        // Bound active workers; waiting for a slot never blocks the async runtime.
        let permit = self
            .slots
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| StorageError {
                message: "Storage worker unavailable".into(),
            })?;
        let directory = self.directory.clone();
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            std::fs::create_dir_all(&directory).map_err(|_| StorageError {
                message: "Prototype directory unavailable".into(),
            })?;
            let mut db = connect(directory.join("synthetic.sqlite3"))?;
            run(&mut db, request).map_err(Into::into)
        })
        .await
        .map_err(|_| StorageError {
            message: "Storage worker failed".into(),
        })?
    }
}

fn connect(path: PathBuf) -> rusqlite::Result<Connection> {
    let db = Connection::open(path)?;
    db.busy_timeout(Duration::from_secs(2))?;
    db.execute_batch(
        "PRAGMA foreign_keys = ON;
         CREATE TABLE IF NOT EXISTS prototype_spaces (
             id TEXT PRIMARY KEY CHECK(id IN ('A', 'B'))
         );
         CREATE TABLE IF NOT EXISTS prototype_columns (
             space TEXT NOT NULL REFERENCES prototype_spaces(id) ON DELETE CASCADE,
             position INTEGER NOT NULL, title TEXT NOT NULL,
             PRIMARY KEY(space, position)
         );
         CREATE TABLE IF NOT EXISTS prototype_cards (
             space TEXT NOT NULL, id INTEGER NOT NULL, column_position INTEGER NOT NULL,
             position INTEGER NOT NULL, title TEXT NOT NULL, text TEXT NOT NULL,
             PRIMARY KEY(space, id),
             FOREIGN KEY(space, column_position) REFERENCES prototype_columns(space, position)
                 ON DELETE CASCADE
         );",
    )?;
    Ok(db)
}

fn load(db: &Connection, space: Space) -> rusqlite::Result<Option<Snapshot>> {
    let exists = db
        .query_row(
            "SELECT id FROM prototype_spaces WHERE id = ?1",
            [space.key()],
            |_| Ok(()),
        )
        .optional()?
        .is_some();
    if !exists {
        return Ok(None);
    }
    let columns = db
        .prepare("SELECT title FROM prototype_columns WHERE space = ?1 ORDER BY position")?
        .query_map([space.key()], |row| row.get(0))?
        .collect::<rusqlite::Result<_>>()?;
    let cards = db
        .prepare(
            "SELECT id, column_position, title, text FROM prototype_cards
         WHERE space = ?1 ORDER BY column_position, position",
        )?
        .query_map([space.key()], |row| {
            Ok(Card {
                id: row.get(0)?,
                column: row.get(1)?,
                title: row.get(2)?,
                text: row.get(3)?,
            })
        })?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(Snapshot {
        space,
        columns,
        cards,
    }))
}

fn save(db: &Connection, snapshot: &Snapshot) -> rusqlite::Result<()> {
    let space = snapshot.space.key();
    db.execute("DELETE FROM prototype_spaces WHERE id = ?1", [space])?;
    db.execute("INSERT INTO prototype_spaces(id) VALUES (?1)", [space])?;
    for (position, title) in snapshot.columns.iter().enumerate() {
        db.execute(
            "INSERT INTO prototype_columns VALUES (?1, ?2, ?3)",
            params![space, position as u32, title],
        )?;
    }
    for (position, card) in snapshot.cards.iter().enumerate() {
        db.execute(
            "INSERT INTO prototype_cards VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                space,
                card.id,
                card.column,
                position as u32,
                card.title,
                card.text
            ],
        )?;
    }
    Ok(())
}

fn run(db: &mut Connection, request: StorageRequest) -> rusqlite::Result<StorageResponse> {
    // A single read transaction spans every table in a logical snapshot.
    // A write response is read in its own transaction before commit, never later.
    let behavior = match &request {
        StorageRequest::Read { .. } | StorageRequest::Snapshot { .. } => {
            TransactionBehavior::Deferred
        }
        _ => TransactionBehavior::Immediate,
    };
    let tx = db.transaction_with_behavior(behavior)?;
    let snapshot = match request {
        StorageRequest::Seed { space } => {
            if load(&tx, space)?.is_none() {
                save(&tx, &synthetic(space))?;
            }
            load(&tx, space)?
        }
        StorageRequest::Read { space, query } => {
            let mut snapshot = load(&tx, space)?;
            if let Some(snapshot) = &mut snapshot {
                let needle = query.trim().to_lowercase();
                snapshot.cards.retain(|card| {
                    card.title.to_lowercase().contains(&needle)
                        || card.text.to_lowercase().contains(&needle)
                });
            }
            snapshot
        }
        StorageRequest::Snapshot { space } => load(&tx, space)?,
        StorageRequest::Restore { snapshot } => {
            save(&tx, &snapshot)?;
            load(&tx, snapshot.space)?
        }
        StorageRequest::Delete { space } => {
            tx.execute("DELETE FROM prototype_spaces WHERE id = ?1", [space.key()])?;
            None
        }
    };
    tx.commit()?;
    Ok(StorageResponse { snapshot })
}

/// Exact initial profile from docs/development/prototypes.md; B is a sentinel.
pub fn synthetic(space: Space) -> Snapshot {
    let cards = (1..=30)
        .map(|id| {
            let lines = match id {
                1..=24 => 5,
                25..=29 => 100,
                _ => 200,
            };
            Card {
                id,
                column: (id - 1) / 10,
                title: format!("Карточка {id:04}"),
                text: (1..=lines)
                    .map(|n| format!("Заметка {n:04}: ёж, ЕЖ, cafe\u{301}, 😀."))
                    .collect::<Vec<_>>()
                    .join("\n"),
            }
        })
        .collect();
    Snapshot {
        space,
        columns: vec!["Колонка 1".into(), "Колонка 2".into(), "Колонка 3".into()],
        cards,
    }
}

/// Contract marker: HTTP and Android IPC call Storage::execute asynchronously.
#[utoipa::path(
    post, path = "/prototype/storage", operation_id = "storage", request_body = StorageRequest,
    responses(
        (status = 200, description = "Committed synthetic result or consistent read", body = StorageResponse),
        (status = 503, description = "Storage failed; success not confirmed", body = StorageError)
    )
)]
pub fn storage() {}

#[cfg(test)]
mod tests;
