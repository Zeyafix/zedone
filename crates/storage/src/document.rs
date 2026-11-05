use std::collections::BTreeMap;

use automerge::{ActorId, Automerge, Change, ReadDoc, sync::State, transaction::Transaction};
use rusqlite::{Connection, OptionalExtension};
use zedone_core::replay::RecordedCommand;

use crate::{StorageError, StorageResult};

struct Document {
    graph: Automerge,
    pending: Vec<Change>,
}

/// Stored evidence, not authorization and not the current object state.
pub type RecordedResult = RecordedCommand;

pub struct ReadContext<'a> {
    connection: &'a Connection,
    documents: BTreeMap<String, Document>,
}

impl<'a> ReadContext<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self {
            connection,
            documents: BTreeMap::new(),
        }
    }

    /// Only this space is loaded. No full save/view or all-space document cache.
    pub fn document(&mut self, space: &str) -> StorageResult<&Automerge> {
        if !self.documents.contains_key(space) {
            let actor: Vec<u8> = self
                .connection
                .query_row(
                    "SELECT actor FROM documents WHERE space = ?1",
                    [space],
                    |r| r.get(0),
                )
                .optional()?
                .ok_or(StorageError::Rejected(zedone_core::error::CommandError {
                    code: zedone_core::error::ErrorCode::NotFound,
                }))?;
            let mut graph = Automerge::new().with_actor(ActorId::from(actor));
            let mut statement = self
                .connection
                .prepare("SELECT hash, bytes FROM changes WHERE space = ?1")?;
            let rows = statement.query_map([space], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?;
            let mut changes = Vec::new();
            for row in rows {
                let (hash, bytes) = row?;
                let change =
                    Change::from_bytes(bytes).map_err(|_| StorageError::InvalidDatabase)?;
                if change.hash().as_ref() != hash {
                    return Err(StorageError::InvalidDatabase);
                }
                changes.push(change);
            }
            graph
                .apply_changes(changes)
                .map_err(|_| StorageError::InvalidDatabase)?;
            self.documents.insert(
                space.to_owned(),
                Document {
                    graph,
                    pending: Vec::new(),
                },
            );
        }
        Ok(&self.documents.get(space).expect("document loaded").graph)
    }

    pub fn recorded(&self, key: &[u8]) -> StorageResult<Option<RecordedResult>> {
        Ok(self
            .connection
            .query_row(
                "SELECT request, result FROM command_results WHERE key = ?1",
                [key],
                |r| {
                    Ok(RecordedResult {
                        request: r.get(0)?,
                        result: r.get(1)?,
                    })
                },
            )
            .optional()?)
    }

    /// Automerge persists shared progress, not in-flight transport state.
    pub fn peer_state(&self, space: &str, peer: &[u8]) -> StorageResult<State> {
        let bytes: Option<Vec<u8>> = self
            .connection
            .query_row(
                "SELECT state FROM peers WHERE space = ?1 AND peer = ?2",
                (space, peer),
                |r| r.get(0),
            )
            .optional()?;
        bytes
            .map(|b| State::decode(&b).map_err(|_| StorageError::InvalidDatabase))
            .unwrap_or_else(|| Ok(State::new()))
    }
}

pub struct WriteContext<'a> {
    read: ReadContext<'a>,
    failed: bool,
}

impl<'a> WriteContext<'a> {
    pub(crate) fn new(connection: &'a Connection) -> Self {
        Self {
            read: ReadContext::new(connection),
            failed: false,
        }
    }

    pub fn read(&mut self) -> &mut ReadContext<'a> {
        &mut self.read
    }

    /// Trusted infrastructure only. Never replace a receipt or prune it by age.
    pub fn record(&mut self, key: &[u8], command: &RecordedCommand) -> StorageResult<()> {
        let result = self
            .read
            .connection
            .execute(
                "INSERT INTO command_results(key, request, result) VALUES (?1, ?2, ?3)",
                (key, &command.request, &command.result),
            )
            .map(|_| ());
        self.checked(result.map_err(StorageError::from))
    }

    // Even if a callback catches a mutator's error, partial work cannot commit.
    fn checked<T>(&mut self, result: StorageResult<T>) -> StorageResult<T> {
        self.failed |= result.is_err();
        result
    }

    pub fn create_document(&mut self, space: &str) -> StorageResult<()> {
        let result = (|| {
            let graph = Automerge::new();
            self.read.connection.execute(
                "INSERT INTO documents(space, actor) VALUES (?1, ?2)",
                (space, graph.get_actor().to_bytes()),
            )?;
            self.read.documents.insert(
                space.to_owned(),
                Document {
                    graph,
                    pending: Vec::new(),
                },
            );
            Ok(())
        })();
        self.checked(result)
    }

    /// Only the transaction is mutable: callers cannot replace the graph/actor.
    /// Domain data and its history must be edited together inside this callback.
    pub fn edit<R>(
        &mut self,
        space: &str,
        work: impl FnOnce(&mut Transaction<'_>) -> StorageResult<R>,
    ) -> StorageResult<R> {
        let result = (|| {
            self.read.document(space)?;
            let document = self.read.documents.get_mut(space).expect("document loaded");
            let mut tx = document.graph.transaction();
            let result = work(&mut tx)?;
            let (hash, _) = tx.commit();
            if let Some(hash) = hash {
                document.pending.push(
                    document
                        .graph
                        .get_change_by_hash(&hash)
                        .ok_or(StorageError::InvalidDatabase)?,
                );
            }
            Ok(result)
        })();
        self.checked(result)
    }

    /// Keep every accepted change, including those waiting for dependencies.
    /// This avoids relying on get_changes/save_after, which omit orphans.
    pub fn apply_changes(&mut self, space: &str, changes: Vec<Change>) -> StorageResult<()> {
        let result = (|| {
            self.read.document(space)?;
            let document = self.read.documents.get_mut(space).expect("document loaded");
            document.graph.apply_changes(changes.clone())?;
            document.pending.extend(changes);
            Ok(())
        })();
        self.checked(result)
    }

    pub fn set_peer_state(&mut self, space: &str, peer: &[u8], state: &State) -> StorageResult<()> {
        let result = (|| {
            let graph = self.read.document(space)?;
            if state
                .shared_heads
                .iter()
                .any(|hash| graph.get_change_by_hash(hash).is_none())
            {
                return Err(StorageError::InvalidDatabase);
            }
            self.read.connection.execute(
                "INSERT INTO peers(space, peer, state) VALUES (?1, ?2, ?3)
             ON CONFLICT(space, peer) DO UPDATE SET state = excluded.state",
                (space, peer, state.encode()),
            )?;
            Ok(())
        })();
        self.checked(result)
    }

    pub(crate) fn flush(self) -> StorageResult<()> {
        if self.failed {
            return Err(StorageError::Unavailable);
        }
        let mut insert = self.read.connection.prepare(
            "INSERT INTO changes(space, hash, bytes) VALUES (?1, ?2, ?3) ON CONFLICT(space, hash) DO NOTHING",
        )?;
        for (space, document) in self.read.documents {
            for change in document.pending {
                insert.execute((&space, change.hash().as_ref(), change.raw_bytes()))?;
            }
        }
        Ok(())
    }
}
