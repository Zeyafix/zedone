//! Installation-local pending requests; no access or graph changes.
use crate::{StorageError, StorageResult, WriteContext, state_version};
use rusqlite::{OptionalExtension, Row};
use subtle::ConstantTimeEq;
use zedone_core::{
    command::CommandResult,
    pairing::{
        PairingRecord, PairingSnapshot, PairingStatus, PairingStore, PendingVerifier,
        RequestPairing,
    },
};

const SELECT: &str = "SELECT r.request, r.invitation, r.pending_verifier, r.access_verifier,
    r.label, r.created_ms, r.cancelled, i.cancelled
    FROM pairing_requests r JOIN pairing_invitations i ON i.invitation = r.invitation";
fn record(row: &Row<'_>) -> rusqlite::Result<PairingRecord> {
    let request: String = row.get(0)?;
    let invitation: String = row.get(1)?;
    let pending: Vec<u8> = row.get(2)?;
    let access: Vec<u8> = row.get(3)?;
    let label: String = row.get(4)?;
    let created_ms: i64 = row.get(5)?;
    let cancelled: bool = row.get(6)?;
    let invitation_cancelled: bool = row.get(7)?;
    let version = state_version(
        &("pairing.v1", &request),
        &(
            &invitation,
            &pending,
            access,
            &label,
            created_ms,
            cancelled,
            invitation_cancelled,
        ),
    )
    .map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(PairingRecord {
        snapshot: PairingSnapshot {
            request,
            invitation,
            label,
            created_at_unix_ms: created_ms,
            version,
            status: if cancelled || invitation_cancelled {
                PairingStatus::Cancelled
            } else {
                PairingStatus::Pending
            },
        },
        verifier: PendingVerifier(
            pending
                .try_into()
                .map_err(|_| rusqlite::Error::InvalidQuery)?,
        ),
    })
}
impl PairingStore for WriteContext<'_> {
    fn pairing(&mut self, id: &str) -> CommandResult<Option<PairingRecord>> {
        let result = self
            .connection()
            .query_row(&format!("{SELECT} WHERE r.request = ?1"), [id], record)
            .optional()
            .map_err(StorageError::from);
        Ok(self.checked(result)?)
    }
    fn insert_pairing(
        &mut self,
        p: &RequestPairing,
        pending: PendingVerifier,
        now: i64,
    ) -> CommandResult<()> {
        let result = self
            .connection()
            .execute(
                "INSERT INTO pairing_requests VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                rusqlite::params![
                    p.request,
                    p.invitation,
                    &pending.0[..],
                    &p.access_verifier.0[..],
                    p.label,
                    now
                ],
            )
            .map(|_| ())
            .map_err(StorageError::from);
        Ok(self.checked(result)?)
    }
    fn fail_invitation(&mut self, id: &str) -> CommandResult<()> {
        let result = self.connection().execute(
            "UPDATE pairing_invitations SET failures = failures + 1 WHERE invitation = ?1 AND failures < 5", [id],
        ).map(|_| ()).map_err(StorageError::from);
        Ok(self.checked(result)?)
    }
    fn cancel_pairing(&mut self, id: &str) -> CommandResult<()> {
        let result = self
            .connection()
            .execute(
                "UPDATE pairing_requests SET cancelled = 1 WHERE request = ?1",
                [id],
            )
            .map(|_| ())
            .map_err(StorageError::from);
        Ok(self.checked(result)?)
    }
    fn pairings(&mut self, after: Option<&str>) -> CommandResult<Vec<PairingSnapshot>> {
        let result = (|| -> StorageResult<_> {
            Ok(self
                .connection()
                .prepare(&format!(
                    "{SELECT} WHERE r.request > coalesce(?1, '') ORDER BY r.request LIMIT 51"
                ))?
                .query_map([after], |row| Ok(record(row)?.snapshot))?
                .collect::<Result<Vec<_>, _>>()?)
        })();
        Ok(self.checked(result)?)
    }
    fn verifier_matches(&self, left: &[u8; 32], right: &[u8; 32]) -> bool {
        bool::from(left.ct_eq(right))
    }
}

#[cfg(test)]
mod tests;
