//! Trusted infrastructure for space archives, not an import/restore command.
//! Decoding never writes a file or database and grants no authority.

use automerge::{Automerge, Change};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{Storage, StorageError, StorageResult};

const MAGIC: &[u8; 8] = b"ZEDSPACE";
const VERSION: u32 = 1;
const HEADER: usize = 28;
const DIGEST: usize = 32;

/// Caller-selected admission budgets, not product limits or a heap bound.
#[derive(Debug, Clone, Copy)]
pub struct SnapshotLimits {
    pub max_bytes: usize,
    pub max_changes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SnapshotMetadata {
    /// Opaque source identity; importing independently must allocate new IDs.
    pub space: String,
    /// Supplied by the trusted caller, never used to issue rights or expire data.
    pub created_at_unix_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapshotError {
    Invalid,
    UnsupportedVersion,
    LimitExceeded,
}

impl std::fmt::Display for SnapshotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for SnapshotError {}

/// Complete accepted change log, including changes awaiting dependencies.
/// Private fields prevent callers from emitting an unchecked archive.
#[derive(Debug)]
pub struct SpaceSnapshot {
    metadata: SnapshotMetadata,
    changes: Vec<Change>,
}

impl SpaceSnapshot {
    pub fn metadata(&self) -> &SnapshotMetadata {
        &self.metadata
    }

    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn encode(&self, limits: SnapshotLimits) -> Result<Vec<u8>, SnapshotError> {
        let metadata = metadata_bytes(&self.metadata)?;
        let size = archive_size(metadata.len(), &self.changes)?;
        check_limits(size, self.changes.len(), limits)?;
        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(size)
            .map_err(|_| SnapshotError::LimitExceeded)?;
        bytes.extend_from_slice(MAGIC);
        bytes.extend_from_slice(&VERSION.to_le_bytes());
        bytes.extend_from_slice(&(metadata.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&(self.changes.len() as u64).to_le_bytes());
        bytes.extend_from_slice(&metadata);
        for change in &self.changes {
            bytes.extend_from_slice(&(change.raw_bytes().len() as u64).to_le_bytes());
            bytes.extend_from_slice(change.raw_bytes());
        }
        let digest = Sha256::digest(&bytes);
        bytes.extend_from_slice(&digest);
        Ok(bytes)
    }

    /// The caller must also bound file/network reads before supplying the slice.
    pub fn decode(bytes: &[u8], limits: SnapshotLimits) -> Result<Self, SnapshotError> {
        check_limits(bytes.len(), 0, limits)?;
        if bytes.len() < HEADER + DIGEST || &bytes[..8] != MAGIC {
            return Err(SnapshotError::Invalid);
        }
        if bytes[8..12] != VERSION.to_le_bytes() {
            return Err(SnapshotError::UnsupportedVersion);
        }
        let (body, digest) = bytes.split_at(bytes.len() - DIGEST);
        if Sha256::digest(body).as_slice() != digest {
            return Err(SnapshotError::Invalid);
        }
        let mut input = &body[12..];
        let metadata_len = length(&mut input)?;
        let count = length(&mut input)?;
        check_limits(bytes.len(), count, limits)?;
        let metadata: SnapshotMetadata = serde_json::from_slice(take(&mut input, metadata_len)?)
            .map_err(|_| SnapshotError::Invalid)?;
        metadata_bytes(&metadata)?;
        // Every record requires a length and at least one byte. Never reserve
        // memory from an unverified count or length declared in the archive.
        if count > input.len() / 9 {
            return Err(SnapshotError::Invalid);
        }
        let mut changes = Vec::new();
        for _ in 0..count {
            let len = length(&mut input)?;
            let change = parse_change(take(&mut input, len)?)?;
            if changes
                .last()
                .is_some_and(|previous: &Change| previous.hash() >= change.hash())
            {
                return Err(SnapshotError::Invalid);
            }
            changes.push(change);
        }
        if !input.is_empty() {
            return Err(SnapshotError::Invalid);
        }
        validate_graph(&changes)?;
        Ok(Self { metadata, changes })
    }
}

fn metadata_bytes(metadata: &SnapshotMetadata) -> Result<Vec<u8>, SnapshotError> {
    if metadata.space.is_empty() {
        return Err(SnapshotError::Invalid);
    }
    serde_json::to_vec(metadata).map_err(|_| SnapshotError::Invalid)
}

fn archive_size(metadata_len: usize, changes: &[Change]) -> Result<usize, SnapshotError> {
    changes
        .iter()
        .try_fold(HEADER + DIGEST + metadata_len, |size, change| {
            size.checked_add(8)
                .and_then(|s| s.checked_add(change.raw_bytes().len()))
                .ok_or(SnapshotError::LimitExceeded)
        })
}

fn check_limits(bytes: usize, changes: usize, limits: SnapshotLimits) -> Result<(), SnapshotError> {
    if bytes > limits.max_bytes || changes > limits.max_changes {
        Err(SnapshotError::LimitExceeded)
    } else {
        Ok(())
    }
}

fn take<'a>(input: &mut &'a [u8], len: usize) -> Result<&'a [u8], SnapshotError> {
    let (value, rest) = input.split_at_checked(len).ok_or(SnapshotError::Invalid)?;
    *input = rest;
    Ok(value)
}

fn length(input: &mut &[u8]) -> Result<usize, SnapshotError> {
    usize::try_from(u64::from_le_bytes(take(input, 8)?.try_into().unwrap()))
        .map_err(|_| SnapshotError::Invalid)
}

fn parse_change(bytes: &[u8]) -> Result<Change, SnapshotError> {
    // V1 only admits uncompressed Automerge change chunks, as emitted by
    // Change::raw_bytes. Reject compressed chunks BEFORE the library inflates.
    if bytes.get(8) != Some(&1) {
        return Err(SnapshotError::Invalid);
    }
    let change = Change::try_from(bytes).map_err(|_| SnapshotError::Invalid)?;
    // Change::try_from verifies operations, but does not reject a mismatched
    // chunk checksum. The first four hash bytes are the Automerge checksum.
    if change.raw_bytes() != bytes || bytes[4..8] != change.hash().as_ref()[..4] {
        return Err(SnapshotError::Invalid);
    }
    Ok(change)
}

fn validate_graph(changes: &[Change]) -> Result<(), SnapshotError> {
    Automerge::new()
        .apply_changes(changes.iter().cloned())
        .map_err(|_| SnapshotError::Invalid)
}

impl Storage {
    /// Consistent committed snapshot on the worker. Unauthenticated trusted
    /// infrastructure, like `read`; must not be exposed as a public command.
    /// Future authorization and capture must share the same transaction.
    pub async fn snapshot(
        &self,
        space: String,
        created_at_unix_ms: u64,
        limits: SnapshotLimits,
    ) -> StorageResult<SpaceSnapshot> {
        let registry = self.registry;
        self.run(move |connection| {
            let tx = connection.transaction()?;
            registry.check_version(&tx)?;
            let snapshot = capture(
                &tx,
                SnapshotMetadata {
                    space,
                    created_at_unix_ms,
                },
                limits,
            )?;
            tx.commit()?;
            Ok(snapshot)
        })
        .await
    }
}

pub(crate) fn capture(
    connection: &Connection,
    metadata: SnapshotMetadata,
    limits: SnapshotLimits,
) -> StorageResult<SpaceSnapshot> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE space = ?1)",
        [&metadata.space],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(zedone_core::error::CommandError {
            code: zedone_core::error::ErrorCode::NotFound,
        }
        .into());
    }
    let metadata_len = metadata_bytes(&metadata)
        .map_err(|_| StorageError::InvalidDatabase)?
        .len();
    let (count, raw_size): (i64, i64) = connection.query_row(
        "SELECT count(*), coalesce(sum(length(bytes)), 0) FROM changes WHERE space = ?1",
        [&metadata.space],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )?;
    let count = usize::try_from(count).map_err(|_| StorageError::Unavailable)?;
    let raw_size = usize::try_from(raw_size).map_err(|_| StorageError::Unavailable)?;
    let size = count
        .checked_mul(8)
        .and_then(|s| s.checked_add(raw_size))
        .and_then(|s| s.checked_add(metadata_len))
        .and_then(|s| s.checked_add(HEADER + DIGEST))
        .ok_or(StorageError::Unavailable)?;
    check_limits(size, count, limits).map_err(|_| StorageError::Unavailable)?;
    let mut statement =
        connection.prepare("SELECT hash, bytes FROM changes WHERE space = ?1 ORDER BY hash")?;
    let rows = statement.query_map([&metadata.space], |r| {
        Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
    })?;
    let mut changes = Vec::new();
    for row in rows {
        let (hash, bytes) = row?;
        let change = parse_change(&bytes).map_err(|_| StorageError::InvalidDatabase)?;
        if change.hash().as_ref() != hash {
            return Err(StorageError::InvalidDatabase);
        }
        changes.push(change);
    }
    validate_graph(&changes).map_err(|_| StorageError::InvalidDatabase)?;
    Ok(SpaceSnapshot { metadata, changes })
}

#[cfg(test)]
mod tests;
