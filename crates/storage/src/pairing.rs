use crate::{StorageError, StorageResult, WriteContext, state_version};
use rusqlite::{OptionalExtension, Row};
use subtle::ConstantTimeEq;
use zedone_core::{
    access_commands::{AccessStore, KeyVerifier},
    command::CommandResult,
    pairing::{
        PairingRecord, PairingSnapshot, PairingStatus, PairingStore, PendingVerifier,
        RequestPairing,
    },
};

const SELECT: &str = "SELECT r.request, r.invitation, r.pending_verifier, r.access_verifier,
    r.label, r.created_ms, r.cancelled, i.cancelled, r.decision, k.active
    FROM pairing_requests r JOIN pairing_invitations i ON i.invitation = r.invitation
    LEFT JOIN access_keys k ON k.subject = r.subject AND k.verifier = r.access_verifier";
fn record(row: &Row<'_>) -> rusqlite::Result<PairingRecord> {
    let request: String = row.get(0)?;
    let invitation: String = row.get(1)?;
    let pending: Vec<u8> = row.get(2)?;
    let access: Vec<u8> = row.get(3)?;
    let label: String = row.get(4)?;
    let created_ms: i64 = row.get(5)?;
    let cancelled: bool = row.get(6)?;
    let invitation_cancelled: bool = row.get(7)?;
    let decision: u8 = row.get(8)?;
    let active: Option<bool> = row.get(9)?;
    if decision == 1 && active.is_none() {
        return Err(rusqlite::Error::InvalidQuery);
    }
    let version = state_version(
        &("pairing.v1", &request),
        &(
            &invitation,
            &pending,
            &access,
            &label,
            created_ms,
            cancelled,
            invitation_cancelled,
            decision,
            active,
        ),
    )
    .map_err(|_| rusqlite::Error::InvalidQuery)?;
    Ok(PairingRecord {
        snapshot: PairingSnapshot {
            request: request.clone(),
            invitation,
            label,
            created_at_unix_ms: created_ms,
            version,
            subject: if decision == 1 {
                Some(request.clone())
            } else {
                None
            },
            status: if decision == 1 {
                if active == Some(true) {
                    PairingStatus::Confirmed
                } else {
                    PairingStatus::Unlinked
                }
            } else if decision == 2 {
                PairingStatus::Rejected
            } else if cancelled || invitation_cancelled {
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
        access_verifier: KeyVerifier(
            access
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
        let mut record = self.checked(result)?;
        if let Some(record) = &mut record {
            self.with_access(record)?;
        }
        Ok(record)
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
                "INSERT INTO pairing_requests VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0, 0, NULL)",
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
                .query_map([after], record)?
                .collect::<Result<Vec<_>, _>>()?)
        })();
        let mut records = self.checked(result)?;
        for record in &mut records {
            self.with_access(record)?;
        }
        Ok(records.into_iter().map(|r| r.snapshot).collect())
    }
    fn decide_pairing(&mut self, id: &str, confirmed: bool) -> CommandResult<()> {
        let result = self.connection().execute(
            "UPDATE pairing_requests SET decision = ?2, subject = CASE WHEN ?2 = 1 THEN request ELSE NULL END WHERE request = ?1 AND decision = 0 AND cancelled = 0",
            rusqlite::params![id, if confirmed { 1 } else { 2 }],
        ).and_then(|n| if n == 1 { Ok(()) } else { Err(rusqlite::Error::InvalidQuery) }).map_err(StorageError::from);
        Ok(self.checked(result)?)
    }
    fn verifier_matches(&self, left: &[u8; 32], right: &[u8; 32]) -> bool {
        bool::from(left.ct_eq(right))
    }
}

impl WriteContext<'_> {
    fn with_access(&mut self, record: &mut PairingRecord) -> CommandResult<()> {
        if matches!(
            record.snapshot.status,
            PairingStatus::Confirmed | PairingStatus::Unlinked
        ) {
            let access = self.access_snapshot(&record.snapshot.request)?.ok_or(
                zedone_core::error::CommandError {
                    code: zedone_core::error::ErrorCode::Unavailable,
                },
            )?;
            record.snapshot.version =
                state_version(&("pairing-access.v1", record.snapshot.version), &access)?;
        }
        Ok(())
    }
}

pub(crate) fn validate(connection: &rusqlite::Connection) -> StorageResult<()> {
    let invalid: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM pairing_requests r WHERE decision = 1 AND NOT EXISTS(
            SELECT 1 FROM access_keys k WHERE k.subject = r.subject AND k.verifier = r.access_verifier))",
        [], |r| r.get(0)
    )?;
    if invalid {
        Err(StorageError::InvalidDatabase)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests;
