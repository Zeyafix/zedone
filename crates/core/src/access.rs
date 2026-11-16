use std::{collections::BTreeMap, time::SystemTime};

use crate::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
};

#[cfg(test)]
mod tests;

pub struct Credential<'a>(pub &'a [u8]);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpacePermission {
    Read,
    ReadWrite,
}

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

#[derive(Clone)]
pub struct Grant {
    pub subject: String,
    pub scope: GrantScope,
    pub expires_at: Option<SystemTime>,
    pub revoked: bool,
}

#[derive(Clone)]
pub struct ResolvedAccess {
    pub subject: String,
    pub grant: Grant,
}

#[derive(Clone, Copy)]
pub enum RequiredAccess<'a> {
    ActiveGrant,
    Space {
        id: &'a str,
        permission: SpacePermission,
    },
    ManageDevices,
    ManageAccess,
}

pub trait AccessTransaction {
    fn resolve_access(
        &mut self,
        credential: &Credential<'_>,
    ) -> CommandResult<Option<ResolvedAccess>>;
}

pub trait Clock {
    fn now(&self) -> CommandResult<SystemTime>;

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
