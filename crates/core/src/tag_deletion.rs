use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    tags::{TagSnapshot, TagStore},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveTagDeletion {
    pub space: String,
    pub tag: String,
    pub keep: bool,
    pub expected: StateVersion,
}
impl Command for ResolveTagDeletion {
    const NAME: &'static str = "tag.resolve_deletion";
    type Output = TagSnapshot;
}

#[utoipa::path(post, path = "/v1/commands/tag.resolve_deletion", operation_id = "tag_resolve_deletion", request_body = CommandRequest<ResolveTagDeletion>, responses((status = 200, body = CommandResponse<TagSnapshot>)), security(("access_key" = [])))]
pub fn resolve(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ResolveTagDeletion>,
) -> CommandResult<Execution<TagSnapshot>> {
    let p = &request.parameters;
    if request.operation != ResolveTagDeletion::NAME || !valid_id(&p.space) || !valid_id(&p.tag) {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
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
            match tx.space_snapshot(&p.space)?.state {
                SpaceState::Active => {}
                SpaceState::Conflicted => {
                    return Err(CommandError {
                        code: ErrorCode::Conflict,
                    });
                }
                SpaceState::Trashed => {
                    return Err(CommandError {
                        code: ErrorCode::InvalidRequest,
                    });
                }
            }
            let before = tx.tag_snapshot(&p.space, &p.tag)?;
            replay::require_current(p.expected, before.version)?;
            if !before.deletion_conflict {
                return Err(CommandError {
                    code: ErrorCode::Conflict,
                });
            }
            tx.resolve_tag_deletion(&p.space, &p.tag, p.keep)?;
            tx.tag_snapshot(&p.space, &p.tag)
        },
    )
}
