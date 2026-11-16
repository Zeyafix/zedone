use rusqlite::OptionalExtension;
use zedone_core::{
    error::{CommandError, ErrorCode},
    replay::{StateVersion, require_current},
};

use crate::{ReadContext, StorageResult, WriteContext, state_version};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpaceSyncControl {
    pub paused: bool,
    pub version: StateVersion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSelection {
    pub selected: bool,
    pub version: StateVersion,
}

impl ReadContext<'_> {
    pub fn space_sync_control(&self, space: &str) -> StorageResult<SpaceSyncControl> {
        let (paused, revision): (bool, i64) = self
            .connection()
            .query_row(
                "SELECT paused, revision FROM sync_spaces WHERE space = ?1",
                [space],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .unwrap_or((false, 0));
        Ok(SpaceSyncControl {
            paused,
            version: state_version(&("sync-space.v1", space), &(paused, revision))?,
        })
    }

    pub fn sync_selection(&self, space: &str, pairing: &str) -> StorageResult<SyncSelection> {
        if !zedone_core::invitations::valid_id(pairing)
            || pairing == "00000000000000000000000000000000"
        {
            return Err(CommandError {
                code: ErrorCode::InvalidRequest,
            }
            .into());
        }
        let (selected, revision): (bool, i64) = self
            .connection()
            .query_row(
                "SELECT selected, revision FROM sync_selections WHERE space = ?1 AND pairing = ?2",
                (space, pairing),
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?
            .unwrap_or((false, 0));
        Ok(SyncSelection {
            selected,
            version: state_version(
                &("sync-selection.v1", space, pairing),
                &(selected, revision),
            )?,
        })
    }
}

impl WriteContext<'_> {
    pub fn set_space_sync_paused(
        &mut self,
        space: &str,
        paused: bool,
        expected: StateVersion,
    ) -> StorageResult<SpaceSyncControl> {
        let result = (|| {
            let current = self.read().space_sync_control(space)?;
            require_current(expected, current.version)?;
            if current.paused != paused {
                self.connection().execute(
                    "INSERT INTO sync_spaces(space, paused, revision) VALUES (?1, ?2, 1)
                     ON CONFLICT(space) DO UPDATE SET paused = excluded.paused, revision = revision + 1",
                    (space, paused),
                )?;
            }
            self.read().space_sync_control(space)
        })();
        self.checked(result)
    }

    pub fn set_sync_selected(
        &mut self,
        space: &str,
        pairing: &str,
        selected: bool,
        expected: StateVersion,
    ) -> StorageResult<SyncSelection> {
        let result = (|| {
            let current = self.read().sync_selection(space, pairing)?;
            require_current(expected, current.version)?;
            if current.selected != selected {
                self.connection().execute(
                    "INSERT INTO sync_selections(space, pairing, selected, revision) VALUES (?1, ?2, ?3, 1)
                     ON CONFLICT(space, pairing) DO UPDATE SET selected = excluded.selected, revision = revision + 1",
                    (space, pairing, selected),
                )?;
                if !selected {
                    self.connection().execute(
                        "DELETE FROM sync_incoming WHERE space = ?1 AND pairing = ?2",
                        (space, pairing),
                    )?;
                }
            }
            self.read().sync_selection(space, pairing)
        })();
        self.checked(result)
    }
}

#[cfg(test)]
mod tests;
