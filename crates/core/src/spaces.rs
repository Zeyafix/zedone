//! Product space operations. Fixed core policy, shared by every adapter.
use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction, StateVersion},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpaceState {
    Active,
    Trashed,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SpaceSnapshot {
    pub space: String,
    /// All distinct current names; multiple names require conflict resolution.
    pub names: Vec<String>,
    pub state: SpaceState,
    pub deletion_groups: Vec<String>,
    /// This slice only trashes/restores spaces without domain content.
    pub empty: bool,
    pub version: StateVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SpaceList {
    pub spaces: Vec<SpaceSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateSpace {
    pub space: String,
    pub name: String,
}
impl Command for CreateSpace {
    const NAME: &'static str = "space.create";
    type Output = SpaceSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadSpace {
    pub space: String,
}
impl Command for ReadSpace {
    const NAME: &'static str = "space.read";
    type Output = SpaceSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListSpaces {
    pub include_trashed: bool,
}
impl Command for ListSpaces {
    const NAME: &'static str = "space.list";
    type Output = SpaceList;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameSpace {
    pub space: String,
    pub name: String,
    pub expected: StateVersion,
}
impl Command for RenameSpace {
    const NAME: &'static str = "space.rename";
    type Output = SpaceSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashSpace {
    pub space: String,
    pub expected: StateVersion,
}
impl Command for TrashSpace {
    const NAME: &'static str = "space.trash";
    type Output = SpaceSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreSpace {
    pub space: String,
    pub expected: StateVersion,
}
impl Command for RestoreSpace {
    const NAME: &'static str = "space.restore";
    type Output = SpaceSnapshot;
}

/// Trusted storage port; authorization and replay always belong to core.
pub trait SpaceStore: ReplayTransaction {
    fn create_space(
        &mut self,
        space: &str,
        name: &str,
        subject: &str,
    ) -> CommandResult<SpaceSnapshot>;
    fn space_snapshot(&mut self, space: &str) -> CommandResult<SpaceSnapshot>;
    fn spaces_for_subject(&mut self, subject: &str) -> CommandResult<Vec<SpaceSnapshot>>;
    fn rename_space(&mut self, space: &str, name: &str) -> CommandResult<()>;
    fn set_space_lifecycle(&mut self, space: &str, lifecycle: &str) -> CommandResult<()>;
}

fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
pub(crate) fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id != "00000000000000000000000000000000"
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn validate<C: Command>(request: &CommandRequest<C>, space: Option<&str>) -> CommandResult<()> {
    if request.operation != C::NAME || space.is_some_and(|id| !valid_id(id)) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}

#[utoipa::path(post, path = "/v1/commands/space.create", request_body = CommandRequest<CreateSpace>, responses((status = 200, body = CommandResponse<SpaceSnapshot>)), security(("access_key" = [])))]
pub fn create(
    tx: &mut impl SpaceStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CreateSpace>,
) -> CommandResult<Execution<SpaceSnapshot>> {
    validate(request, Some(&request.parameters.space))?;
    replay::execute_as(
        tx,
        credential,
        clock,
        &[RequiredAccess::ActiveGrant],
        request.id,
        &request.parameters,
        |tx, subject| tx.create_space(&request.parameters.space, &request.parameters.name, subject),
    )
}

#[utoipa::path(post, path = "/v1/commands/space.read", request_body = CommandRequest<ReadSpace>, responses((status = 200, body = CommandResponse<SpaceSnapshot>)), security(("access_key" = [])))]
pub fn read(
    tx: &mut impl SpaceStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ReadSpace>,
) -> CommandResult<Execution<SpaceSnapshot>> {
    validate(request, Some(&request.parameters.space))?;
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
        |tx| tx.space_snapshot(&request.parameters.space),
    )
}

#[utoipa::path(post, path = "/v1/commands/space.list", request_body = CommandRequest<ListSpaces>, responses((status = 200, body = CommandResponse<SpaceList>)), security(("access_key" = [])))]
pub fn list(
    tx: &mut impl SpaceStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ListSpaces>,
) -> CommandResult<Execution<SpaceList>> {
    validate(request, None)?;
    let execution = replay::execute_as(
        tx,
        credential,
        clock,
        &[RequiredAccess::ActiveGrant],
        request.id,
        &request.parameters,
        |tx, subject| {
            let mut spaces = tx.spaces_for_subject(subject)?;
            spaces.retain(|space| {
                request.parameters.include_trashed || space.state != SpaceState::Trashed
            });
            // Name conflicts first, then Unicode lowercase, original spelling and ID.
            spaces.sort_by_cached_key(|space| {
                if space.names.len() > 1 {
                    (0, String::new(), String::new(), space.space.clone())
                } else {
                    let name = space.names.first().cloned().unwrap_or_default();
                    (1, name.to_lowercase(), name, space.space.clone())
                }
            });
            Ok(SpaceList { spaces })
        },
    )?;
    // A saved list may contain spaces removed from this subject's grant since
    // the original read. Validate every returned target before exposing it.
    if execution.replayed {
        let required: Vec<_> = execution
            .result
            .spaces
            .iter()
            .map(|space| RequiredAccess::Space {
                id: &space.space,
                permission: SpacePermission::Read,
            })
            .collect();
        if !required.is_empty() {
            with_authorized(tx, credential, &required, clock, |_, _| Ok(()))?;
        }
    }
    Ok(execution)
}

enum Change<'a> {
    Rename(&'a str),
    Trash,
    Restore,
}
fn change<C: Command<Output = SpaceSnapshot>>(
    tx: &mut impl SpaceStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<C>,
    target: (&str, StateVersion),
    action: Change<'_>,
) -> CommandResult<Execution<SpaceSnapshot>> {
    let (space, expected) = target;
    validate(request, Some(space))?;
    replay::execute_as(
        tx,
        credential,
        clock,
        &[RequiredAccess::Space {
            id: space,
            permission: SpacePermission::ReadWrite,
        }],
        request.id,
        &request.parameters,
        |tx, subject| {
            let before = tx.space_snapshot(space)?;
            replay::require_current(expected, before.version)?;
            if before.names.len() != 1 || before.state == SpaceState::Conflicted {
                return Err(error(ErrorCode::Conflict));
            }
            match action {
                Change::Rename(name) if before.state == SpaceState::Active => {
                    tx.rename_space(space, name)?
                }
                Change::Trash if before.state == SpaceState::Active && before.empty => {
                    let id: String = request.id.iter().map(|b| format!("{b:02x}")).collect();
                    tx.set_space_lifecycle(space, &format!("trash:{subject}:{id}"))?;
                }
                Change::Restore if before.state == SpaceState::Trashed && before.empty => {
                    tx.set_space_lifecycle(space, "active")?
                }
                _ => return Err(error(ErrorCode::InvalidRequest)),
            }
            tx.space_snapshot(space)
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/space.rename", request_body = CommandRequest<RenameSpace>, responses((status = 200, body = CommandResponse<SpaceSnapshot>)), security(("access_key" = [])))]
pub fn rename(
    tx: &mut impl SpaceStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RenameSpace>,
) -> CommandResult<Execution<SpaceSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, p.expected),
        Change::Rename(&p.name),
    )
}

#[utoipa::path(post, path = "/v1/commands/space.trash", request_body = CommandRequest<TrashSpace>, responses((status = 200, body = CommandResponse<SpaceSnapshot>)), security(("access_key" = [])))]
pub fn trash(
    tx: &mut impl SpaceStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<TrashSpace>,
) -> CommandResult<Execution<SpaceSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, p.expected),
        Change::Trash,
    )
}

#[utoipa::path(post, path = "/v1/commands/space.restore", request_body = CommandRequest<RestoreSpace>, responses((status = 200, body = CommandResponse<SpaceSnapshot>)), security(("access_key" = [])))]
pub fn restore(
    tx: &mut impl SpaceStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RestoreSpace>,
) -> CommandResult<Execution<SpaceSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, p.expected),
        Change::Restore,
    )
}
