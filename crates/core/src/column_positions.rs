use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    cards::CardStore,
    columns::ColumnSnapshot,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    ordering::{OrderStore, Rank},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    wire::{CommandRequest, CommandResponse},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ColumnPositionChoice {
    pub position: Rank,
    pub before: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ColumnPositions {
    pub column: ColumnSnapshot,
    pub choices: Vec<ColumnPositionChoice>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectColumnPositions {
    pub space: String,
    pub column: String,
}
impl Command for InspectColumnPositions {
    const NAME: &'static str = "column.inspect_positions";
    type Output = ColumnPositions;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveColumnPosition {
    pub space: String,
    pub column: String,
    pub position: Rank,
    pub expected: StateVersion,
}
impl Command for ResolveColumnPosition {
    const NAME: &'static str = "column.resolve_position";
    type Output = ColumnSnapshot;
}
pub trait ColumnPositionStore: OrderStore {
    fn resolve_column_position(
        &mut self,
        space: &str,
        column: &str,
        position: &Rank,
    ) -> CommandResult<()>;
}
fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn validate<C: Command>(
    request: &CommandRequest<C>,
    space: &str,
    column: &str,
) -> CommandResult<()> {
    if request.operation != C::NAME || !valid_id(space) || !valid_id(column) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}

#[utoipa::path(post, path = "/v1/commands/column.inspect_positions", operation_id = "column_inspect_positions", request_body = CommandRequest<InspectColumnPositions>, responses((status = 200, body = CommandResponse<ColumnPositions>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectColumnPositions>,
) -> CommandResult<Execution<ColumnPositions>> {
    let p = &request.parameters;
    validate(request, &p.space, &p.column)?;
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::Space {
            id: &p.space,
            permission: SpacePermission::Read,
        }],
        request.id,
        p,
        |tx| {
            let column = tx.column_snapshot(&p.space, &p.column)?;
            let columns = tx.columns(&p.space)?;
            let choices = column
                .positions
                .iter()
                .map(|position| {
                    let key = position.key(&column.column);
                    let before = columns
                        .iter()
                        .filter(|other| {
                            other.column != column.column
                                && other.deletion_groups.is_empty()
                                && other.state != SpaceState::Trashed
                                && other.positions.len() == 1
                                && other.positions[0].key(&other.column) > key
                        })
                        .min_by_key(|other| (other.positions[0].key(&other.column), &other.column))
                        .map(|other| other.column.clone());
                    ColumnPositionChoice {
                        position: position.clone(),
                        before,
                    }
                })
                .collect();
            Ok(ColumnPositions { column, choices })
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/column.resolve_position", operation_id = "column_resolve_position", request_body = CommandRequest<ResolveColumnPosition>, responses((status = 200, body = CommandResponse<ColumnSnapshot>)), security(("access_key" = [])))]
pub fn resolve(
    tx: &mut impl ColumnPositionStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ResolveColumnPosition>,
) -> CommandResult<Execution<ColumnSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, &p.column)?;
    if !p.position.valid() {
        return Err(error(ErrorCode::InvalidRequest));
    }
    replay::execute(
        tx,
        credential,
        clock,
        &[RequiredAccess::Space {
            id: &p.space,
            permission: SpacePermission::ReadWrite,
        }],
        request.id,
        p,
        |tx| {
            let column = tx.column_snapshot(&p.space, &p.column)?;
            replay::require_current(p.expected, column.version)?;
            if column.positions.len() < 2
                || column.state != SpaceState::Active
                || !column.deletion_groups.is_empty()
            {
                return Err(error(ErrorCode::Conflict));
            }
            if !column.positions.contains(&p.position) {
                return Err(error(ErrorCode::InvalidRequest));
            }
            match tx.space_snapshot(&p.space)?.state {
                SpaceState::Active => {}
                SpaceState::Conflicted => return Err(error(ErrorCode::Conflict)),
                SpaceState::Trashed => return Err(error(ErrorCode::InvalidRequest)),
            }
            tx.resolve_column_position(&p.space, &p.column, &p.position)?;
            tx.column_snapshot(&p.space, &p.column)
        },
    )
}
