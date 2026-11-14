//! Administrative preparation only. An invitation grants no access and is not
//! an unauthenticated pairing endpoint. Secrets never enter command receipts.
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess},
    access_commands::MAX_EXPIRY_UNIX_MS,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction, StateVersion},
    wire::{CommandRequest, CommandResponse},
};

pub const INVITATION_LIFETIME_MS: i64 = 600_000;

/// SHA-256("zedone/invitation-code/v1\0" || 32 random bytes).
/// Neither an access key nor the original invitation code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct InvitationVerifier(pub [u8; 32]);
impl utoipa::PartialSchema for InvitationVerifier {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        crate::replay::byte_array_schema::<32>().into()
    }
}
impl ToSchema for InvitationVerifier {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum InvitationStatus {
    Open,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InvitationSnapshot {
    pub invitation: String,
    #[schema(minimum = 0, maximum = 253402300799999_i64)]
    pub created_at_unix_ms: i64,
    #[schema(minimum = 0, maximum = 253402300799999_i64)]
    pub expires_at_unix_ms: i64,
    pub status: InvitationStatus,
    pub version: StateVersion,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateInvitation {
    pub invitation: String,
    pub verifier: InvitationVerifier,
}
impl Command for CreateInvitation {
    const NAME: &'static str = "invitation.create";
    type Output = InvitationSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectInvitation {
    pub invitation: String,
}
impl Command for InspectInvitation {
    const NAME: &'static str = "invitation.inspect";
    type Output = InvitationSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CancelInvitation {
    pub invitation: String,
    pub expected: StateVersion,
}
impl Command for CancelInvitation {
    const NAME: &'static str = "invitation.cancel";
    type Output = InvitationSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListInvitations {
    pub after: Option<String>,
}
impl Command for ListInvitations {
    const NAME: &'static str = "invitation.list";
    type Output = InvitationList;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InvitationList {
    #[schema(max_items = 50)]
    pub entries: Vec<InvitationSnapshot>,
    pub next: Option<String>,
}

/// Trusted persistence projection, never decoded from client JSON.
pub struct InvitationRecord {
    pub invitation: String,
    pub created_ms: i64,
    pub expires_ms: i64,
    pub cancelled: bool,
    pub version: StateVersion,
}
impl InvitationRecord {
    fn snapshot(self, now: i64) -> InvitationSnapshot {
        InvitationSnapshot {
            invitation: self.invitation,
            created_at_unix_ms: self.created_ms,
            expires_at_unix_ms: self.expires_ms,
            status: if self.cancelled {
                InvitationStatus::Cancelled
            } else if now >= self.expires_ms {
                InvitationStatus::Expired
            } else {
                InvitationStatus::Open
            },
            version: self.version,
        }
    }
}

pub trait InvitationStore: ReplayTransaction {
    fn invitation(&mut self, id: &str) -> CommandResult<Option<InvitationRecord>>;
    fn insert_invitation(
        &mut self,
        id: &str,
        verifier: InvitationVerifier,
        created_ms: i64,
        expires_ms: i64,
    ) -> CommandResult<()>;
    fn cancel_invitation(&mut self, id: &str) -> CommandResult<()>;
    /// Ascending IDs, exclusive cursor, at most 51 for one-page lookahead.
    fn invitations(&mut self, after: Option<&str>) -> CommandResult<Vec<InvitationRecord>>;
}

pub fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn validate<C: Command>(id: &str, request: &CommandRequest<C>) -> CommandResult<()> {
    if request.operation != C::NAME || !valid_id(id) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
fn now_ms(clock: &impl Clock) -> CommandResult<i64> {
    let ms = clock
        .now()?
        .duration_since(UNIX_EPOCH)
        .map_err(|_| error(ErrorCode::Unavailable))?
        .as_millis();
    let ms = i64::try_from(ms).map_err(|_| error(ErrorCode::Unavailable))?;
    if ms > MAX_EXPIRY_UNIX_MS {
        return Err(error(ErrorCode::Unavailable));
    }
    Ok(ms)
}
fn load(tx: &mut impl InvitationStore, id: &str) -> CommandResult<InvitationRecord> {
    tx.invitation(id)?.ok_or(error(ErrorCode::NotFound))
}

#[utoipa::path(post, path = "/v1/commands/invitation.create", operation_id = "invitation_create", request_body = CommandRequest<CreateInvitation>, responses((status = 200, body = CommandResponse<InvitationSnapshot>)), security(("access_key" = [])))]
pub fn create(
    tx: &mut impl InvitationStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CreateInvitation>,
) -> CommandResult<Execution<InvitationSnapshot>> {
    let p = &request.parameters;
    validate(&p.invitation, request)?;
    if p.verifier.0 == [0; 32] {
        return Err(error(ErrorCode::InvalidRequest));
    }
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices],
        request.id,
        p,
        |tx| {
            let now = now_ms(clock)?;
            let end = now
                .checked_add(INVITATION_LIFETIME_MS)
                .filter(|end| *end <= MAX_EXPIRY_UNIX_MS)
                .ok_or(error(ErrorCode::Unavailable))?;
            tx.insert_invitation(&p.invitation, p.verifier, now, end)?;
            Ok(load(tx, &p.invitation)?.snapshot(now))
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/invitation.inspect", operation_id = "invitation_inspect", request_body = CommandRequest<InspectInvitation>, responses((status = 200, body = CommandResponse<InvitationSnapshot>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl InvitationStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectInvitation>,
) -> CommandResult<Execution<InvitationSnapshot>> {
    let p = &request.parameters;
    validate(&p.invitation, request)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices],
        request.id,
        p,
        |tx| Ok(load(tx, &p.invitation)?.snapshot(now_ms(clock)?)),
    )
}

#[utoipa::path(post, path = "/v1/commands/invitation.cancel", operation_id = "invitation_cancel", request_body = CommandRequest<CancelInvitation>, responses((status = 200, body = CommandResponse<InvitationSnapshot>)), security(("access_key" = [])))]
pub fn cancel(
    tx: &mut impl InvitationStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CancelInvitation>,
) -> CommandResult<Execution<InvitationSnapshot>> {
    let p = &request.parameters;
    validate(&p.invitation, request)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices],
        request.id,
        p,
        |tx| {
            let before = load(tx, &p.invitation)?;
            replay::require_current(p.expected, before.version)?;
            tx.cancel_invitation(&p.invitation)?;
            Ok(load(tx, &p.invitation)?.snapshot(now_ms(clock)?))
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/invitation.list", operation_id = "invitation_list", request_body = CommandRequest<ListInvitations>, responses((status = 200, body = CommandResponse<InvitationList>)), security(("access_key" = [])))]
pub fn list(
    tx: &mut impl InvitationStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ListInvitations>,
) -> CommandResult<Execution<InvitationList>> {
    let p = &request.parameters;
    if request.operation != ListInvitations::NAME
        || p.after.as_deref().is_some_and(|id| !valid_id(id))
    {
        return Err(error(ErrorCode::InvalidRequest));
    }
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::ManageDevices],
        request.id,
        p,
        |tx| {
            let now = now_ms(clock)?;
            let mut records = tx.invitations(p.after.as_deref())?;
            let next = if records.len() > 50 {
                records.truncate(50);
                records.last().map(|record| record.invitation.clone())
            } else {
                None
            };
            Ok(InvitationList {
                entries: records
                    .into_iter()
                    .map(|record| record.snapshot(now))
                    .collect(),
                next,
            })
        },
    )
}
