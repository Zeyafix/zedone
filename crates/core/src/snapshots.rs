use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction},
    wire::{CommandRequest, CommandResponse},
};

pub const MAX_ARCHIVE_BYTES: usize = 1024 * 1024;
pub const MAX_ARCHIVE_CHANGES: usize = 10_000;

pub(crate) fn archive_schema() -> utoipa::openapi::schema::Object {
    use utoipa::openapi::schema::{ObjectBuilder, Type};
    ObjectBuilder::new()
        .schema_type(Type::String)
        .pattern(Some(format!(
            r"^([0-9a-f]{{2}}){{1,{MAX_ARCHIVE_BYTES}}}(?![\s\S])"
        )))
        .build()
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ExportSpace {
    pub space: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SpaceArchive {
    #[schema(schema_with = archive_schema)]
    pub archive: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportSpace {
    #[schema(schema_with = archive_schema)]
    pub archive: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ImportedSpace {
    pub space: String,
}

impl Command for ExportSpace {
    const NAME: &'static str = "space.export";
    type Output = SpaceArchive;
}

impl Command for ImportSpace {
    const NAME: &'static str = "space.import";
    type Output = ImportedSpace;
}

pub trait SnapshotStore: ReplayTransaction {
    fn export_space(&mut self, space: &str, created_at_unix_ms: u64)
    -> CommandResult<SpaceArchive>;
    fn import_space(&mut self, archive: &str, subject: &str) -> CommandResult<ImportedSpace>;
}

fn invalid() -> CommandError {
    CommandError {
        code: ErrorCode::InvalidRequest,
    }
}

#[utoipa::path(post, path = "/v1/commands/space.export", operation_id = "space_export", request_body = CommandRequest<ExportSpace>, responses((status = 200, body = CommandResponse<SpaceArchive>)), security(("access_key" = [])))]
pub fn export(
    tx: &mut impl SnapshotStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ExportSpace>,
) -> CommandResult<Execution<SpaceArchive>> {
    if request.operation != ExportSpace::NAME || !crate::spaces::valid_id(&request.parameters.space)
    {
        return Err(invalid());
    }
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::Space {
            id: &request.parameters.space,
            permission: SpacePermission::Read,
        }],
        request.id,
        &request.parameters,
        |tx| {
            let now = clock
                .now()?
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| CommandError {
                    code: ErrorCode::Unavailable,
                })?;
            let now = u64::try_from(now.as_millis()).map_err(|_| CommandError {
                code: ErrorCode::Unavailable,
            })?;
            tx.export_space(&request.parameters.space, now)
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/space.import", operation_id = "space_import", request_body = CommandRequest<ImportSpace>, responses((status = 200, body = CommandResponse<ImportedSpace>)), security(("access_key" = [])))]
pub fn import(
    tx: &mut impl SnapshotStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ImportSpace>,
) -> CommandResult<Execution<ImportedSpace>> {
    let archive = &request.parameters.archive;
    if request.operation != ImportSpace::NAME
        || archive.is_empty()
        || archive.len() > MAX_ARCHIVE_BYTES * 2
        || !archive.len().is_multiple_of(2)
        || !archive
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(invalid());
    }
    let execution = replay::execute_as(
        tx,
        credential,
        clock,
        &[RequiredAccess::ActiveGrant],
        request.id,
        &request.parameters,
        |tx, subject| tx.import_space(archive, subject),
    )?;
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::Space {
            id: &execution.result.space,
            permission: SpacePermission::Read,
        }],
        clock,
        |_, _| Ok(()),
    )?;
    Ok(execution)
}
