//! Trusted installation access, serialized with domain work on the SQL worker.
//! Not a network API. Transports cannot call bootstrap or supply a Clock/handler.

use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use rusqlite::OptionalExtension;
use sha2::{Digest, Sha256};
use zedone_core::{
    access::{
        AccessTransaction, Clock, Credential, Grant, GrantScope, RequiredAccess, ResolvedAccess,
        SpacePermission, with_authorized,
    },
    command::CommandResult,
    error::{CommandError, ErrorCode},
    replay::{RecordedCommand, ReplayTransaction},
};

use crate::{StorageError, StorageResult, WriteContext};

#[cfg(test)]
mod tests;

/// Generated before starting an issuance/replacement transaction. Keep it until
/// the outcome is known; never log, serialize in receipts, or regenerate on retry.
/// No Debug/Serialize/Clone implementations by design.
pub struct AccessKey([u8; 32]);

impl AccessKey {
    pub fn generate() -> StorageResult<Self> {
        let mut bytes = [0; 32];
        getrandom::fill(&mut bytes).map_err(|_| StorageError::Unavailable)?;
        Ok(Self(bytes))
    }

    pub fn credential(&self) -> Credential<'_> {
        Credential(&self.0)
    }

    fn verifier(&self) -> [u8; 32] {
        verifier(&self.0)
    }
}

/// A fresh stable subject and independent key. This object grants nothing until
/// an explicit bootstrap/authorized issuance transaction has committed.
pub struct PendingAccess {
    subject: String,
    key: AccessKey,
}

impl PendingAccess {
    pub fn generate() -> StorageResult<Self> {
        let mut id = [0; 16];
        getrandom::fill(&mut id).map_err(|_| StorageError::Unavailable)?;
        Ok(Self {
            subject: id.iter().map(|byte| format!("{byte:02x}")).collect(),
            key: AccessKey::generate()?,
        })
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn credential(&self) -> Credential<'_> {
        self.key.credential()
    }
}

fn verifier(secret: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"zedone/access-key/v1\0");
    hash.update(secret);
    hash.finalize().into()
}

fn error(code: ErrorCode) -> StorageError {
    StorageError::Rejected(CommandError { code })
}

fn expiry(time: Option<SystemTime>) -> StorageResult<Option<i64>> {
    time.map(|time| {
        let duration = time
            .duration_since(UNIX_EPOCH)
            .map_err(|_| error(ErrorCode::InvalidRequest))?;
        let ms =
            i64::try_from(duration.as_millis()).map_err(|_| error(ErrorCode::InvalidRequest))?;
        // No silent truncation that would alter the exclusive expiry boundary.
        if duration != Duration::from_millis(ms as u64) {
            return Err(error(ErrorCode::InvalidRequest));
        }
        Ok(ms)
    })
    .transpose()
}

