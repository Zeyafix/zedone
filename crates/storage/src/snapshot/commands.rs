use zedone_core::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
    snapshots::{
        ImportedSpace, MAX_ARCHIVE_BYTES, MAX_ARCHIVE_CHANGES, SnapshotStore, SpaceArchive,
    },
};

use super::{SnapshotLimits, export::capture_export};
use crate::{StorageError, StorageResult, WriteContext};

const LIMITS: SnapshotLimits = SnapshotLimits {
    max_bytes: MAX_ARCHIVE_BYTES,
    max_changes: MAX_ARCHIVE_CHANGES,
};

#[cfg(test)]
mod tests;

impl SnapshotStore for WriteContext<'_> {
    fn export_space(
        &mut self,
        space: &str,
        created_at_unix_ms: u64,
    ) -> CommandResult<SpaceArchive> {
        let result = (|| -> StorageResult<_> {
            self.flush_document(space)?;
            let snapshot = capture_export(
                self.connection(),
                space.to_owned(),
                created_at_unix_ms,
                LIMITS,
            )?;
            let bytes = snapshot
                .encode(LIMITS)
                .map_err(|_| StorageError::Unavailable)?;
            Ok(SpaceArchive {
                archive: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            })
        })();
        Ok(self.checked(result)?)
    }

    fn import_space(&mut self, archive: &str, subject: &str) -> CommandResult<ImportedSpace> {
        let result = (|| -> StorageResult<_> {
            if archive.is_empty()
                || archive.len() > 2 * MAX_ARCHIVE_BYTES
                || !archive.len().is_multiple_of(2)
                || !archive
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
            {
                return Err(CommandError {
                    code: ErrorCode::InvalidRequest,
                }
                .into());
            }
            let bytes = (0..archive.len())
                .step_by(2)
                .map(|i| {
                    u8::from_str_radix(&archive[i..i + 2], 16)
                        .map_err(|_| StorageError::Unavailable)
                })
                .collect::<StorageResult<Vec<_>>>()?;
            let imported = self.import_snapshot(&bytes, LIMITS)?;
            self.connection().execute(
                "INSERT INTO access_spaces VALUES (?1, ?2, 1)",
                (subject, &imported.space),
            )?;
            Ok(ImportedSpace {
                space: imported.space,
            })
        })();
        Ok(self.checked(result)?)
    }
}
