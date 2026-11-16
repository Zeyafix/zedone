use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess},
    access_commands::{self, AccessScope, AccessStore, KeyVerifier},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    invitations::{InvitationStore, InvitationVerifier, now_ms, valid_id},
    replay::{self, Execution, RecordedCommand, StateVersion},
    wire::{CommandRequest, CommandResponse},
};

pub const MAX_ATTEMPTS: u8 = 5;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PendingVerifier(pub [u8; 32]);

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RequestPairing {
    pub request: String,
    pub invitation: String,
    pub label: String,
    pub access_verifier: KeyVerifier,
}
impl Command for RequestPairing {
    const NAME: &'static str = "pairing.request";
    type Output = PairingSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectPairing {
    pub request: String,
}
impl Command for InspectPairing {
    const NAME: &'static str = "pairing.inspect";
    type Output = PairingSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelPairing {
    pub request: String,
    pub expected: StateVersion,
}
impl Command for CancelPairing {
    const NAME: &'static str = "pairing.cancel";
    type Output = PairingSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ConfirmPairing {
    pub request: String,
    pub expected: StateVersion,
    pub scope: AccessScope,
    #[schema(minimum = 0, maximum = 253402300799999_i64)]
    pub expires_at_unix_ms: Option<i64>,
}
impl Command for ConfirmPairing {
    const NAME: &'static str = "pairing.confirm";
    type Output = PairingSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RejectPairing {
    pub request: String,
    pub expected: StateVersion,
}
impl Command for RejectPairing {
    const NAME: &'static str = "pairing.reject";
    type Output = PairingSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct UnlinkPairing {
    pub request: String,
    pub expected: StateVersion,
}
impl Command for UnlinkPairing {
    const NAME: &'static str = "pairing.unlink";
    type Output = PairingSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListPairings {
    pub after: Option<String>,
}
impl Command for ListPairings {
    const NAME: &'static str = "pairing.list";
    type Output = PairingList;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PairingList {
    #[schema(max_items = 50)]
    pub entries: Vec<PairingSnapshot>,
    pub next: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PairingStatus {
    Pending,
    Cancelled,
    Rejected,
    Confirmed,
    Unlinked,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PairingSnapshot {
    pub request: String,
    pub invitation: String,
    pub label: String,
    #[schema(minimum = 0, maximum = 253402300799999_i64)]
    pub created_at_unix_ms: i64,
    pub status: PairingStatus,
    pub subject: Option<String>,
    pub version: StateVersion,
}
pub struct PairingRecord {
    pub snapshot: PairingSnapshot,
    pub verifier: PendingVerifier,
    pub access_verifier: KeyVerifier,
}
pub trait PairingStore: InvitationStore + AccessStore {
    fn pairing(&mut self, id: &str) -> CommandResult<Option<PairingRecord>>;
    fn insert_pairing(
        &mut self,
        parameters: &RequestPairing,
        pending: PendingVerifier,
        now: i64,
    ) -> CommandResult<()>;
    fn fail_invitation(&mut self, id: &str) -> CommandResult<()>;
    fn cancel_pairing(&mut self, id: &str) -> CommandResult<()>;
    fn decide_pairing(&mut self, id: &str, confirmed: bool) -> CommandResult<()>;
    fn pairings(&mut self, after: Option<&str>) -> CommandResult<Vec<PairingSnapshot>>;
    fn verifier_matches(&self, left: &[u8; 32], right: &[u8; 32]) -> bool;
}
fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn load(tx: &mut impl PairingStore, id: &str) -> CommandResult<PairingRecord> {
    tx.pairing(id)?.ok_or(error(ErrorCode::NotFound))
}
fn validate<C: Command>(id: &str, r: &CommandRequest<C>) -> CommandResult<()> {
    if r.operation != C::NAME || r.id == [0; 16] || !valid_id(id) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
fn peer_replay<C: Command, T: PairingStore>(
    tx: &mut T,
    r: &CommandRequest<C>,
    target: &str,
    action: impl FnOnce(&mut T) -> CommandResult<C::Output>,
) -> CommandResult<Execution<C::Output>>
where
    C::Output: serde::de::DeserializeOwned,
{
    let key = replay::canonical_json(&("zedone.pending-command.v1", target, r.id))?;
    let request = replay::canonical_json(&(C::NAME, &r.parameters))?;
    if let Some(previous) = tx.recorded(&key)? {
        if previous.request != request {
            return Err(error(ErrorCode::CommandMismatch));
        }
        return Ok(Execution {
            result: serde_json::from_slice(&previous.result)
                .map_err(|_| error(ErrorCode::Unavailable))?,
            replayed: true,
        });
    }
    let result = action(tx)?;
    tx.record(
        &key,
        &RecordedCommand {
            request,
            result: serde_json::to_vec(&result).map_err(|_| error(ErrorCode::Unavailable))?,
        },
    )?;
    Ok(Execution {
        result,
        replayed: false,
    })
}

pub fn request(
    tx: &mut impl PairingStore,
    clock: &impl Clock,
    r: &CommandRequest<RequestPairing>,
    code: InvitationVerifier,
    pending: PendingVerifier,
) -> CommandResult<Option<Execution<PairingSnapshot>>> {
    let p = &r.parameters;
    validate(&p.request, r)?;
    if !valid_id(&p.invitation)
        || p.label.trim().is_empty()
        || p.label.chars().count() > 80
        || p.label.chars().any(char::is_control)
        || pending.0 == [0; 32]
        || p.access_verifier.0 == [0; 32]
    {
        return Err(error(ErrorCode::InvalidRequest));
    }
    let denied_key = replay::canonical_json(&("zedone.pending-denied.v1", &p.request, pending.0))?;
    let denied_request = replay::canonical_json(&(r, code.0))?;
    if let Some(previous) = tx.recorded(&denied_key)? {
        if previous.request != denied_request {
            return Err(error(ErrorCode::CommandMismatch));
        }
        return Ok(None);
    }
    fn deny(
        tx: &mut impl PairingStore,
        key: &[u8],
        request: Vec<u8>,
    ) -> CommandResult<Option<Execution<PairingSnapshot>>> {
        tx.record(
            key,
            &RecordedCommand {
                request,
                result: b"null".to_vec(),
            },
        )?;
        Ok(None)
    }
    let Some(invitation) = tx.invitation(&p.invitation)? else {
        return Err(error(ErrorCode::Unavailable));
    };
    let correct = tx.verifier_matches(&invitation.verifier.0, &code.0);
    if let Some(existing) = tx.pairing(&p.request)? {
        if !correct || !tx.verifier_matches(&existing.verifier.0, &pending.0) {
            return Ok(None);
        }
        return peer_replay(tx, r, &p.request, |_| Err(error(ErrorCode::AccessDenied))).map(Some);
    }
    if invitation.cancelled
        || invitation.used
        || invitation.failures >= MAX_ATTEMPTS
        || now_ms(clock)? >= invitation.expires_ms
    {
        return Ok(None);
    }
    if !correct {
        tx.fail_invitation(&p.invitation)?;
        return deny(tx, &denied_key, denied_request);
    }
    clock.check_expiry(Some(
        std::time::UNIX_EPOCH + std::time::Duration::from_millis(invitation.expires_ms as u64),
    ))?;
    peer_replay(tx, r, &p.request, |tx| {
        tx.insert_pairing(p, pending, now_ms(clock)?)?;
        Ok(load(tx, &p.request)?.snapshot)
    })
    .map(Some)
}

pub fn peer_inspect(
    tx: &mut impl PairingStore,
    r: &CommandRequest<InspectPairing>,
    pending: PendingVerifier,
) -> CommandResult<Execution<PairingSnapshot>> {
    validate(&r.parameters.request, r)?;
    authenticate(tx, &r.parameters.request, pending)?;
    peer_replay(tx, r, &r.parameters.request, |tx| {
        Ok(load(tx, &r.parameters.request)?.snapshot)
    })
}
pub fn peer_cancel(
    tx: &mut impl PairingStore,
    r: &CommandRequest<CancelPairing>,
    pending: PendingVerifier,
) -> CommandResult<Execution<PairingSnapshot>> {
    validate(&r.parameters.request, r)?;
    authenticate(tx, &r.parameters.request, pending)?;
    peer_replay(tx, r, &r.parameters.request, |tx| {
        cancel_current(tx, &r.parameters)
    })
}
pub fn peer_unlink(
    tx: &mut impl PairingStore,
    r: &CommandRequest<UnlinkPairing>,
    pending: PendingVerifier,
) -> CommandResult<Execution<PairingSnapshot>> {
    validate(&r.parameters.request, r)?;
    authenticate(tx, &r.parameters.request, pending)?;
    peer_replay(tx, r, &r.parameters.request, |tx| {
        unlink_current(tx, &r.parameters)
    })
}
fn authenticate(
    tx: &mut impl PairingStore,
    id: &str,
    pending: PendingVerifier,
) -> CommandResult<()> {
    let record = tx.pairing(id)?.ok_or(error(ErrorCode::AccessDenied))?;
    if !tx.verifier_matches(&record.verifier.0, &pending.0) {
        return Err(error(ErrorCode::AccessDenied));
    }
    Ok(())
}
fn cancel_current(tx: &mut impl PairingStore, p: &CancelPairing) -> CommandResult<PairingSnapshot> {
    let before = load(tx, &p.request)?.snapshot;
    replay::require_current(p.expected, before.version)?;
    if before.status != PairingStatus::Pending {
        return Err(error(ErrorCode::InvalidRequest));
    }
    tx.cancel_pairing(&p.request)?;
    Ok(load(tx, &p.request)?.snapshot)
}

#[utoipa::path(post, path = "/v1/commands/pairing.confirm", operation_id = "pairing_confirm", request_body = CommandRequest<ConfirmPairing>, responses((status = 200, body = CommandResponse<PairingSnapshot>)), security(("access_key" = [])))]
pub fn confirm(
    tx: &mut impl PairingStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<ConfirmPairing>,
) -> CommandResult<Execution<PairingSnapshot>> {
    let p = &r.parameters;
    validate(&p.request, r)?;
    replay::execute_as(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices, RequiredAccess::ManageAccess],
        r.id,
        p,
        |tx, administrator| {
            let before = load(tx, &p.request)?;
            replay::require_current(p.expected, before.snapshot.version)?;
            if before.snapshot.status != PairingStatus::Pending || administrator == p.request {
                return Err(error(ErrorCode::InvalidRequest));
            }
            access_commands::validate_issue(&p.scope, before.access_verifier)?;
            let expires = access_commands::future_expiry(clock, p.expires_at_unix_ms)?;
            clock.check_expiry(expires)?;
            tx.issue_subject(&p.request, before.access_verifier, &p.scope, expires)?;
            tx.decide_pairing(&p.request, true)?;
            Ok(load(tx, &p.request)?.snapshot)
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/pairing.reject", operation_id = "pairing_reject", request_body = CommandRequest<RejectPairing>, responses((status = 200, body = CommandResponse<PairingSnapshot>)), security(("access_key" = [])))]
pub fn reject(
    tx: &mut impl PairingStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<RejectPairing>,
) -> CommandResult<Execution<PairingSnapshot>> {
    let p = &r.parameters;
    validate(&p.request, r)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices],
        r.id,
        p,
        |tx| {
            let before = load(tx, &p.request)?.snapshot;
            replay::require_current(p.expected, before.version)?;
            if before.status != PairingStatus::Pending {
                return Err(error(ErrorCode::InvalidRequest));
            }
            tx.decide_pairing(&p.request, false)?;
            Ok(load(tx, &p.request)?.snapshot)
        },
    )
}

fn unlink_current(tx: &mut impl PairingStore, p: &UnlinkPairing) -> CommandResult<PairingSnapshot> {
    let before = load(tx, &p.request)?.snapshot;
    replay::require_current(p.expected, before.version)?;
    if before.status != PairingStatus::Confirmed {
        return Err(error(ErrorCode::InvalidRequest));
    }
    tx.revoke_subject(&p.request)?;
    Ok(load(tx, &p.request)?.snapshot)
}

#[utoipa::path(post, path = "/v1/commands/pairing.unlink", operation_id = "pairing_unlink", request_body = CommandRequest<UnlinkPairing>, responses((status = 200, body = CommandResponse<PairingSnapshot>)), security(("access_key" = [])))]
pub fn unlink(
    tx: &mut impl PairingStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<UnlinkPairing>,
) -> CommandResult<Execution<PairingSnapshot>> {
    validate(&r.parameters.request, r)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices, RequiredAccess::ManageAccess],
        r.id,
        &r.parameters,
        |tx| unlink_current(tx, &r.parameters),
    )
}

#[utoipa::path(post, path = "/v1/commands/pairing.inspect", operation_id = "pairing_inspect", request_body = CommandRequest<InspectPairing>, responses((status = 200, body = CommandResponse<PairingSnapshot>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl PairingStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<InspectPairing>,
) -> CommandResult<Execution<PairingSnapshot>> {
    validate(&r.parameters.request, r)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices],
        r.id,
        &r.parameters,
        |tx| Ok(load(tx, &r.parameters.request)?.snapshot),
    )
}
#[utoipa::path(post, path = "/v1/commands/pairing.cancel", operation_id = "pairing_cancel", request_body = CommandRequest<CancelPairing>, responses((status = 200, body = CommandResponse<PairingSnapshot>)), security(("access_key" = [])))]
pub fn cancel(
    tx: &mut impl PairingStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<CancelPairing>,
) -> CommandResult<Execution<PairingSnapshot>> {
    validate(&r.parameters.request, r)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices],
        r.id,
        &r.parameters,
        |tx| cancel_current(tx, &r.parameters),
    )
}
#[utoipa::path(post, path = "/v1/commands/pairing.list", operation_id = "pairing_list", request_body = CommandRequest<ListPairings>, responses((status = 200, body = CommandResponse<PairingList>)), security(("access_key" = [])))]
pub fn list(
    tx: &mut impl PairingStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<ListPairings>,
) -> CommandResult<Execution<PairingList>> {
    if r.operation != ListPairings::NAME
        || r.parameters.after.as_deref().is_some_and(|s| !valid_id(s))
    {
        return Err(error(ErrorCode::InvalidRequest));
    }
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices],
        r.id,
        &r.parameters,
        |tx| {
            let mut entries = tx.pairings(r.parameters.after.as_deref())?;
            let next = if entries.len() > 50 {
                entries.truncate(50);
                entries.last().map(|r| r.request.clone())
            } else {
                None
            };
            Ok(PairingList { entries, next })
        },
    )
}
