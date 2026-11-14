//! A pending credential authorizes only its own inspection/cancellation.
//! It is never resolved as a normal access principal or an administrator.
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess},
    access_commands::KeyVerifier,
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
    /// Claimed display name, never an authenticated device identity.
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
    pub version: StateVersion,
}
pub struct PairingRecord {
    pub snapshot: PairingSnapshot,
    pub verifier: PendingVerifier,
}
pub trait PairingStore: InvitationStore {
    fn pairing(&mut self, id: &str) -> CommandResult<Option<PairingRecord>>;
    fn insert_pairing(
        &mut self,
        parameters: &RequestPairing,
        pending: PendingVerifier,
        now: i64,
    ) -> CommandResult<()>;
    fn fail_invitation(&mut self, id: &str) -> CommandResult<()>;
    fn cancel_pairing(&mut self, id: &str) -> CommandResult<()>;
    fn pairings(&mut self, after: Option<&str>) -> CommandResult<Vec<PairingSnapshot>>;
    /// Compare secrets in constant time in the crypto-capable persistence layer.
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

/// None is a committed authentication failure, including its attempt counter.
/// Err is rolled back. The adapter must preserve this distinction.
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
    let Some(invitation) = tx.invitation(&p.invitation)? else {
        return Ok(None);
    };
    let correct = tx.verifier_matches(&invitation.verifier.0, &code.0);
    // A retry must prove the same pending secret, not merely know the invite.
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
        return Ok(None);
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
    tx.cancel_pairing(&p.request)?;
    Ok(load(tx, &p.request)?.snapshot)
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
