use super::*;
use zedone_core::{
    recovery::missing::{MissingRecoveryState, MissingRecoveryStore, RestoreMissingSnapshot},
    replay::require_current,
    snapshots::ImportedSpace,
};

#[cfg(test)]
mod tests;

fn source(archive: &str) -> StorageResult<SpaceSnapshot> {
    if archive.is_empty()
        || archive.len() > MAX_ARCHIVE_BYTES * 2
        || !archive.len().is_multiple_of(2)
        || !archive
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    let bytes = (0..archive.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&archive[i..i + 2], 16).map_err(|_| invalid()))
        .collect::<StorageResult<Vec<_>>>()?;
    let snapshot = SpaceSnapshot::decode(&bytes, LIMITS).map_err(|_| invalid())?;
    if !crate::columns::valid_id(&snapshot.metadata().space) {
        return Err(invalid());
    }
    Ok(snapshot)
}

fn absent(connection: &rusqlite::Connection, space: &str) -> StorageResult<()> {
    let exists: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE space = ?1)",
        [space],
        |r| r.get(0),
    )?;
    if exists {
        return Err(CommandError {
            code: ErrorCode::StaleState,
        }
        .into());
    }
    Ok(())
}

fn state(
    connection: &rusqlite::Connection,
    snapshot: &SpaceSnapshot,
    graph: &automerge::Automerge,
) -> StorageResult<MissingRecoveryState> {
    let space = &snapshot.metadata().space;
    absent(connection, space)?;
    let names = crate::spaces::snapshot(graph, space, true)?.names;
    Ok(MissingRecoveryState {
        space: space.clone(),
        names,
        version: state_version(
            &("space-recovery-missing.v1", space),
            &Sha256::digest(snapshot.encode(LIMITS).map_err(|_| invalid())?).as_slice(),
        )?,
    })
}

impl MissingRecoveryStore for WriteContext<'_> {
    fn missing_recovery_state(&mut self, archive: &str) -> CommandResult<MissingRecoveryState> {
        let result = (|| {
            let snapshot = source(archive)?;
            let graph = graph::source(&snapshot)?;
            state(self.connection(), &snapshot, &graph)
        })();
        Ok(self.checked(result)?)
    }

    fn restore_missing_snapshot(
        &mut self,
        request: &RestoreMissingSnapshot,
        subject: &str,
    ) -> CommandResult<ImportedSpace> {
        let result = (|| {
            let snapshot = source(&request.archive)?;
            if snapshot.metadata().space != request.space {
                return Err(invalid());
            }
            let graph = graph::source(&snapshot)?;
            require_current(
                request.expected,
                state(self.connection(), &snapshot, &graph)?.version,
            )?;
            let changes = graph::plan(&graph, &snapshot)?;
            self.create_document(&request.space)?;
            self.apply_changes(&request.space, changes)?;
            self.connection().execute(
                "INSERT INTO access_spaces VALUES (?1, ?2, 1)",
                (subject, &request.space),
            )?;
            let policy = self.read().space_sync_control(&request.space)?;
            self.set_space_sync_paused(&request.space, true, policy.version)?;
            Ok(ImportedSpace {
                space: request.space.clone(),
            })
        })();
        Ok(self.checked(result)?)
    }
}
