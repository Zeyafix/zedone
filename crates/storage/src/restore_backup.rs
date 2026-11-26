use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use rusqlite::{Connection, TransactionBehavior};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use crate::{
    Storage, StorageError, StorageResult, schema,
    snapshot::{SnapshotLimits, SnapshotMetadata, SpaceSnapshot, export::capture_export},
};

pub(crate) const ARCHIVE: &str = "space.bin";

#[derive(Debug)]
pub struct PreparedRestoreBackup {
    pub(crate) directory: TempDir,
    pub(crate) metadata: SnapshotMetadata,
    pub(crate) bytes: usize,
    pub(crate) digest: [u8; 32],
}

impl PreparedRestoreBackup {
    pub fn path(&self) -> &Path {
        self.directory.path()
    }

    pub fn metadata(&self) -> &SnapshotMetadata {
        &self.metadata
    }

    pub fn verify(&self, limits: SnapshotLimits) -> StorageResult<()> {
        directory(self.path())?;
        let mut entries = io(fs::read_dir(self.path()))?;
        let entry = io(entries.next().ok_or(StorageError::InvalidDatabase)?)?;
        if entry.file_name() != ARCHIVE
            || !io(entry.file_type())?.is_file()
            || entries.next().is_some()
            || self.bytes > limits.max_bytes
        {
            return Err(StorageError::InvalidDatabase);
        }
        let mut file = io(File::open(entry.path()))?;
        if io(file.metadata())?.len() != self.bytes as u64 {
            return Err(StorageError::InvalidDatabase);
        }
        let mut bytes = Vec::new();
        io((&mut file)
            .take((self.bytes as u64).saturating_add(1))
            .read_to_end(&mut bytes))?;
        if bytes.len() != self.bytes || Sha256::digest(&bytes).as_slice() != self.digest {
            return Err(StorageError::InvalidDatabase);
        }
        let snapshot =
            SpaceSnapshot::decode(&bytes, limits).map_err(|_| StorageError::InvalidDatabase)?;
        if snapshot.metadata() != &self.metadata {
            return Err(StorageError::InvalidDatabase);
        }
        Ok(())
    }

    pub fn discard(self) -> StorageResult<()> {
        let parent = self
            .path()
            .parent()
            .ok_or(StorageError::Unavailable)?
            .to_path_buf();
        io(self.directory.close())?;
        sync_directory(&parent)
    }
}

impl Storage {
    pub async fn prepare_restore_backup(
        &self,
        folder: impl Into<PathBuf>,
        space: String,
        created_at_unix_ms: u64,
        limits: SnapshotLimits,
    ) -> StorageResult<PreparedRestoreBackup> {
        let folder = folder.into();
        self.run(move |connection| {
            prepare(
                connection,
                &folder,
                space,
                created_at_unix_ms,
                limits,
                |_, _| Ok(()),
            )
        })
        .await
    }
}

fn io<T>(result: std::io::Result<T>) -> StorageResult<T> {
    result.map_err(|_| StorageError::Unavailable)
}

fn directory(path: &Path) -> StorageResult<()> {
    if !io(fs::symlink_metadata(path))?.file_type().is_dir() {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}

pub(crate) fn sync_directory(path: &Path) -> StorageResult<()> {
    #[cfg(unix)]
    io(io(File::open(path))?.sync_all())?;
    #[cfg(not(unix))]
    directory(path)?;
    Ok(())
}

fn prepare(
    connection: &mut Connection,
    folder: &Path,
    space: String,
    date: u64,
    limits: SnapshotLimits,
    checkpoint: impl Fn(Checkpoint, &Path) -> StorageResult<()>,
) -> StorageResult<PreparedRestoreBackup> {
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    schema::check_layout(&tx)?;
    let candidate = capture(&tx, folder, space, date, limits, checkpoint)?;
    tx.rollback()?;
    Ok(candidate)
}

pub(crate) fn capture(
    locked: &Connection,
    folder: &Path,
    space: String,
    date: u64,
    limits: SnapshotLimits,
    checkpoint: impl Fn(Checkpoint, &Path) -> StorageResult<()>,
) -> StorageResult<PreparedRestoreBackup> {
    capture_named(
        locked,
        folder,
        space,
        date,
        limits,
        "zedone-restore-candidate-",
        checkpoint,
    )
}

pub(crate) fn capture_named(
    locked: &Connection,
    folder: &Path,
    space: String,
    date: u64,
    limits: SnapshotLimits,
    prefix: &str,
    checkpoint: impl Fn(Checkpoint, &Path) -> StorageResult<()>,
) -> StorageResult<PreparedRestoreBackup> {
    directory(folder)?;
    let snapshot = capture_export(locked, space, date, limits)?;
    let bytes = snapshot
        .encode(limits)
        .map_err(|_| StorageError::Unavailable)?;
    let directory = io(tempfile::Builder::new().prefix(prefix).tempdir_in(folder))?;
    let mut candidate = PreparedRestoreBackup {
        directory,
        metadata: snapshot.metadata().clone(),
        bytes: bytes.len(),
        digest: Sha256::digest(&bytes).into(),
    };
    checkpoint(Checkpoint::BeforeWrite, candidate.path())?;
    let mut file = io(OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(candidate.path().join(ARCHIVE)))?;
    io(file.write_all(&bytes))?;
    checkpoint(Checkpoint::AfterWrite, candidate.path())?;
    io(file.sync_all())?;
    drop(file);
    sync_directory(candidate.path())?;
    sync_directory(folder)?;
    checkpoint(Checkpoint::BeforeVerify, candidate.path())?;
    candidate.verify(limits)?;
    checkpoint(Checkpoint::AfterVerify, candidate.path())?;
    candidate.directory.disable_cleanup(true);
    Ok(candidate)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Checkpoint {
    BeforeWrite,
    AfterWrite,
    BeforeVerify,
    AfterVerify,
}

#[cfg(test)]
pub(crate) mod tests;
