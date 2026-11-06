//! Column policy shared by HTTP, CLI and IPC. No adapter supplies authority.
use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, SpaceStore, valid_id},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ColumnSnapshot {
    pub space: String,
    pub column: String,
    pub names: Vec<String>,
    pub state: SpaceState,
    pub deletion_groups: Vec<String>,
    /// All distinct position registers; conflicting values are never overwritten.
    pub positions: Vec<u64>,
    /// Unknown/future contents block trash and restore until supported.
    pub empty: bool,
    pub version: StateVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ColumnList {
    pub columns: Vec<ColumnSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateColumn {
    pub space: String,
    pub column: String,
    pub name: String,
}
impl Command for CreateColumn {
    const NAME: &'static str = "column.create";
    type Output = ColumnSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadColumn {
    pub space: String,
    pub column: String,
}
impl Command for ReadColumn {
    const NAME: &'static str = "column.read";
    type Output = ColumnSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListColumns {
    pub space: String,
    pub include_trashed: bool,
}
impl Command for ListColumns {
    const NAME: &'static str = "column.list";
    type Output = ColumnList;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameColumn {
    pub space: String,
    pub column: String,
    pub name: String,
    pub expected: StateVersion,
}
impl Command for RenameColumn {
    const NAME: &'static str = "column.rename";
    type Output = ColumnSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashColumn {
    pub space: String,
    pub column: String,
    pub expected: StateVersion,
}
impl Command for TrashColumn {
    const NAME: &'static str = "column.trash";
    type Output = ColumnSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreColumn {
    pub space: String,
    pub column: String,
    pub expected: StateVersion,
}
impl Command for RestoreColumn {
    const NAME: &'static str = "column.restore";
    type Output = ColumnSnapshot;
}

pub trait ColumnStore: SpaceStore {
    fn create_column(
        &mut self,
        space: &str,
        column: &str,
        name: &str,
    ) -> CommandResult<ColumnSnapshot>;
    fn column_snapshot(&mut self, space: &str, column: &str) -> CommandResult<ColumnSnapshot>;
    fn columns(&mut self, space: &str) -> CommandResult<Vec<ColumnSnapshot>>;
    fn rename_column(&mut self, space: &str, column: &str, name: &str) -> CommandResult<()>;
    fn set_column_lifecycle(
        &mut self,
        space: &str,
        column: &str,
        lifecycle: &str,
    ) -> CommandResult<()>;
}

fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn validate<C: Command>(
    request: &CommandRequest<C>,
    space: &str,
    column: Option<&str>,
) -> CommandResult<()> {
    if request.operation != C::NAME || !valid_id(space) || column.is_some_and(|id| !valid_id(id)) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
fn active(tx: &mut impl ColumnStore, space: &str) -> CommandResult<()> {
    match tx.space_snapshot(space)?.state {
        SpaceState::Active => Ok(()),
        SpaceState::Conflicted => Err(error(ErrorCode::Conflict)),
        SpaceState::Trashed => Err(error(ErrorCode::InvalidRequest)),
    }
}
fn required(space: &str, permission: SpacePermission) -> [RequiredAccess<'_>; 1] {
    [RequiredAccess::Space {
        id: space,
        permission,
    }]
}

#[utoipa::path(post, path = "/v1/commands/column.create", operation_id = "column_create", request_body = CommandRequest<CreateColumn>, responses((status = 200, body = CommandResponse<ColumnSnapshot>)), security(("access_key" = [])))]
pub fn create(
    tx: &mut impl ColumnStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CreateColumn>,
) -> CommandResult<Execution<ColumnSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.column))?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| {
            active(tx, &p.space)?;
            tx.create_column(&p.space, &p.column, &p.name)
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/column.read", operation_id = "column_read", request_body = CommandRequest<ReadColumn>, responses((status = 200, body = CommandResponse<ColumnSnapshot>)), security(("access_key" = [])))]
pub fn read(
    tx: &mut impl ColumnStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ReadColumn>,
) -> CommandResult<Execution<ColumnSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.column))?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::Read),
        request.id,
        p,
        |tx| tx.column_snapshot(&p.space, &p.column),
    )
}

#[utoipa::path(post, path = "/v1/commands/column.list", operation_id = "column_list", request_body = CommandRequest<ListColumns>, responses((status = 200, body = CommandResponse<ColumnList>)), security(("access_key" = [])))]
pub fn list(
    tx: &mut impl ColumnStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ListColumns>,
) -> CommandResult<Execution<ColumnList>> {
    let p = &request.parameters;
    validate(request, &p.space, None)?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::Read),
        request.id,
        p,
        |tx| {
            let mut columns = tx.columns(&p.space)?;
            columns.retain(|c| p.include_trashed || c.state != SpaceState::Trashed);
            // Concurrent appends may share a position. Identity is the stable tie-break.
            columns.sort_by(|a, b| a.positions.cmp(&b.positions).then(a.column.cmp(&b.column)));
            Ok(ColumnList { columns })
        },
    )
}

