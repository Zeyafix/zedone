use crate::restore_backup::{ARCHIVE, PreparedRestoreBackup};
use crate::{
    Storage, StorageError, StorageResult, WriteContext, restore_backup,
    snapshot::{SnapshotLimits, SpaceSnapshot},
    state_version,
};
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, File},
    io::Read,
    path::PathBuf,
};
use zedone_core::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
    recovery::{RecoveryState, RecoveryStore, RestoreSnapshot, RestoredSnapshot, SafetyBackup},
    snapshots::{MAX_ARCHIVE_BYTES, MAX_ARCHIVE_CHANGES, SpaceArchive},
};
mod graph;
#[cfg(test)]
mod tests;
const LIMITS: SnapshotLimits = SnapshotLimits {
    max_bytes: MAX_ARCHIVE_BYTES,
    max_changes: MAX_ARCHIVE_CHANGES,
};
const REQUEST: &[u8] = b"space-restore-backup.v1";
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    name: String,
    space: String,
    date: u64,
    bytes: usize,
    digest: [u8; 32],
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Witness {
    current: Reference,
    previous: Option<Reference>,
}
#[derive(Serialize)]
struct EditingProjection {
    card: String,
    subject: String,
    session: String,
    pending_id: Option<String>,
    title: Option<String>,
    description: Option<String>,
    changed_ms: Option<i64>,
}
fn invalid() -> StorageError {
    CommandError {
        code: ErrorCode::InvalidRequest,
    }
    .into()
}
fn key(space: &str) -> Vec<u8> {
    format!("\0zedone/internal/restore-backup/v1/{space}").into_bytes()
}
fn folder(connection: &rusqlite::Connection) -> StorageResult<PathBuf> {
    let path = connection.path().ok_or(StorageError::Unavailable)?;
    let database = fs::canonicalize(path).map_err(|_| StorageError::Unavailable)?;
    let name = database
        .file_name()
        .ok_or(StorageError::Unavailable)?
        .to_string_lossy();
    Ok(database.with_file_name(format!("{name}.backups")))
}
fn checked(reference: &Reference, space: &str) -> StorageResult<()> {
    if reference.space != space
        || reference.bytes > MAX_ARCHIVE_BYTES
        || reference.bytes == 0
        || !reference
            .name
            .strip_prefix("zedone-restore-candidate-")
            .is_some_and(|name| {
                !name.is_empty()
                    && name.len() <= 64
                    && name.bytes().all(|b| b.is_ascii_alphanumeric())
            })
    {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}
fn witness(connection: &rusqlite::Connection, space: &str) -> StorageResult<Option<Witness>> {
    let row: Option<(Vec<u8>, Vec<u8>)> = connection.query_row("SELECT substr(request,1,4097), substr(result,1,4097) FROM command_results WHERE key = ?1", [key(space)], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
    row.map(|(request, result)| {
        if request != REQUEST || result.len() > 4096 {
            return Err(StorageError::InvalidDatabase);
        }
        let value: Witness =
            serde_json::from_slice(&result).map_err(|_| StorageError::InvalidDatabase)?;
        checked(&value.current, space)?;
        if let Some(previous) = &value.previous {
            checked(previous, space)?;
            if previous.name == value.current.name {
                return Err(StorageError::InvalidDatabase);
            }
        }
        Ok(value)
    })
    .transpose()
}
fn read_reference(
    connection: &rusqlite::Connection,
    reference: &Reference,
) -> StorageResult<Vec<u8>> {
    checked(reference, &reference.space)?;
    if !fs::symlink_metadata(folder(connection)?)
        .map_err(|_| StorageError::Unavailable)?
        .file_type()
        .is_dir()
    {
        return Err(StorageError::InvalidDatabase);
    }
    let directory = folder(connection)?.join(&reference.name);
    if !fs::symlink_metadata(&directory)
        .map_err(|_| StorageError::Unavailable)?
        .file_type()
        .is_dir()
    {
        return Err(StorageError::InvalidDatabase);
    }
    let path = directory.join(ARCHIVE);
    if !fs::symlink_metadata(&path)
        .map_err(|_| StorageError::Unavailable)?
        .file_type()
        .is_file()
    {
        return Err(StorageError::InvalidDatabase);
    }
    let mut bytes = Vec::new();
    File::open(path)
        .map_err(|_| StorageError::Unavailable)?
        .take(reference.bytes as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| StorageError::Unavailable)?;
    if bytes.len() != reference.bytes || Sha256::digest(&bytes).as_slice() != reference.digest {
        return Err(StorageError::InvalidDatabase);
    }
    let snapshot =
        SpaceSnapshot::decode(&bytes, LIMITS).map_err(|_| StorageError::InvalidDatabase)?;
    if snapshot.metadata().space != reference.space
        || snapshot.metadata().created_at_unix_ms != reference.date
    {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(bytes)
}
fn save_witness(
    connection: &rusqlite::Connection,
    space: &str,
    value: &Witness,
) -> StorageResult<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| StorageError::Unavailable)?;
    connection.execute("INSERT INTO command_results VALUES (?1, ?2, ?3) ON CONFLICT(key) DO UPDATE SET request=excluded.request, result=excluded.result", (key(space), REQUEST, bytes))?;
    Ok(())
}
fn cleanup(connection: &rusqlite::Connection, space: &str) -> StorageResult<()> {
    if let Some(mut value) = witness(connection, space)?
        && let Some(previous) = value.previous.as_ref()
    {
        read_reference(connection, &value.current)?;
        let path = folder(connection)?.join(&previous.name);
        match fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Ok(metadata) if metadata.file_type().is_dir() => {
                match fs::remove_file(path.join(restore_backup::ARCHIVE)) {
                    Ok(()) => {}
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                    _ => return Err(StorageError::Unavailable),
                }
                fs::remove_dir(&path).map_err(|_| StorageError::Unavailable)?;
            }
            _ => return Err(StorageError::Unavailable),
        }
        restore_backup::sync_directory(&folder(connection)?)?;
        value.previous = None;
        save_witness(connection, space, &value)?;
    }
    Ok(())
}
impl Storage {
    pub async fn cleanup_restore_backup(&self, space: String) -> StorageResult<()> {
        self.transact(move |ctx| cleanup(ctx.connection(), &space))
            .await
    }
}
impl RecoveryStore for WriteContext<'_> {
    fn set_recovery_pause(
        &mut self,
        request: &zedone_core::recovery::RecoveryPause,
    ) -> CommandResult<RecoveryState> {
        let result = (|| {
            self.set_space_sync_paused(&request.space, request.paused, request.expected)?;
            Ok(self.recovery_state(&request.space)?)
        })();
        Ok(self.checked(result)?)
    }
    fn recovery_state(&mut self, space: &str) -> CommandResult<RecoveryState> {
        let result = (|| {
            self.flush_document(space)?;
            let graph = self.read().document(space)?;
            crate::spaces::snapshot(graph, space, true)?;
            let mut hashes: Vec<Vec<u8>> = self
                .connection()
                .prepare("SELECT hash FROM changes WHERE space = ?1 ORDER BY hash")?
                .query_map([space], |r| r.get(0))?
                .collect::<Result<_, _>>()?;
            hashes.sort();
            let pending: Vec<EditingProjection> = self.connection().prepare("SELECT card, subject, session, pending_id, title, description, changed_ms FROM card_editing WHERE space = ?1 ORDER BY card")?.query_map([space], |r| Ok(EditingProjection { card: r.get(0)?, subject: r.get(1)?, session: r.get(2)?, pending_id: r.get(3)?, title: r.get(4)?, description: r.get(5)?, changed_ms: r.get(6)? }))?.collect::<Result<_, _>>()?;
            let policy = self.read().space_sync_control(space)?;
            let saved = witness(self.connection(), space)?;
            let backup = saved.as_ref().map(|w| SafetyBackup {
                id: w.current.name.clone(),
                created_at_unix_ms: w.current.date,
            });
            Ok(RecoveryState {
                space: space.into(),
                version: state_version(
                    &("space-recovery.v1", space),
                    &(
                        hashes,
                        pending,
                        policy.version,
                        &backup.as_ref().map(|b| &b.id),
                    ),
                )?,
                paused: policy.paused,
                pause_version: policy.version,
                backup,
                cleanup_pending: saved.is_some_and(|w| w.previous.is_some()),
            })
        })();
        Ok(self.checked(result)?)
    }
    fn restore_snapshot(
        &mut self,
        request: &RestoreSnapshot,
        date: u64,
    ) -> CommandResult<RestoredSnapshot> {
        let result = (|| {
            if request.archive.is_empty()
                || request.archive.len() > MAX_ARCHIVE_BYTES * 2
                || !request.archive.len().is_multiple_of(2)
                || !request
                    .archive
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(invalid());
            }
            let bytes = (0..request.archive.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&request.archive[i..i + 2], 16).map_err(|_| invalid()))
                .collect::<StorageResult<Vec<_>>>()?;
            let snapshot = SpaceSnapshot::decode(&bytes, LIMITS).map_err(|_| invalid())?;
            if snapshot.metadata().space != request.space {
                return Err(invalid());
            }
            self.flush_document(&request.space)?;
            let changes = graph::plan(self.read().document(&request.space)?, &snapshot)?;
            cleanup(self.connection(), &request.space)?;
            let previous = witness(self.connection(), &request.space)?.map(|w| w.current);
            let folder = folder(self.connection())?;
            if !folder.exists() && previous.is_none() {
                fs::create_dir(&folder).map_err(|_| StorageError::Unavailable)?;
            }
            let candidate: PreparedRestoreBackup = restore_backup::capture(
                self.connection(),
                &folder,
                request.space.clone(),
                date,
                LIMITS,
                |_, _| Ok(()),
            )?;
            let reference = Reference {
                name: candidate
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or(StorageError::Unavailable)?
                    .into(),
                space: request.space.clone(),
                date,
                bytes: candidate.bytes,
                digest: candidate.digest,
            };
            self.apply_changes(&request.space, changes)?;
            self.connection().execute(
                "DELETE FROM card_editing WHERE space = ?1",
                [&request.space],
            )?;
            let policy = self.read().space_sync_control(&request.space)?;
            self.set_space_sync_paused(&request.space, true, policy.version)?;
            save_witness(
                self.connection(),
                &request.space,
                &Witness {
                    current: reference.clone(),
                    previous,
                },
            )?;
            read_reference(self.connection(), &reference)?;
            Ok(RestoredSnapshot {
                space: request.space.clone(),
                safety: reference.name,
            })
        })();
        Ok(self.checked(result)?)
    }
    fn read_safety_backup(&mut self, space: &str, safety: &str) -> CommandResult<SpaceArchive> {
        let result = (|| {
            let saved = witness(self.connection(), space)?.ok_or(CommandError {
                code: ErrorCode::NotFound,
            })?;
            if saved.current.name != safety {
                return Err(CommandError {
                    code: ErrorCode::StaleState,
                }
                .into());
            }
            let bytes = read_reference(self.connection(), &saved.current)?;
            Ok(SpaceArchive {
                archive: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            })
        })();
        Ok(self.checked(result)?)
    }
}
