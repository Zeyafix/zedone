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

/// Administration is granted separately and never implies access to data.
#[derive(Clone)]
pub enum GrantScope {
    Spaces(BTreeMap<String, SpacePermission>),
    Administration,
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
/// checked at this boundary; this does not define cancellation/commit semantics
/// for future long-running operations. The callback must not publish effects
/// before commit. As with other core handlers, adapters cannot supply it.
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
    let now = clock.now()?;
    if access.subject.is_empty()
        || access.subject != grant.subject
        || grant.revoked
        || grant.expires_at.is_some_and(|end| now >= end)
        || !required
            .iter()
            .all(|requirement| match (&grant.scope, requirement) {
                (GrantScope::Spaces(spaces), RequiredAccess::Space { id, permission }) => {
                    !id.is_empty()
                        && spaces.get(*id).is_some_and(|granted| {
                            *permission == SpacePermission::Read
                                || *granted == SpacePermission::ReadWrite
                        })
                }
                (
                    GrantScope::Administration,
                    RequiredAccess::ManageDevices | RequiredAccess::ManageAccess,
                ) => true,
                _ => false,
            })
    {
        return Err(denied());
    }
    operation(transaction, &access.subject)
}