enum Change<'a> {
    Rename(&'a str),
    Trash,
    Restore,
}
fn change<C: Command<Output = ColumnSnapshot>>(
    tx: &mut impl ColumnStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<C>,
    target: (&str, &str, StateVersion),
    action: Change<'_>,
) -> CommandResult<Execution<ColumnSnapshot>> {
    let (space, column, expected) = target;
    validate(request, space, Some(column))?;
    replay::execute_as(
        tx,
        credential,
        clock,
        &required(space, SpacePermission::ReadWrite),
        request.id,
        &request.parameters,
        |tx, subject| {
            active(tx, space)?;
            let before = tx.column_snapshot(space, column)?;
            replay::require_current(expected, before.version)?;
            if before.names.len() != 1
                || before.positions.len() != 1
                || before.state == SpaceState::Conflicted
            {
                return Err(error(ErrorCode::Conflict));
            }
            match action {
                Change::Rename(name) if before.state == SpaceState::Active => {
                    tx.rename_column(space, column, name)?
                }
                Change::Trash if before.state == SpaceState::Active && before.empty => {
                    let id: String = request.id.iter().map(|b| format!("{b:02x}")).collect();
                    tx.set_column_lifecycle(space, column, &format!("trash:{subject}:{id}"))?;
                }
                Change::Restore if before.state == SpaceState::Trashed && before.empty => {
                    tx.set_column_lifecycle(space, column, "active")?
                }
                _ => return Err(error(ErrorCode::InvalidRequest)),
            }
            tx.column_snapshot(space, column)
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/column.rename", operation_id = "column_rename", request_body = CommandRequest<RenameColumn>, responses((status = 200, body = CommandResponse<ColumnSnapshot>)), security(("access_key" = [])))]
pub fn rename(
    tx: &mut impl ColumnStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RenameColumn>,
) -> CommandResult<Execution<ColumnSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.column, p.expected),
        Change::Rename(&p.name),
    )
}
#[utoipa::path(post, path = "/v1/commands/column.trash", operation_id = "column_trash", request_body = CommandRequest<TrashColumn>, responses((status = 200, body = CommandResponse<ColumnSnapshot>)), security(("access_key" = [])))]
pub fn trash(
    tx: &mut impl ColumnStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<TrashColumn>,
) -> CommandResult<Execution<ColumnSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.column, p.expected),
        Change::Trash,
    )
}
#[utoipa::path(post, path = "/v1/commands/column.restore", operation_id = "column_restore", request_body = CommandRequest<RestoreColumn>, responses((status = 200, body = CommandResponse<ColumnSnapshot>)), security(("access_key" = [])))]
pub fn restore(
    tx: &mut impl ColumnStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RestoreColumn>,
) -> CommandResult<Execution<ColumnSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.column, p.expected),
        Change::Restore,
    )
}
