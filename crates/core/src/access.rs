//! Authorization of already authenticated, currently granted access.
//!
//! This is not credential issuance, a permission cache or a public API. The
//! trusted transaction resolves credentials anew and serializes authorization
//! with the operation and grant updates. Never implement it in a transport.

use std::{collections::BTreeMap, time::SystemTime};

use crate::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
};

#[cfg(test)]
mod tests;

/// Opaque authentication material. No Debug/Serialize implementation by design.
pub struct Credential<'a>(pub &'a [u8]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpacePermission {
    Read,
    ReadWrite,
}

/// Administration and exact data rights are independent, even when combined.
#[derive(Clone)]
pub enum GrantScope {
    Spaces(BTreeMap<String, SpacePermission>),
    Administration,
    AdministrationAndSpaces(BTreeMap<String, SpacePermission>),
}

impl GrantScope {
    pub fn spaces(&self) -> Option<&BTreeMap<String, SpacePermission>> {
        match self {
            Self::Spaces(spaces) | Self::AdministrationAndSpaces(spaces) => Some(spaces),
            Self::Administration => None,
        }
    }

    pub fn administration(&self) -> bool {
        matches!(
            self,
            Self::Administration | Self::AdministrationAndSpaces(_)
        )
    }

    fn valid(&self) -> bool {
        self.spaces()
            .is_none_or(|spaces| !spaces.is_empty() && spaces.keys().all(|id| !id.is_empty()))
    }
}

/// Trusted current state, never accepted from a request or restored space copy.
#[derive(Clone)]
pub struct Grant {
    pub subject: String,
    pub scope: GrantScope,
    pub expires_at: Option<SystemTime>,
    pub revoked: bool,
}

/// Identity authenticated by the resolver, independently of the grant's subject.
#[derive(Clone)]
pub struct ResolvedAccess {
    pub subject: String,
    pub grant: Grant,
}

/// Determined by the core handler from the actual operation and target spaces.
/// Clients cannot supply a weaker requirement in place of the handler's policy.
#[derive(Clone, Copy)]
pub enum RequiredAccess<'a> {
    /// Any current, nonempty grant; used for creation, not an issued capability.
    /// Does not authorize reading or changing an existing space.
    ActiveGrant,
    Space {
        id: &'a str,
        permission: SpacePermission,
    },
    ManageDevices,
    ManageAccess,
}

/// A trusted transaction/session, not a standalone credential lookup.
///
/// Implementations must authenticate the credential (including its own validity),
/// load authoritative grants, and exclude concurrent grant changes until the
/// synchronous operation finishes. Missing/invalid credentials return None;
/// unavailable state returns an error, never a fallback local/admin identity.
/// The persistence implementation and credential protocol are not yet exposed.
pub trait AccessTransaction {
    fn resolve_access(
        &mut self,
        credential: &Credential<'_>,
    ) -> CommandResult<Option<ResolvedAccess>>;
}

/// Trusted time source; timestamps from a request are not an authorization clock.
/// Clock validity and handling of rollback/restart belong to the infrastructure;
/// if time cannot be established, return an error instead of granting access.
pub trait Clock {
    fn now(&self) -> CommandResult<SystemTime>;

    /// Infrastructure also retains the earliest deadline for the final commit
    /// check. A custom clock is trusted infrastructure, never client input.
    fn check_expiry(&self, end: Option<SystemTime>) -> CommandResult<()> {
        let now = self.now()?;
        if end.is_some_and(|end| now >= end) {
            return Err(denied());
        }
        Ok(())
    }
}

fn denied() -> CommandError {
    CommandError {
        code: ErrorCode::AccessDenied,
    }
}

/// Re-resolve and check *all* required rights before reading a target, looking
/// up a saved command result or running a mutation. No reusable permit escapes.
///
/// Call inside the working transaction, after any queue wait. Grant expiry is
/// checked before and after the callback. Infrastructure checks registered
/// deadlines after flush immediately before commit (ADR-0038). The callback
/// must not publish effects before commit; adapters cannot supply it.
pub fn with_authorized<T: AccessTransaction, R>(
    transaction: &mut T,
    credential: &Credential<'_>,
    required: &[RequiredAccess<'_>],
    clock: &impl Clock,
    operation: impl FnOnce(&mut T, &str) -> CommandResult<R>,
) -> CommandResult<R> {
    if required.is_empty() || credential.0.is_empty() {
        return Err(denied());
    }
    let access = transaction.resolve_access(credential)?.ok_or_else(denied)?;
    let grant = &access.grant;
    clock.check_expiry(grant.expires_at)?;
    if access.subject.is_empty()
        || access.subject != grant.subject
        || grant.revoked
        || !grant.scope.valid()
        || !required.iter().all(|requirement| match requirement {
            RequiredAccess::ActiveGrant => true,
            RequiredAccess::Space { id, permission } => {
                !id.is_empty()
                    && grant
                        .scope
                        .spaces()
                        .and_then(|spaces| spaces.get(*id))
                        .is_some_and(|granted| {
                            *permission == SpacePermission::Read
                                || *granted == SpacePermission::ReadWrite
                        })
            }
            RequiredAccess::ManageDevices | RequiredAccess::ManageAccess => {
                grant.scope.administration()
            }
        })
    {
        return Err(denied());
    }
    let result = operation(transaction, &access.subject)?;
    clock.check_expiry(grant.expires_at)?;
    Ok(result)
}
