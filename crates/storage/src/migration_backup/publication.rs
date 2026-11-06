//! A SQLite commit witness is authoritative; the file index is repairable.
//! Trusted startup infrastructure only; no authenticated restore command.

use rusqlite::{OptionalExtension, Transaction};
use tempfile::NamedTempFile;

use super::*;

// Longer than a wire CommandId; never a user command receipt. Kept in the
// existing atomic result store, not in a space graph or a second live database.
const KEY: &[u8] = b"\0zedone/internal/migration-backup/v1";
const REQUEST: &[u8] = b"migration-backup-commit-v1";
const INDEX: &str = "migration-current.json";
const RECORD_LIMIT: u64 = 4096;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackupRef {
    name: String,
    manifest: FileDigest,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CommitRecord {
    version: u32,
    current: BackupRef,
    previous: Option<BackupRef>,
}

/// Confirmed commit and publication. Cleanup failure does not undo the commit.
#[derive(Debug)]
pub struct PublishedBackup {
    pub path: PathBuf,
    pub cleanup_pending: bool,
}

impl Storage {
    /// Capture and run one trusted transaction under an uninterrupted writer
    /// lock. `apply` must not manage transactions or publish external effects.
    /// `validate` checks the target result both before and after commit.
    /// An error after a commit attempt is OutcomeUnknown: recover, never rerun
    /// the operation on that basis. This does not implement a migration chain.
    pub async fn with_migration_backup(
        &self,
        folder: impl Into<PathBuf>,
        date: u64,
        limits: BackupLimits,
        apply: impl FnOnce(&Transaction<'_>) -> StorageResult<()> + Send + 'static,
        validate: impl Fn(&Connection) -> StorageResult<()> + Send + 'static,
    ) -> StorageResult<PublishedBackup> {
        let folder = folder.into();
        self.run(move |connection| {
            execute(connection, &folder, date, limits, apply, validate, |_| {
                Ok(())
            })
        })
        .await
    }

    /// Reconcile a committed attempt without executing its mutation again.
    /// A missing/corrupt candidate or failed target validation stops recovery;
    /// neither an old backup nor an orphan is promoted as a fallback.
    pub async fn recover_migration_backup(
        &self,
        folder: impl Into<PathBuf>,
        limits: BackupLimits,
        validate: impl Fn(&Connection) -> StorageResult<()> + Send + 'static,
    ) -> StorageResult<Option<PublishedBackup>> {
        let folder = folder.into();
        self.run(move |connection| {
            let _lock = lock(&folder)?;
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let result = reconcile(&tx, &folder, limits, &validate, &|_| Ok(()))?;
            tx.rollback()?;
            Ok(result)
        })
        .await
    }

    /// Explicitly delete an abandoned candidate, never a committed current or
    /// previous reference. Caller supplies the basename from a private folder.
    pub async fn discard_migration_candidate(
        &self,
        folder: impl Into<PathBuf>,
        name: String,
    ) -> StorageResult<()> {
        let folder = folder.into();
        self.run(move |connection| {
            let _lock = lock(&folder)?;
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            let record = committed(&tx)?;
            let indexed = index(&folder)?;
            if indexed.as_ref().is_some_and(|r| r.name == name)
                || record.as_ref().is_some_and(|r| {
                    r.current.name == name || r.previous.as_ref().is_some_and(|r| r.name == name)
                })
            {
                return Err(StorageError::Unavailable);
            }
            remove_candidate(&folder, &name)?;
            tx.rollback()?;
            Ok(())
        })
        .await
    }
}

fn valid_name(name: &str) -> bool {
    name.strip_prefix("zedone-migration-candidate-")
        .is_some_and(|suffix| {
            !suffix.is_empty()
                && suffix.len() <= 64
                && suffix.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}

fn directory(path: &Path) -> StorageResult<()> {
    if !io(fs::symlink_metadata(path))?.file_type().is_dir() {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}

struct FolderLock(File);

impl Drop for FolderLock {
    fn drop(&mut self) {
        // Closing alone retains the lock while a duplicate/inherited descriptor
        // survives, e.g. during a concurrent fork before exec. End ownership
        // explicitly; closing the file remains the fallback if unlock fails.
        let _ = self.0.unlock();
    }
}

fn lock(folder: &Path) -> StorageResult<FolderLock> {
    directory(folder)?;
    let file = io(OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(folder.join("migration.lock")))?;
    file.try_lock().map_err(|_| StorageError::Unavailable)?;
    Ok(FolderLock(file))
}

// Unix can fsync directory entries. std/tempfile do not provide a Windows
// directory durability barrier: Windows coverage is process termination, not
// power loss. All file contents are synced on both platforms. Never silently
// fall back to another folder or treat a missing committed candidate as success.
fn sync_directory(path: &Path) -> StorageResult<()> {
    #[cfg(unix)]
    io(io(File::open(path))?.sync_all())?;
    #[cfg(not(unix))]
    directory(path)?;
    Ok(())
}

fn decode<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> StorageResult<T> {
    serde_json::from_slice(bytes).map_err(|_| StorageError::InvalidDatabase)
}

fn encode<T: Serialize>(value: &T) -> StorageResult<Vec<u8>> {
    serde_json::to_vec(value).map_err(|_| StorageError::Unavailable)
}

fn checked(reference: &BackupRef) -> StorageResult<()> {
    if !valid_name(&reference.name) {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}

pub(crate) fn has_commit_witness(connection: &Connection) -> StorageResult<bool> {
    Ok(committed(connection)?.is_some())
}

fn committed(connection: &Connection) -> StorageResult<Option<CommitRecord>> {
    let row: Option<(Vec<u8>, Vec<u8>)> = connection
        .query_row(
            "SELECT substr(request, 1, ?2), substr(result, 1, ?2)
             FROM command_results WHERE key = ?1",
            rusqlite::params![KEY, (RECORD_LIMIT + 1) as i64],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(request, bytes)| {
        if request != REQUEST || bytes.len() as u64 > RECORD_LIMIT {
            return Err(StorageError::InvalidDatabase);
        }
        let record: CommitRecord = decode(&bytes)?;
        checked(&record.current)?;
        if let Some(previous) = &record.previous {
            checked(previous)?;
            if previous.name == record.current.name {
                return Err(StorageError::InvalidDatabase);
            }
        }
        if record.version != 1 {
            return Err(StorageError::InvalidDatabase);
        }
        Ok(record)
    })
    .transpose()
}

fn index(folder: &Path) -> StorageResult<Option<BackupRef>> {
    let path = folder.join(INDEX);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        result => {
            if !io(result)?.file_type().is_file() {
                return Err(StorageError::InvalidDatabase);
            }
        }
    }
    let reference = decode(&read_bounded(&path, RECORD_LIMIT)?)?;
    checked(&reference)?;
    Ok(Some(reference))
}

fn verify_reference(
    folder: &Path,
    reference: &BackupRef,
    limits: BackupLimits,
    registry: schema::Registry,
) -> StorageResult<()> {
    checked(reference)?;
    let path = folder.join(&reference.name);
    directory(&path)?;
    if digest(&path.join("manifest.json"), manifest_limit(limits))? != reference.manifest {
        return Err(StorageError::InvalidDatabase);
    }
    verify_with(&path, limits, registry)
}

fn publish(folder: &Path, reference: &BackupRef) -> StorageResult<()> {
    let mut temporary = io(NamedTempFile::new_in(folder))?;
    io(temporary.write_all(&encode(reference)?))?;
    io(temporary.as_file().sync_all())?;
    temporary
        .persist(folder.join(INDEX))
        .map_err(|_| StorageError::Unavailable)?;
    sync_directory(folder)
}

fn remove_candidate(folder: &Path, name: &str) -> StorageResult<()> {
    if !valid_name(name) {
        return Err(StorageError::InvalidDatabase);
    }
    let path = folder.join(name);
    match fs::symlink_metadata(&path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        result => {
            if !io(result)?.file_type().is_dir() {
                return Err(StorageError::InvalidDatabase);
            }
        }
    }
    io(fs::remove_dir_all(path))?;
    sync_directory(folder)
}

fn reconcile(
    connection: &Connection,
    folder: &Path,
    limits: BackupLimits,
    validate: &impl Fn(&Connection) -> StorageResult<()>,
    checkpoint: &impl Fn(Point) -> StorageResult<()>,
) -> StorageResult<Option<PublishedBackup>> {
    reconcile_with(
        connection,
        folder,
        limits,
        validate,
        checkpoint,
        schema::REGISTRY,
    )
}

/// Startup reconciles or migrates while retaining the folder lock. It rechecks
/// the version under the writer lock, so competing starts cannot rotate a backup
/// by executing an already completed migration a second time.
pub(crate) fn migrate_startup(
    connection: &mut Connection,
    options: &crate::startup::MigrationOptions,
    registry: schema::Registry,
    checkpoint: impl Fn(Point) -> StorageResult<()>,
) -> StorageResult<()> {
    execute_with(
        connection,
        &options.folder,
        options.created_at_unix_ms,
        options.limits,
        registry,
        true,
        |tx| registry.migrate(tx),
        |c| registry.target(c),
        checkpoint,
    )?;
    Ok(())
}

fn reconcile_with(
    connection: &Connection,
    folder: &Path,
    limits: BackupLimits,
    validate: &impl Fn(&Connection) -> StorageResult<()>,
    checkpoint: &impl Fn(Point) -> StorageResult<()>,
    registry: schema::Registry,
) -> StorageResult<Option<PublishedBackup>> {
    let indexed = index(folder)?;
    let Some(record) = committed(connection)? else {
        return if indexed.is_none() {
            Ok(None)
        } else {
            Err(StorageError::InvalidDatabase)
        };
    };
    if indexed.is_some() && indexed.as_ref() != Some(&record.current) && indexed != record.previous
    {
        return Err(StorageError::InvalidDatabase);
    }
    validate(connection)?;
    verify_reference(folder, &record.current, limits, registry)?;
    sync_directory(&folder.join(&record.current.name))?;
    sync_directory(folder)?;
    checkpoint(Point::BeforePublish)?;
    publish(folder, &record.current)?;
    checkpoint(Point::AfterPublish)?;
    verify_reference(folder, &record.current, limits, registry)?;
    if index(folder)?.as_ref() != Some(&record.current) {
        return Err(StorageError::InvalidDatabase);
    }
    let cleanup_pending = if let Some(previous) = &record.previous {
        checkpoint(Point::BeforeCleanup)
            .and_then(|_| remove_candidate(folder, &previous.name))
            .is_err()
    } else {
        false
    };
    Ok(Some(PublishedBackup {
        path: folder.join(record.current.name),
        cleanup_pending,
    }))
}

fn execute(
    connection: &mut Connection,
    folder: &Path,
    date: u64,
    limits: BackupLimits,
    apply: impl FnOnce(&Transaction<'_>) -> StorageResult<()>,
    validate: impl Fn(&Connection) -> StorageResult<()>,
    checkpoint: impl Fn(Point) -> StorageResult<()>,
) -> StorageResult<PublishedBackup> {
    execute_with(
        connection,
        folder,
        date,
        limits,
        schema::REGISTRY,
        false,
        apply,
        validate,
        checkpoint,
    )?
    .ok_or(StorageError::Unavailable)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn execute_with(
    connection: &mut Connection,
    folder: &Path,
    date: u64,
    limits: BackupLimits,
    registry: schema::Registry,
    migration: bool,
    apply: impl FnOnce(&Transaction<'_>) -> StorageResult<()>,
    validate: impl Fn(&Connection) -> StorageResult<()>,
    checkpoint: impl Fn(Point) -> StorageResult<()>,
) -> StorageResult<Option<PublishedBackup>> {
    let _lock = lock(folder)?;
    let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
    registry.validate(&tx)?;
    if let Some(record) = committed(&tx)?
        && !migration
        && index(folder)?.as_ref() != Some(&record.current)
    {
        return Err(StorageError::OutcomeUnknown);
    }
    // An unfinished publication must be reconciled before any new attempt.
    let prior = reconcile_with(
        &tx,
        folder,
        limits,
        &|c| validate_database_with(c, registry),
        &|_| Ok(()),
        registry,
    )?;
    if migration && registry.known(&tx)?.version == registry.current().version {
        registry.target(&tx)?;
        tx.rollback()?;
        return Ok(prior);
    }
    if prior.is_some_and(|result| result.cleanup_pending) {
        return Err(StorageError::Unavailable);
    }
    let previous = index(folder)?;
    let candidate = capture_with(&tx, folder, date, limits, registry, |point| {
        if point == Checkpoint::AfterCopy {
            checkpoint(Point::CopyWritten)?;
        }
        Ok(())
    })?;
    sync_directory(candidate.path())?;
    sync_directory(folder)?;
    let current = BackupRef {
        name: candidate
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(StorageError::Unavailable)?
            .to_owned(),
        manifest: digest(
            &candidate.path().join("manifest.json"),
            manifest_limit(limits),
        )?,
    };
    checked(&current)?;
    checkpoint(Point::AfterCapture)?;
    apply(&tx)?;
    validate(&tx)?;
    let record = CommitRecord {
        version: 1,
        current,
        previous,
    };
    tx.execute(
        "INSERT INTO command_results(key, request, result) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET request = excluded.request, result = excluded.result",
        rusqlite::params![KEY, REQUEST, encode(&record)?],
    )?;
    validate(&tx)?;
    verify_reference(folder, &record.current, limits, registry)?;
    checkpoint(Point::BeforeCommit)?;
    tx.commit().map_err(|_| StorageError::OutcomeUnknown)?;
    // From here a failure must not be presented as a rollback of the operation.
    (|| {
        checkpoint(Point::AfterCommit)?;
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        // Another process must not supersede the witness during publication.
        if committed(&tx)?.as_ref().map(|r| &r.current) != Some(&record.current) {
            return Err(StorageError::InvalidDatabase);
        }
        let result = reconcile_with(&tx, folder, limits, &validate, &checkpoint, registry)?
            .ok_or(StorageError::InvalidDatabase)?;
        tx.rollback()?;
        Ok(Some(result))
    })()
    .map_err(|_| StorageError::OutcomeUnknown)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Point {
    CopyWritten,
    AfterCapture,
    BeforeCommit,
    AfterCommit,
    BeforePublish,
    AfterPublish,
    BeforeCleanup,
}

#[cfg(test)]
mod tests;
