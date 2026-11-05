//! The first bounded application commands. Adapter code cannot select a policy
//! or callback; both operations always require separate access administration.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction, StateVersion},
    wire::{CommandRequest, CommandResponse},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessMode {
    Read,
    ReadWrite,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum AccessScope {
    Administration(AdministrationScope),
    Spaces(SpacesScope),
}

// Separate strict structs keep Serde and utoipa's variant schemas identical,
// including additionalProperties:false (as for wire::CommandResponse).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AdministrationScope {
    pub kind: AdministrationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdministrationKind {
    Administration,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SpacesScope {
    pub kind: SpacesKind,
    pub spaces: BTreeMap<String, AccessMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpacesKind {
    Spaces,
}

impl AccessScope {
    pub fn administration() -> Self {
        Self::Administration(AdministrationScope {
            kind: AdministrationKind::Administration,
        })
    }
    pub fn spaces(spaces: BTreeMap<String, AccessMode>) -> Self {
        Self::Spaces(SpacesScope {
            kind: SpacesKind::Spaces,
            spaces,
        })
    }
}

/// No keys or verifiers leave storage. Version includes retired/current key
/// identities internally so a regrant cannot revive a stale revocation request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AccessSnapshot {
    pub subject: String,
    pub scope: AccessScope,
    pub expires_at_unix_ms: Option<i64>,
    pub revoked: bool,
    pub version: StateVersion,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectAccess {
    pub subject: String,
}
impl Command for InspectAccess {
    const NAME: &'static str = "access.inspect";
    type Output = AccessSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RevokeAccess {
    pub subject: String,
    pub expected: StateVersion,
}
impl Command for RevokeAccess {
    const NAME: &'static str = "access.revoke";
    type Output = AccessSnapshot;
}

pub trait AccessStore: ReplayTransaction {
    fn access_snapshot(&mut self, subject: &str) -> CommandResult<Option<AccessSnapshot>>;
    fn revoke_subject(&mut self, subject: &str) -> CommandResult<()>;
}

fn validate<C: Command>(subject: &str, request: &CommandRequest<C>) -> CommandResult<()> {
    if request.operation != C::NAME
        || subject.len() != 32
        || !subject
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    Ok(())
}
fn snapshot(tx: &mut impl AccessStore, subject: &str) -> CommandResult<AccessSnapshot> {
    tx.access_snapshot(subject)?.ok_or(CommandError {
        code: ErrorCode::NotFound,
    })
}

#[utoipa::path(post, path = "/v1/commands/access.inspect", request_body = CommandRequest<InspectAccess>, responses((status = 200, body = CommandResponse<AccessSnapshot>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl AccessStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectAccess>,
) -> CommandResult<Execution<AccessSnapshot>> {
    validate(&request.parameters.subject, request)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageAccess],
        request.id,
        &request.parameters,
        |tx| snapshot(tx, &request.parameters.subject),
    )
}

#[utoipa::path(post, path = "/v1/commands/access.revoke", request_body = CommandRequest<RevokeAccess>, responses((status = 200, body = CommandResponse<AccessSnapshot>)), security(("access_key" = [])))]
pub fn revoke(
    tx: &mut impl AccessStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RevokeAccess>,
) -> CommandResult<Execution<AccessSnapshot>> {
    validate(&request.parameters.subject, request)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageAccess],
        request.id,
        &request.parameters,
        |tx| {
            let before = snapshot(tx, &request.parameters.subject)?;
            replay::require_current(request.parameters.expected, before.version)?;
            tx.revoke_subject(&request.parameters.subject)?;
            snapshot(tx, &request.parameters.subject)
        },
    )
}
