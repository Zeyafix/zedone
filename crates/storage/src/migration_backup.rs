//! Trusted backup infrastructure; never an authenticated migration/restore API.

use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rusqlite::{
    Connection, OpenFlags, TransactionBehavior,
    backup::{Backup, StepResult},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

pub mod publication;

use crate::{
    Storage, StorageError, StorageResult, schema,
    snapshot::{SnapshotLimits, SnapshotMetadata, SpaceSnapshot},
};

/// Admission budgets selected by trusted infrastructure, not product limits.
#[derive(Clone, Copy, Debug)]
pub struct BackupLimits {
    pub max_database_bytes: u64,
    pub max_spaces: usize,
    pub max_total_snapshot_bytes: u64,
    pub snapshot: SnapshotLimits,
}

/// An isolated, verified candidate. Retained on drop, including lost responses.
/// Its files contain installation secrets and must stay in a private directory.
#[derive(Debug)]
pub struct PreparedBackup {
    directory: TempDir,
}

impl PreparedBackup {
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    /// Re-read all files; the earlier successful preparation is not a lease.
    pub fn verify(&self, limits: BackupLimits) -> StorageResult<()> {
        verify(self.path(), limits)
    }

    /// Delete only this candidate, never a previous successful backup.
    pub fn discard(self) -> StorageResult<()> {
        io(self.directory.close())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileDigest {
    bytes: u64,
    sha256: [u8; 32],
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    created_at_unix_ms: u64,
    database: FileDigest,
    // Names are derived from indices, never from space IDs or file input.
    spaces: Vec<FileDigest>,
}

impl Storage {
    /// Capture a consistent full database and all space archives on the worker.
    /// `folder` must already exist: no fallback or automatic directory choice.
    /// Future migration must capture under its own uninterrupted writer lock.
    pub async fn prepare_migration_backup(
        &self,
        folder: impl Into<PathBuf>,
        created_at_unix_ms: u64,
        limits: BackupLimits,
    ) -> StorageResult<PreparedBackup> {
        let folder = folder.into();
        self.run(move |connection| {
            prepare(connection, &folder, created_at_unix_ms, limits, |_| Ok(()))
        })
        .await
    }
}

fn io<T>(result: std::io::Result<T>) -> StorageResult<T> {
    result.map_err(|_| StorageError::Unavailable)
}

pub(crate) fn readonly(path: &Path) -> StorageResult<Connection> {
    Ok(Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?)
}

pub(crate) fn spaces_v1(
    connection: &Connection,
    limits: BackupLimits,
) -> StorageResult<Vec<String>> {
    let count: i64 = connection.query_row("SELECT count(*) FROM documents", [], |r| r.get(0))?;
    if !usize::try_from(count).is_ok_and(|count| count <= limits.max_spaces) {
        return Err(StorageError::Unavailable);
    }
    Ok(connection
        .prepare("SELECT space FROM documents ORDER BY space")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?)
}

#[cfg(test)]
fn validate_database(connection: &Connection) -> StorageResult<()> {
    validate_database_with(connection, schema::REGISTRY)
}

fn validate_database_with(
    connection: &Connection,
    registry: schema::Registry,
) -> StorageResult<()> {
    schema::check_schema_layout(connection, registry.known(connection)?)?;
    let mut statement = connection.prepare("PRAGMA integrity_check")?;
    let result = statement
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if result != ["ok"]
        || connection
            .prepare("PRAGMA foreign_key_check")?
            .query([])?
            .next()?
            .is_some()
    {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}

fn digest(path: &Path, max_bytes: u64) -> StorageResult<FileDigest> {
    let mut file = io(File::open(path))?;
    let bytes = io(file.metadata())?.len();
    if bytes > max_bytes {
        return Err(StorageError::Unavailable);
    }
    let mut hasher = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    let mut count = 0u64;
    loop {
        let read = io(file.read(&mut buffer))?;
        if read == 0 {
            break;
        }
        count = count
            .checked_add(read as u64)
            .ok_or(StorageError::Unavailable)?;
        if count > max_bytes {
            return Err(StorageError::Unavailable);
        }
        hasher.update(&buffer[..read]);
    }
    if count != bytes {
        return Err(StorageError::Unavailable);
    }
    Ok(FileDigest {
        bytes,
        sha256: hasher.finalize().into(),
    })
}

fn read_bounded(path: &Path, max_bytes: u64) -> StorageResult<Vec<u8>> {
    let mut bytes = Vec::new();
    io(io(File::open(path))?
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes))?;
    if bytes.len() as u64 > max_bytes {
        return Err(StorageError::Unavailable);
    }
    Ok(bytes)
}

fn write_new(path: &Path, bytes: &[u8]) -> StorageResult<()> {
    let mut file = io(OpenOptions::new().write(true).create_new(true).open(path))?;
    io(file.write_all(bytes))?;
    io(file.sync_all())
}

fn archive(
    connection: &Connection,
    space: String,
    date: u64,
    limits: BackupLimits,
    registry: schema::Registry,
) -> StorageResult<Vec<u8>> {
    (registry.known(connection)?.archive.capture)(
        connection,
        SnapshotMetadata {
            space,
            created_at_unix_ms: date,
        },
        limits.snapshot,
    )?
    .encode(limits.snapshot)
    .map_err(|_| StorageError::Unavailable)
}

fn manifest_limit(limits: BackupLimits) -> u64 {
    // Fixed JSON fields and at most 32 three-digit digest bytes per entry.
    (limits.max_spaces as u64)
        .saturating_add(1)
        .saturating_mul(256)
        .saturating_add(256)
}

fn verify(folder: &Path, limits: BackupLimits) -> StorageResult<()> {
    verify_with(folder, limits, schema::REGISTRY)
}

pub(crate) fn verify_with(
    folder: &Path,
    limits: BackupLimits,
    registry: schema::Registry,
) -> StorageResult<()> {
    let manifest: Manifest = serde_json::from_slice(&read_bounded(
        &folder.join("manifest.json"),
        manifest_limit(limits),
    )?)
    .map_err(|_| StorageError::InvalidDatabase)?;
    if manifest.version != 1 || manifest.spaces.len() > limits.max_spaces {
        return Err(StorageError::InvalidDatabase);
    }
    // A stray WAL/journal must not contribute un-hashed database state.
    // Only the expected regular files belong to a self-contained candidate.
    let mut count = 0usize;
    for entry in io(fs::read_dir(folder))? {
        let entry = io(entry)?;
        count += 1;
        if !io(entry.file_type())?.is_file() || count > manifest.spaces.len() + 2 {
            return Err(StorageError::InvalidDatabase);
        }
    }
    if count != manifest.spaces.len() + 2 {
        return Err(StorageError::InvalidDatabase);
    }
    let database = folder.join("database.sqlite");
    if digest(&database, limits.max_database_bytes)? != manifest.database {
        return Err(StorageError::InvalidDatabase);
    }
    let connection = readonly(&database)?;
    validate_database_with(&connection, registry)?;
    let names = (registry.known(&connection)?.archive.spaces)(&connection, limits)?;
    if names.len() != manifest.spaces.len() {
        return Err(StorageError::InvalidDatabase);
    }
    let mut total = 0u64;
    for (index, (space, expected)) in names.into_iter().zip(manifest.spaces).enumerate() {
        total = total
            .checked_add(expected.bytes)
            .ok_or(StorageError::Unavailable)?;
        if total > limits.max_total_snapshot_bytes {
            return Err(StorageError::Unavailable);
        }
        let path = folder.join(format!("space-{index}.bin"));
        if digest(&path, limits.snapshot.max_bytes as u64)? != expected {
            return Err(StorageError::InvalidDatabase);
        }
        let bytes = read_bounded(&path, limits.snapshot.max_bytes as u64)?;
        SpaceSnapshot::decode(&bytes, limits.snapshot)
            .map_err(|_| StorageError::InvalidDatabase)?;
        if bytes
            != archive(
                &connection,
                space,
                manifest.created_at_unix_ms,
                limits,
                registry,
            )?
        {
            return Err(StorageError::InvalidDatabase);
        }
    }
    Ok(())
}

fn prepare(
    connection: &mut Connection,
    folder: &Path,
    date: u64,
    limits: BackupLimits,
    checkpoint: impl Fn(Checkpoint) -> StorageResult<()>,
) -> StorageResult<PreparedBackup> {
    // A second read-only connection is necessary: Backup::step on a source
    // connection with an active write transaction returns SQLITE_LOCKED.
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    schema::check_layout(&tx)?;
    let candidate = capture(&tx, folder, date, limits, checkpoint)?;
    tx.rollback()?;
    Ok(candidate)
}

// Caller owns the uninterrupted writer transaction, including eventual commit.
fn capture(
    locked: &rusqlite::Transaction<'_>,
    folder: &Path,
    date: u64,
    limits: BackupLimits,
    checkpoint: impl Fn(Checkpoint) -> StorageResult<()>,
) -> StorageResult<PreparedBackup> {
    capture_with(locked, folder, date, limits, schema::REGISTRY, checkpoint)
}

fn capture_with(
    locked: &rusqlite::Transaction<'_>,
    folder: &Path,
    date: u64,
    limits: BackupLimits,
    registry: schema::Registry,
    checkpoint: impl Fn(Checkpoint) -> StorageResult<()>,
) -> StorageResult<PreparedBackup> {
    registry.validate(locked)?;
    let path = PathBuf::from(locked.path().ok_or(StorageError::Unavailable)?);
    let source = readonly(&path)?;
    let pages: u64 =
        source.pragma_query_value(None, "page_count", |r| r.get::<_, u32>(0).map(u64::from))?;
    let page_size: u64 =
        source.pragma_query_value(None, "page_size", |r| r.get::<_, u32>(0).map(u64::from))?;
    if pages
        .checked_mul(page_size)
        .is_none_or(|bytes| bytes > limits.max_database_bytes)
    {
        return Err(StorageError::Unavailable);
    }
    (registry.known(&source)?.archive.spaces)(&source, limits)?;
    let mut directory = io(tempfile::Builder::new()
        .prefix("zedone-migration-candidate-")
        .tempdir_in(folder))?;
    let database = directory.path().join("database.sqlite");
    let mut destination = Connection::open(&database)?;
    checkpoint(Checkpoint::BeforeCopy)?;
    {
        let backup = Backup::new(&source, &mut destination)?;
        if backup.step(-1)? != StepResult::Done {
            return Err(StorageError::Unavailable);
        }
    }
    // Make the candidate self-contained before close/re-open verification.
    destination.pragma_update(None, "journal_mode", "DELETE")?;
    destination.close().map_err(|_| StorageError::Unavailable)?;
    io(io(OpenOptions::new().read(true).write(true).open(&database))?.sync_all())?;
    checkpoint(Checkpoint::AfterCopy)?;
    let copied = readonly(&database)?;
    validate_database_with(&copied, registry)?;
    let mut manifest = Manifest {
        version: 1,
        created_at_unix_ms: date,
        database: digest(&database, limits.max_database_bytes)?,
        spaces: Vec::new(),
    };
    let mut total = 0u64;
    for (index, space) in (registry.known(&copied)?.archive.spaces)(&copied, limits)?
        .into_iter()
        .enumerate()
    {
        let bytes = archive(&copied, space, date, limits, registry)?;
        total = total
            .checked_add(bytes.len() as u64)
            .ok_or(StorageError::Unavailable)?;
        if total > limits.max_total_snapshot_bytes {
            return Err(StorageError::Unavailable);
        }
        let path = directory.path().join(format!("space-{index}.bin"));
        write_new(&path, &bytes)?;
        manifest
            .spaces
            .push(digest(&path, limits.snapshot.max_bytes as u64)?);
        checkpoint(Checkpoint::AfterSpace)?;
    }
    drop(copied);
    let bytes = serde_json::to_vec(&manifest).map_err(|_| StorageError::Unavailable)?;
    write_new(&directory.path().join("manifest.json"), &bytes)?;
    checkpoint(Checkpoint::BeforeVerify)?;
    verify_with(directory.path(), limits, registry)?;
    checkpoint(Checkpoint::AfterVerify)?;
    // Successful candidates survive dropped futures/handles. Only explicit
    // discard deletes one. Crash remnants are never promoted by this module.
    directory.disable_cleanup(true);
    Ok(PreparedBackup { directory })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Checkpoint {
    BeforeCopy,
    AfterCopy,
    AfterSpace,
    BeforeVerify,
    AfterVerify,
}

#[cfg(test)]
pub(crate) mod tests;
