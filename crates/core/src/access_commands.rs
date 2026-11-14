//! Bounded access administration commands. Adapter code cannot select a policy
//! or callback; every operation requires separate access administration.
use std::{
    collections::BTreeMap,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction, StateVersion},
    wire::{CommandRequest, CommandResponse},
};

/// Last millisecond of year 9999 UTC. A common, exactly representable wire
/// range avoids differing SystemTime limits on Windows/Android and JS rounding.
pub const MAX_EXPIRY_UNIX_MS: i64 = 253_402_300_799_999;

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
    AdministrationAndSpaces(AdministrationAndSpacesScope),
    Spaces(SpacesScope),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AdministrationAndSpacesScope {
    pub kind: AdministrationAndSpacesKind,
    pub spaces: BTreeMap<String, AccessMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AdministrationAndSpacesKind {
    AdministrationAndSpaces,
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
    pub fn administration_and_spaces(spaces: BTreeMap<String, AccessMode>) -> Self {
        Self::AdministrationAndSpaces(AdministrationAndSpacesScope {
            kind: AdministrationAndSpacesKind::AdministrationAndSpaces,
            spaces,
        })
    }
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

/// Domain-separated SHA-256 of a recipient-generated 256-bit access key.
/// This is registration material, never accepted as a bearer credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct KeyVerifier(pub [u8; 32]);
impl utoipa::PartialSchema for KeyVerifier {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        crate::replay::byte_array_schema::<32>().into()
    }
}
impl ToSchema for KeyVerifier {}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IssueAccess {
    pub subject: String,
    pub verifier: KeyVerifier,
    pub scope: AccessScope,
    #[schema(minimum = 0, maximum = 253402300799999_i64)]
    pub expires_at_unix_ms: Option<i64>,
}
impl Command for IssueAccess {
    const NAME: &'static str = "access.issue";
    type Output = AccessSnapshot;
}

/// Permission changes and re-issuance always rotate the key. A stale preview
/// cannot replace newer permissions or revive an already revoked credential.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReplaceAccess {
    pub subject: String,
    pub verifier: KeyVerifier,
    pub scope: AccessScope,
    #[schema(minimum = 0, maximum = 253402300799999_i64)]
    pub expires_at_unix_ms: Option<i64>,
    pub expected: StateVersion,
}
impl Command for ReplaceAccess {
    const NAME: &'static str = "access.replace";
    type Output = AccessSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListAccess {
    /// Exclusive subject cursor. New reads need new command IDs.
    pub after: Option<String>,
}
impl Command for ListAccess {
    const NAME: &'static str = "access.list";
    type Output = AccessList;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AccessList {
    /// At most 50 entries, including revoked/expired grants. No secret/verifier.
    #[schema(max_items = 50)]
    pub entries: Vec<AccessSnapshot>,
    pub next: Option<String>,
}

pub trait AccessStore: ReplayTransaction {
    fn access_snapshot(&mut self, subject: &str) -> CommandResult<Option<AccessSnapshot>>;
    fn revoke_subject(&mut self, subject: &str) -> CommandResult<()>;
    fn issue_subject(
        &mut self,
        subject: &str,
        verifier: KeyVerifier,
        scope: &AccessScope,
        expires_at: Option<SystemTime>,
    ) -> CommandResult<()>;
    fn replace_subject(
        &mut self,
        subject: &str,
        verifier: KeyVerifier,
        scope: &AccessScope,
        expires_at: Option<SystemTime>,
    ) -> CommandResult<()>;
    fn list_access(&mut self, after: Option<&str>) -> CommandResult<AccessList>;
}

fn valid_id(subject: &str) -> bool {
    subject.len() == 32
        && subject
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn validate<C: Command>(subject: &str, request: &CommandRequest<C>) -> CommandResult<()> {
    if request.operation != C::NAME || !valid_id(subject) {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    Ok(())
}

fn validate_issue(scope: &AccessScope, verifier: KeyVerifier) -> CommandResult<()> {
    let spaces = match scope {
        AccessScope::Administration(_) => None,
        AccessScope::AdministrationAndSpaces(scope) => Some(&scope.spaces),
        AccessScope::Spaces(scope) => Some(&scope.spaces),
    };
    if verifier.0 == [0; 32]
        || spaces.is_some_and(|s| s.is_empty() || s.keys().any(|s| !valid_id(s)))
    {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    Ok(())
}

fn future_expiry(clock: &impl Clock, ms: Option<i64>) -> CommandResult<Option<SystemTime>> {
    ms.map(|ms| {
        let invalid = || CommandError {
            code: ErrorCode::InvalidRequest,
        };
        if !(0..=MAX_EXPIRY_UNIX_MS).contains(&ms) {
            return Err(invalid());
        }
        let duration = Duration::from_millis(u64::try_from(ms).map_err(|_| invalid())?);
        let time = UNIX_EPOCH.checked_add(duration).ok_or_else(invalid)?;
        if time <= clock.now()? {
            return Err(invalid());
        }
        Ok(time)
    })
    .transpose()
}

#[utoipa::path(post, path = "/v1/commands/access.issue", operation_id = "issue_access", request_body = CommandRequest<IssueAccess>, responses((status = 200, body = CommandResponse<AccessSnapshot>)), security(("access_key" = [])))]
pub fn issue(
    tx: &mut impl AccessStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<IssueAccess>,
) -> CommandResult<Execution<AccessSnapshot>> {
    let p = &request.parameters;
    validate(&p.subject, request)?;
    validate_issue(&p.scope, p.verifier)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageAccess],
        request.id,
        p,
        |tx| {
            let expires_at = future_expiry(clock, p.expires_at_unix_ms)?;
            tx.issue_subject(&p.subject, p.verifier, &p.scope, expires_at)?;
            snapshot(tx, &p.subject)
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/access.replace", operation_id = "replace_access", request_body = CommandRequest<ReplaceAccess>, responses((status = 200, body = CommandResponse<AccessSnapshot>)), security(("access_key" = [])))]
pub fn replace(
    tx: &mut impl AccessStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ReplaceAccess>,
) -> CommandResult<Execution<AccessSnapshot>> {
    let p = &request.parameters;
    validate(&p.subject, request)?;
    validate_issue(&p.scope, p.verifier)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageAccess],
        request.id,
        p,
        |tx| {
            let before = snapshot(tx, &p.subject)?;
            replay::require_current(p.expected, before.version)?;
            let expires_at = future_expiry(clock, p.expires_at_unix_ms)?;
            tx.replace_subject(&p.subject, p.verifier, &p.scope, expires_at)?;
            snapshot(tx, &p.subject)
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/access.list", operation_id = "list_access", request_body = CommandRequest<ListAccess>, responses((status = 200, body = CommandResponse<AccessList>)), security(("access_key" = [])))]
pub fn list(
    tx: &mut impl AccessStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ListAccess>,
) -> CommandResult<Execution<AccessList>> {
    if request.operation != ListAccess::NAME
        || request
            .parameters
            .after
            .as_deref()
            .is_some_and(|s| !valid_id(s))
    {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageAccess],
        request.id,
        &request.parameters,
        |tx| tx.list_access(request.parameters.after.as_deref()),
    )
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