fn future_expiry(clock: &impl Clock, time: Option<SystemTime>) -> CommandResult<()> {
    if let Some(end) = time
        && end <= clock.now()?
    {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    Ok(())
}

impl WriteContext<'_> {
    /// Initial installation provisioning only. Host must obtain explicit local
    /// confirmation and retain the pending key through commit. No localhost,
    /// IPC or transport request can invoke this trusted infrastructure entry.
    /// Revoking every admin never makes the installation eligible again.
    pub fn bootstrap_access(&mut self, pending: &PendingAccess) -> StorageResult<()> {
        let result = (|| {
            let count: i64 = self.connection().query_row(
                "SELECT (SELECT count(*) FROM access_bootstrap) +
                        (SELECT count(*) FROM access_principals)",
                [],
                |r| r.get(0),
            )?;
            if count != 0 {
                return Err(error(ErrorCode::AccessDenied));
            }
            self.store_access(
                &pending.subject,
                &pending.key,
                &GrantScope::Administration,
                None,
            )?;
            self.connection().execute(
                "INSERT INTO access_bootstrap VALUES (1, ?1)",
                [&pending.subject],
            )?;
            Ok(())
        })();
        self.checked(result)
    }

    /// Fixed ManageAccess policy, checked after queue admission and before any
    /// target lookup. A newly issued credential has an independent stable subject.
    pub fn issue_access(
        &mut self,
        credential: &Credential<'_>,
        clock: &impl Clock,
        pending: &PendingAccess,
        scope: &GrantScope,
        expires_at: Option<SystemTime>,
    ) -> StorageResult<()> {
        let result = with_authorized(
            self,
            credential,
            &[RequiredAccess::ManageAccess],
            clock,
            |ctx, _| {
                future_expiry(clock, expires_at)?;
                if ctx.load_access(&pending.subject)?.is_some() {
                    return Err(CommandError {
                        code: ErrorCode::InvalidRequest,
                    });
                }
                ctx.store_access(&pending.subject, &pending.key, scope, expires_at)?;
                Ok(())
            },
        );
        self.checked(result.map_err(StorageError::from))
    }

    /// Explicit regrant/permission change, always with a fresh key. The subject
    /// and its receipts persist, while every previous key remains invalid.
    pub fn replace_access(
        &mut self,
        credential: &Credential<'_>,
        clock: &impl Clock,
        subject: &str,
        replacement: &AccessKey,
        scope: &GrantScope,
        expires_at: Option<SystemTime>,
    ) -> StorageResult<()> {
        let result = with_authorized(
            self,
            credential,
            &[RequiredAccess::ManageAccess],
            clock,
            |ctx, _| {
                future_expiry(clock, expires_at)?;
                if ctx.load_access(subject)?.is_none() {
                    return Err(CommandError {
                        code: ErrorCode::NotFound,
                    });
                }
                ctx.store_access(subject, replacement, scope, expires_at)?;
                Ok(())
            },
        );
        self.checked(result.map_err(StorageError::from))
    }

    pub fn revoke_access(
        &mut self,
        credential: &Credential<'_>,
        clock: &impl Clock,
        subject: &str,
    ) -> StorageResult<()> {
        let result = with_authorized(
            self,
            credential,
            &[RequiredAccess::ManageAccess],
            clock,
            |ctx, _| zedone_core::access_commands::AccessStore::revoke_subject(ctx, subject),
        );
        self.checked(result.map_err(StorageError::from))
    }

    pub fn inspect_access(
        &mut self,
        credential: &Credential<'_>,
        clock: &impl Clock,
        subject: &str,
    ) -> StorageResult<Option<Grant>> {
        Ok(with_authorized(
            self,
            credential,
            &[RequiredAccess::ManageAccess],
            clock,
            |ctx, _| Ok(ctx.load_access(subject)?),
        )?)
    }

    fn store_access(
        &mut self,
        subject: &str,
        key: &AccessKey,
        scope: &GrantScope,
        expires_at: Option<SystemTime>,
    ) -> StorageResult<()> {
        let expires_ms = expiry(expires_at)?;
        if let GrantScope::Spaces(spaces) = scope
            && (spaces.is_empty() || spaces.keys().any(|id| id.is_empty()))
        {
            return Err(error(ErrorCode::InvalidRequest));
        }
        // Every old verifier remains a tombstone, even after replacement/restart.
        let reused: bool = self.connection().query_row(
            "SELECT EXISTS(SELECT 1 FROM access_keys WHERE verifier = ?1)",
            [key.verifier().as_slice()],
            |r| r.get(0),
        )?;
        if reused {
            return Err(error(ErrorCode::InvalidRequest));
        }
        self.connection().execute(
            "INSERT INTO access_principals VALUES (?1, ?2, ?3, 0)
             ON CONFLICT(subject) DO UPDATE SET
             administration = excluded.administration, expires_ms = excluded.expires_ms,
             revoked = 0",
            rusqlite::params![
                subject,
                matches!(scope, GrantScope::Administration),
                expires_ms
            ],
        )?;
        self.connection().execute(
            "UPDATE access_keys SET active = 0 WHERE subject = ?1",
            [subject],
        )?;
        self.connection().execute(
            "INSERT INTO access_keys VALUES (?1, ?2, 1)",
            (key.verifier().as_slice(), subject),
        )?;
        self.connection()
            .execute("DELETE FROM access_spaces WHERE subject = ?1", [subject])?;
        if let GrantScope::Spaces(spaces) = scope {
            for (space, permission) in spaces {
                self.connection().execute(
                    "INSERT INTO access_spaces VALUES (?1, ?2, ?3)",
                    (subject, space, *permission == SpacePermission::ReadWrite),
                )?;
            }
        }
        self.ensure_administrator()?;
        Ok(())
    }

    // Keep a reversible management path. Transfer administration first, then
    // revoke/downgrade the old grant; never introduce an implicit owner bypass.
    fn ensure_administrator(&self) -> StorageResult<()> {
        let present: bool = self.connection().query_row(
            "SELECT EXISTS(SELECT 1 FROM access_principals
             WHERE administration = 1 AND revoked = 0 AND expires_ms IS NULL)",
            [],
            |r| r.get(0),
        )?;
        if present {
            Ok(())
        } else {
            Err(error(ErrorCode::InvalidRequest))
        }
    }

    fn load_access(&self, subject: &str) -> StorageResult<Option<Grant>> {
        let row: Option<(bool, Option<i64>, bool)> = self.connection().query_row(
            "SELECT administration, expires_ms, revoked FROM access_principals WHERE subject = ?1",
            [subject], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?))).optional()?;
        let Some((administration, expires_ms, revoked)) = row else {
            return Ok(None);
        };
        let spaces = self
            .connection()
            .prepare("SELECT space, writable FROM access_spaces WHERE subject = ?1")?
            .query_map([subject], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    if r.get::<_, bool>(1)? {
                        SpacePermission::ReadWrite
                    } else {
                        SpacePermission::Read
                    },
                ))
            })?
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        if administration != spaces.is_empty() {
            return Err(StorageError::InvalidDatabase);
        }
        Ok(Some(Grant {
            subject: subject.into(),
            scope: if administration {
                GrantScope::Administration
            } else {
                GrantScope::Spaces(spaces)
            },
            expires_at: expires_ms
                .map(|ms| {
                    UNIX_EPOCH
                        .checked_add(Duration::from_millis(ms as u64))
                        .ok_or(StorageError::InvalidDatabase)
                })
                .transpose()?,
            revoked,
        }))
    }
}

