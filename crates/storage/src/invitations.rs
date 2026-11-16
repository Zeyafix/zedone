use rusqlite::{OptionalExtension, Row};
use zedone_core::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
    invitations::{InvitationRecord, InvitationStore, InvitationVerifier},
};

use crate::{StorageError, StorageResult, WriteContext, state_version};

#[cfg(test)]
mod tests;

fn record(row: &Row<'_>) -> rusqlite::Result<InvitationRecord> {
    let invitation: String = row.get(0)?;
    let verifier: Vec<u8> = row.get(1)?;
    let created_ms: i64 = row.get(2)?;
    let expires_ms: i64 = row.get(3)?;
    let cancelled: bool = row.get(4)?;
    let failures: u8 = row.get(5)?;
    let used: bool = row.get(6)?;
    let version = state_version(
        &("invitation.v1", &invitation),
        &(&verifier, created_ms, expires_ms, cancelled, failures, used),
    )
    .map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(InvitationRecord {
        invitation,
        created_ms,
        expires_ms,
        cancelled,
        verifier: InvitationVerifier(
            verifier
                .try_into()
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
        ),
        failures,
        used,
        version,
    })
}

impl InvitationStore for WriteContext<'_> {
    fn invitation(&mut self, id: &str) -> CommandResult<Option<InvitationRecord>> {
        let result = self
            .connection()
            .query_row(
                "SELECT invitation, verifier, created_ms, expires_ms, cancelled, failures,
                    EXISTS(SELECT 1 FROM pairing_requests r WHERE r.invitation = pairing_invitations.invitation)
             FROM pairing_invitations WHERE invitation = ?1",
                [id],
                record,
            )
            .optional()
            .map_err(StorageError::from);
        Ok(self.checked(result)?)
    }

    fn insert_invitation(
        &mut self,
        id: &str,
        verifier: InvitationVerifier,
        created_ms: i64,
        expires_ms: i64,
    ) -> CommandResult<()> {
        let result = (|| -> StorageResult<()> {
            let exists: bool = self.connection().query_row(
                "SELECT EXISTS(SELECT 1 FROM pairing_invitations WHERE invitation = ?1 OR verifier = ?2)",
                rusqlite::params![id, &verifier.0[..]], |row| row.get(0),
            )?;
            if exists {
                return Err(CommandError {
                    code: ErrorCode::InvalidRequest,
                }
                .into());
            }
            self.connection().execute(
                "INSERT INTO pairing_invitations VALUES (?1, ?2, ?3, ?4, 0, 0)",
                rusqlite::params![id, &verifier.0[..], created_ms, expires_ms],
            )?;
            Ok(())
        })();
        Ok(self.checked(result)?)
    }

    fn cancel_invitation(&mut self, id: &str) -> CommandResult<()> {
        let result = self
            .connection()
            .execute(
                "UPDATE pairing_invitations SET cancelled = 1 WHERE invitation = ?1",
                [id],
            )
            .map(|_| ())
            .map_err(StorageError::from);
        Ok(self.checked(result)?)
    }

    fn invitations(&mut self, after: Option<&str>) -> CommandResult<Vec<InvitationRecord>> {
        let result = (|| -> StorageResult<_> {
            Ok(self
                .connection()
                .prepare(
                    "SELECT invitation, verifier, created_ms, expires_ms, cancelled, failures,
                        EXISTS(SELECT 1 FROM pairing_requests r WHERE r.invitation = pairing_invitations.invitation)
                 FROM pairing_invitations WHERE invitation > coalesce(?1, '')
                 ORDER BY invitation LIMIT 51",
                )?
                .query_map([after], record)?
                .collect::<Result<Vec<_>, _>>()?)
        })();
        Ok(self.checked(result)?)
    }
}
