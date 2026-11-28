use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction, StateVersion},
    snapshots::{ImportedSpace, MAX_ARCHIVE_BYTES},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectMissingRecovery {
    #[schema(schema_with = crate::snapshots::archive_schema)]
    pub archive: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MissingRecoveryState {
    pub space: String,
    pub names: Vec<String>,
    pub version: StateVersion,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreMissingSnapshot {
    pub space: String,
    #[schema(schema_with = crate::snapshots::archive_schema)]
    pub archive: String,
    pub expected: StateVersion,
}

impl Command for InspectMissingRecovery {
    const NAME: &'static str = "space.recovery.inspect_missing";
    type Output = MissingRecoveryState;
}

impl Command for RestoreMissingSnapshot {
    const NAME: &'static str = "space.recovery.restore_missing";
    type Output = ImportedSpace;
}

pub trait MissingRecoveryStore: ReplayTransaction {
    fn missing_recovery_state(&mut self, archive: &str) -> CommandResult<MissingRecoveryState>;
    fn restore_missing_snapshot(
        &mut self,
        request: &RestoreMissingSnapshot,
        subject: &str,
    ) -> CommandResult<ImportedSpace>;
}

fn valid_archive(archive: &str) -> bool {
    !archive.is_empty()
        && archive.len() <= MAX_ARCHIVE_BYTES * 2
        && archive.len().is_multiple_of(2)
        && archive
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn invalid() -> CommandError {
    CommandError {
        code: ErrorCode::InvalidRequest,
    }
}

#[utoipa::path(post, path = "/v1/commands/space.recovery.inspect_missing", operation_id = "space_recovery_inspect_missing", request_body = CommandRequest<InspectMissingRecovery>, responses((status = 200, body = CommandResponse<MissingRecoveryState>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl MissingRecoveryStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectMissingRecovery>,
) -> CommandResult<Execution<MissingRecoveryState>> {
    if request.operation != InspectMissingRecovery::NAME
        || request.id == [0; 16]
        || !valid_archive(&request.parameters.archive)
    {
        return Err(invalid());
    }
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, _| {
            Ok(Execution {
                result: tx.missing_recovery_state(&request.parameters.archive)?,
                replayed: false,
            })
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/space.recovery.restore_missing", operation_id = "space_recovery_restore_missing", request_body = CommandRequest<RestoreMissingSnapshot>, responses((status = 200, body = CommandResponse<ImportedSpace>)), security(("access_key" = [])))]
pub fn restore(
    tx: &mut impl MissingRecoveryStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RestoreMissingSnapshot>,
) -> CommandResult<Execution<ImportedSpace>> {
    let p = &request.parameters;
    if request.operation != RestoreMissingSnapshot::NAME
        || !crate::spaces::valid_id(&p.space)
        || !valid_archive(&p.archive)
    {
        return Err(invalid());
    }
    let execution = replay::execute_as(
        tx,
        credential,
        clock,
        &[RequiredAccess::ActiveGrant],
        request.id,
        p,
        |tx, subject| tx.restore_missing_snapshot(p, subject),
    )?;
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::Space {
            id: &execution.result.space,
            permission: SpacePermission::ReadWrite,
        }],
        clock,
        |_, _| Ok(()),
    )?;
    Ok(execution)
}