impl AccessTransaction for WriteContext<'_> {
    fn resolve_access(
        &mut self,
        credential: &Credential<'_>,
    ) -> CommandResult<Option<ResolvedAccess>> {
        if credential.0.len() != 32 {
            return Ok(None);
        }
        let subject: Option<String> = self
            .connection()
            .query_row(
                "SELECT p.subject FROM access_keys k JOIN access_principals p ON p.subject = k.subject
             WHERE k.verifier = ?1 AND k.active = 1 AND p.revoked = 0",
                [verifier(credential.0).as_slice()],
                |r| r.get(0),
            )
            .optional()
            .map_err(StorageError::from)?;
        subject
            .map(|subject| {
                let grant = self
                    .load_access(&subject)?
                    .ok_or(StorageError::InvalidDatabase)?;
                Ok(ResolvedAccess { subject, grant })
            })
            .transpose()
    }
}

impl ReplayTransaction for WriteContext<'_> {
    fn recorded(&mut self, key: &[u8]) -> CommandResult<Option<RecordedCommand>> {
        Ok(self.read().recorded(key)?)
    }

    fn record(&mut self, key: &[u8], command: &RecordedCommand) -> CommandResult<()> {
        Ok(WriteContext::record(self, key, command)?)
    }
}

impl zedone_core::access_commands::AccessStore for WriteContext<'_> {
    fn access_snapshot(
        &mut self,
        subject: &str,
    ) -> CommandResult<Option<zedone_core::access_commands::AccessSnapshot>> {
        use zedone_core::access_commands::{AccessMode, AccessScope, AccessSnapshot};
        let Some(grant) = self.load_access(subject)? else {
            return Ok(None);
        };
        let scope = match grant.scope {
            GrantScope::Administration => AccessScope::administration(),
            GrantScope::Spaces(spaces) => AccessScope::spaces(
                spaces
                    .into_iter()
                    .map(|(id, permission)| {
                        (
                            id,
                            match permission {
                                SpacePermission::Read => AccessMode::Read,
                                SpacePermission::ReadWrite => AccessMode::ReadWrite,
                            },
                        )
                    })
                    .collect(),
            ),
        };
        let keys = self
            .connection()
            .prepare(
                "SELECT verifier, active FROM access_keys WHERE subject = ?1 ORDER BY verifier",
            )
            .map_err(StorageError::from)?
            .query_map([subject], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, bool>(1)?))
            })
            .map_err(StorageError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)?;
        let expires_at_unix_ms = expiry(grant.expires_at)?;
        let version = crate::state_version(
            &("access.v1", subject),
            &(&scope, expires_at_unix_ms, grant.revoked, keys),
        )?;
        Ok(Some(AccessSnapshot {
            subject: subject.into(),
            scope,
            expires_at_unix_ms,
            revoked: grant.revoked,
            version,
        }))
    }

    fn revoke_subject(&mut self, subject: &str) -> CommandResult<()> {
        let result = (|| -> StorageResult<()> {
            if self.connection().execute(
                "UPDATE access_principals SET revoked = 1 WHERE subject = ?1",
                [subject],
            )? == 0
            {
                return Err(error(ErrorCode::NotFound));
            }
            self.connection().execute(
                "UPDATE access_keys SET active = 0 WHERE subject = ?1",
                [subject],
            )?;
            self.ensure_administrator()
        })();
        Ok(self.checked(result)?)
    }
}

/// In addition to DDL/FK checks, reject incomplete bootstrap/key/grant state.
/// No missing or damaged authorization state is treated as a new installation.
pub(crate) fn validate(connection: &rusqlite::Connection) -> StorageResult<()> {
    let invalid: bool = connection.query_row(
        "SELECT EXISTS(
            SELECT 1 FROM access_principals p WHERE
              p.administration = EXISTS(SELECT 1 FROM access_spaces s WHERE s.subject = p.subject)
              OR (SELECT count(*) FROM access_keys k WHERE k.subject = p.subject AND k.active = 1)
                 != 1 - p.revoked
        ) OR ((SELECT count(*) FROM access_principals) > 0
              AND ((SELECT count(*) FROM access_bootstrap) != 1
                   OR NOT EXISTS(SELECT 1 FROM access_principals
                       WHERE administration = 1 AND revoked = 0 AND expires_ms IS NULL)))
          OR ((SELECT count(*) FROM access_principals) = 0
              AND (SELECT count(*) FROM access_bootstrap) != 0)",
        [],
        |r| r.get(0),
    )?;
    if invalid {
        Err(StorageError::InvalidDatabase)
    } else {
        Ok(())
    }
}
