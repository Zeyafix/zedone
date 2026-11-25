use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction, StateVersion, require_current},
    snapshots::{MAX_ARCHIVE_BYTES, SpaceArchive},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectRecovery {
    pub space: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SafetyBackup {
    pub id: String,
    pub created_at_unix_ms: u64,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoveryState {
    pub space: String,
    pub version: StateVersion,
    pub paused: bool,
    pub pause_version: StateVersion,
    pub backup: Option<SafetyBackup>,
    pub cleanup_pending: bool,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreSnapshot {
    pub space: String,
    #[schema(schema_with = crate::snapshots::archive_schema)]
    pub archive: String,
    pub expected: StateVersion,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoredSnapshot {
    pub space: String,
    pub safety: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadSafetyBackup {
    pub space: String,
    pub safety: String,
}
impl Command for InspectRecovery {
    const NAME: &'static str = "space.recovery.inspect";
    type Output = RecoveryState;
}
impl Command for RestoreSnapshot {
    const NAME: &'static str = "space.recovery.restore";
    type Output = RestoredSnapshot;
}
impl Command for ReadSafetyBackup {
    const NAME: &'static str = "space.recovery.read_backup";
    type Output = SpaceArchive;
}
pub trait RecoveryStore: ReplayTransaction {
    fn recovery_state(&mut self, space: &str) -> CommandResult<RecoveryState>;
    fn restore_snapshot(
        &mut self,
        request: &RestoreSnapshot,
        date: u64,
    ) -> CommandResult<RestoredSnapshot>;
    fn read_safety_backup(&mut self, space: &str, safety: &str) -> CommandResult<SpaceArchive>;
    fn set_recovery_pause(&mut self, request: &RecoveryPause) -> CommandResult<RecoveryState>;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RecoveryPause {
    pub space: String,
    pub paused: bool,
    pub expected: StateVersion,
}
impl Command for RecoveryPause {
    const NAME: &'static str = "space.recovery.pause";
    type Output = RecoveryState;
}

#[utoipa::path(post, path = "/v1/commands/space.recovery.pause", operation_id = "space_recovery_pause", request_body = CommandRequest<RecoveryPause>, responses((status = 200, body = CommandResponse<RecoveryState>)), security(("access_key" = [])))]
pub fn pause(
    tx: &mut impl RecoveryStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RecoveryPause>,
) -> CommandResult<Execution<RecoveryState>> {
    let p = &request.parameters;
    if request.operation != RecoveryPause::NAME || !crate::spaces::valid_id(&p.space) {
        return Err(invalid());
    }
    replay::execute(
        tx,
        credential,
        clock,
        &scope(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| tx.set_recovery_pause(p),
    )
}
fn invalid() -> CommandError {
    CommandError {
        code: ErrorCode::InvalidRequest,
    }
}
fn scope(space: &str, permission: SpacePermission) -> [RequiredAccess<'_>; 1] {
    [RequiredAccess::Space {
        id: space,
        permission,
    }]
}

#[utoipa::path(post, path = "/v1/commands/space.recovery.inspect", operation_id = "space_recovery_inspect", request_body = CommandRequest<InspectRecovery>, responses((status = 200, body = CommandResponse<RecoveryState>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl RecoveryStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectRecovery>,
) -> CommandResult<Execution<RecoveryState>> {
    if request.operation != InspectRecovery::NAME
        || !crate::spaces::valid_id(&request.parameters.space)
        || request.id == [0; 16]
    {
        return Err(invalid());
    }
    with_authorized(
        tx,
        credential,
        &scope(&request.parameters.space, SpacePermission::Read),
        clock,
        |tx, _| {
            Ok(Execution {
                result: tx.recovery_state(&request.parameters.space)?,
                replayed: false,
            })
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/space.recovery.restore", operation_id = "space_recovery_restore", request_body = CommandRequest<RestoreSnapshot>, responses((status = 200, body = CommandResponse<RestoredSnapshot>)), security(("access_key" = [])))]
pub fn restore(
    tx: &mut impl RecoveryStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RestoreSnapshot>,
) -> CommandResult<Execution<RestoredSnapshot>> {
    let p = &request.parameters;
    if request.operation != RestoreSnapshot::NAME
        || !crate::spaces::valid_id(&p.space)
        || p.archive.is_empty()
        || p.archive.len() > MAX_ARCHIVE_BYTES * 2
        || !p.archive.len().is_multiple_of(2)
        || !p
            .archive
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    replay::execute(
        tx,
        credential,
        clock,
        &scope(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| {
            require_current(p.expected, tx.recovery_state(&p.space)?.version)?;
            let date = clock
                .now()?
                .duration_since(std::time::UNIX_EPOCH)
                .ok()
                .and_then(|d| u64::try_from(d.as_millis()).ok())
                .ok_or(CommandError {
                    code: ErrorCode::Unavailable,
                })?;
            tx.restore_snapshot(p, date)
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/space.recovery.read_backup", operation_id = "space_recovery_read_backup", request_body = CommandRequest<ReadSafetyBackup>, responses((status = 200, body = CommandResponse<SpaceArchive>)), security(("access_key" = [])))]
pub fn read_backup(
    tx: &mut impl RecoveryStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ReadSafetyBackup>,
) -> CommandResult<Execution<SpaceArchive>> {
    let p = &request.parameters;
    if request.operation != ReadSafetyBackup::NAME
        || !crate::spaces::valid_id(&p.space)
        || p.safety.is_empty()
        || p.safety.len() > 100
        || request.id == [0; 16]
    {
        return Err(invalid());
    }
    with_authorized(
        tx,
        credential,
        &scope(&p.space, SpacePermission::Read),
        clock,
        |tx, _| {
            Ok(Execution {
                result: tx.read_safety_backup(&p.space, &p.safety)?,
                replayed: false,
            })
        },
    )
}
