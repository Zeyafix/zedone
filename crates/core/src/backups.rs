use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction},
    snapshots::SpaceArchive,
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DailyCopy {
    pub id: String,
    pub created_at_unix_ms: u64,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum DailyStatus {
    Ready,
    Saved,
    Unchanged,
    DailyLimit,
    Waiting,
    Unavailable,
    CalendarRollback,
    Running,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DailyBackups {
    pub space: String,
    pub status: DailyStatus,
    pub last_success: Option<DailyCopy>,
    pub failed_sequence: Option<u64>,
    pub retry_after_ms: Option<u64>,
    pub copies: Vec<DailyCopy>,
    pub cleanup_pending: bool,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectDailyBackups {
    pub space: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RetryDailyBackup {
    pub space: String,
    pub failed_sequence: u64,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteDailyBackup {
    pub space: String,
    pub copy: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadDailyBackup {
    pub space: String,
    pub copy: String,
}
impl Command for InspectDailyBackups {
    const NAME: &'static str = "space.backups.inspect";
    type Output = DailyBackups;
}
impl Command for RetryDailyBackup {
    const NAME: &'static str = "space.backups.retry";
    type Output = DailyBackups;
}
impl Command for DeleteDailyBackup {
    const NAME: &'static str = "space.backups.delete";
    type Output = DailyBackups;
}
impl Command for ReadDailyBackup {
    const NAME: &'static str = "space.backups.read";
    type Output = SpaceArchive;
}
pub trait DailyBackupStore: ReplayTransaction {
    fn inspect_daily(&mut self, space: &str) -> CommandResult<DailyBackups>;
    fn retry_daily(&mut self, request: &RetryDailyBackup) -> CommandResult<DailyBackups>;
    fn delete_daily(&mut self, request: &DeleteDailyBackup) -> CommandResult<DailyBackups>;
    fn read_daily(&mut self, request: &ReadDailyBackup) -> CommandResult<SpaceArchive>;
}
fn scope(space: &str, permission: SpacePermission) -> [RequiredAccess<'_>; 1] {
    [RequiredAccess::Space {
        id: space,
        permission,
    }]
}
fn validate(valid: bool) -> CommandResult<()> {
    if valid {
        Ok(())
    } else {
        Err(CommandError {
            code: ErrorCode::InvalidRequest,
        })
    }
}
fn copy_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 100 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

#[utoipa::path(post, path = "/v1/commands/space.backups.inspect", operation_id = "space_backups_inspect", request_body = CommandRequest<InspectDailyBackups>, responses((status = 200, body = CommandResponse<DailyBackups>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl DailyBackupStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectDailyBackups>,
) -> CommandResult<Execution<DailyBackups>> {
    validate(
        request.operation == InspectDailyBackups::NAME
            && crate::spaces::valid_id(&request.parameters.space)
            && request.id != [0; 16],
    )?;
    with_authorized(
        tx,
        credential,
        &scope(&request.parameters.space, SpacePermission::Read),
        clock,
        |tx, _| {
            Ok(Execution {
                result: tx.inspect_daily(&request.parameters.space)?,
                replayed: false,
            })
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/space.backups.retry", operation_id = "space_backups_retry", request_body = CommandRequest<RetryDailyBackup>, responses((status = 200, body = CommandResponse<DailyBackups>)), security(("access_key" = [])))]
pub fn retry(
    tx: &mut impl DailyBackupStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RetryDailyBackup>,
) -> CommandResult<Execution<DailyBackups>> {
    let p = &request.parameters;
    validate(
        request.operation == RetryDailyBackup::NAME
            && crate::spaces::valid_id(&p.space)
            && p.failed_sequence > 0,
    )?;
    replay::execute(
        tx,
        credential,
        clock,
        &scope(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| tx.retry_daily(p),
    )
}
#[utoipa::path(post, path = "/v1/commands/space.backups.delete", operation_id = "space_backups_delete", request_body = CommandRequest<DeleteDailyBackup>, responses((status = 200, body = CommandResponse<DailyBackups>)), security(("access_key" = [])))]
pub fn delete(
    tx: &mut impl DailyBackupStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<DeleteDailyBackup>,
) -> CommandResult<Execution<DailyBackups>> {
    let p = &request.parameters;
    validate(
        request.operation == DeleteDailyBackup::NAME
            && crate::spaces::valid_id(&p.space)
            && copy_id(&p.copy),
    )?;
    replay::execute(
        tx,
        credential,
        clock,
        &scope(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| tx.delete_daily(p),
    )
}
#[utoipa::path(post, path = "/v1/commands/space.backups.read", operation_id = "space_backups_read", request_body = CommandRequest<ReadDailyBackup>, responses((status = 200, body = CommandResponse<SpaceArchive>)), security(("access_key" = [])))]
pub fn read(
    tx: &mut impl DailyBackupStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ReadDailyBackup>,
) -> CommandResult<Execution<SpaceArchive>> {
    let p = &request.parameters;
    validate(
        request.operation == ReadDailyBackup::NAME
            && crate::spaces::valid_id(&p.space)
            && copy_id(&p.copy)
            && request.id != [0; 16],
    )?;
    with_authorized(
        tx,
        credential,
        &scope(&p.space, SpacePermission::Read),
        clock,
        |tx, _| {
            Ok(Execution {
                result: tx.read_daily(p)?,
                replayed: false,
            })
        },
    )
}
