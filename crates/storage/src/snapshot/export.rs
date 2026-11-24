use automerge::ReadDoc;
use rusqlite::Connection;

use super::{SnapshotLimits, SnapshotMetadata, SpaceSnapshot, capture};
use crate::{Storage, StorageError, StorageResult, cards};

impl Storage {
    pub async fn export_snapshot(
        &self,
        space: String,
        created_at_unix_ms: u64,
        limits: SnapshotLimits,
    ) -> StorageResult<SpaceSnapshot> {
        self.read(move |ctx| capture_export(ctx.connection(), space, created_at_unix_ms, limits))
            .await
    }
}

pub(super) fn capture_export(
    connection: &Connection,
    space: String,
    created_at_unix_ms: u64,
    limits: SnapshotLimits,
) -> StorageResult<SpaceSnapshot> {
    let mut snapshot = capture(
        connection,
        SnapshotMetadata {
            space: space.clone(),
            created_at_unix_ms,
        },
        limits,
    )?;
    let mut graph = automerge::Automerge::new();
    graph.apply_changes(snapshot.changes.iter().cloned())?;
    if crate::spaces::snapshot(&graph, &space, true)?
        .contents
        .is_none()
    {
        return Err(StorageError::InvalidDatabase);
    }
    let (count, size): (i64, i64) = connection.query_row(
        "SELECT count(*), coalesce(sum(length(CAST(title AS BLOB)) + length(CAST(description AS BLOB))), 0) FROM card_editing WHERE space = ?1 AND pending_id IS NOT NULL",
        [&space], |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let count = usize::try_from(count).map_err(|_| StorageError::Unavailable)?;
    let size = usize::try_from(size).map_err(|_| StorageError::Unavailable)?;
    if snapshot
        .changes
        .len()
        .checked_add(count)
        .is_none_or(|n| n > limits.max_changes)
        || size > limits.max_bytes
    {
        return Err(StorageError::Unavailable);
    }
    let mut statement = connection.prepare(
        "SELECT card, pending_id, title, description FROM card_editing WHERE space = ?1 AND pending_id IS NOT NULL ORDER BY card",
    )?;
    let rows = statement.query_map([&space], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (card, id, title, description) = row?;
        let object = cards::object(&graph, &card)?;
        let sequence = cards::next_sequence(&graph, &card)?;
        let history = cards::map(&graph, &object, "history")?;
        if !graph.get_all(&history, &id)?.is_empty() {
            return Err(StorageError::InvalidDatabase);
        }
        let mut tx = graph.transaction();
        cards::append_version(&mut tx, &history, &id, sequence, &title, &description)?;
        let (hash, _) = tx.commit();
        let change = graph
            .get_change_by_hash(&hash.ok_or(StorageError::InvalidDatabase)?)
            .ok_or(StorageError::InvalidDatabase)?;
        snapshot.changes.push(change);
    }
    for key in graph.keys(automerge::ROOT) {
        if let Some(card) = key.strip_prefix("card:") {
            cards::history(&graph, &cards::object(&graph, card)?)?;
        }
    }
    snapshot.changes.sort_by_key(|change| change.hash());
    snapshot.encode(limits).map_err(|error| match error {
        super::SnapshotError::LimitExceeded => StorageError::Unavailable,
        _ => StorageError::InvalidDatabase,
    })?;
    Ok(snapshot)
}
