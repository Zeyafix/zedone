use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    tags::{TagList, TagState, TagStore, name_key},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TagNameChoice {
    pub tag: String,
    pub name: String,
    pub expected: StateVersion,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveTagNames {
    pub space: String,
    pub tag: String,
    pub names: Vec<TagNameChoice>,
}
impl Command for ResolveTagNames {
    const NAME: &'static str = "tag.resolve_names";
    type Output = TagList;
}
fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}

#[utoipa::path(post, path = "/v1/commands/tag.resolve_names", operation_id = "tag_resolve_names", request_body = CommandRequest<ResolveTagNames>, responses((status = 200, body = CommandResponse<TagList>)), security(("access_key" = [])))]
pub fn resolve(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ResolveTagNames>,
) -> CommandResult<Execution<TagList>> {
    let p = &request.parameters;
    if request.operation != ResolveTagNames::NAME
        || !valid_id(&p.space)
        || !valid_id(&p.tag)
        || p.names.is_empty()
        || p.names
            .iter()
            .any(|n| !valid_id(&n.tag) || n.name.trim().is_empty())
    {
        return Err(error(ErrorCode::InvalidRequest));
    }
    let ids: BTreeSet<_> = p.names.iter().map(|n| n.tag.as_str()).collect();
    let keys: BTreeSet<_> = p.names.iter().map(|n| name_key(&n.name)).collect();
    if ids.len() != p.names.len() || keys.len() != p.names.len() {
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
            match tx.space_snapshot(&p.space)?.state {
                SpaceState::Active => {}
                SpaceState::Conflicted => return Err(error(ErrorCode::Conflict)),
                SpaceState::Trashed => return Err(error(ErrorCode::InvalidRequest)),
            }
            let group = tx.tag_name_group(&p.space, &p.tag)?;
            if group
                .iter()
                .map(|t| t.tag.as_str())
                .collect::<BTreeSet<_>>()
                != ids
            {
                return Err(error(ErrorCode::InvalidRequest));
            }
            for choice in &p.names {
                let before = group
                    .iter()
                    .find(|t| t.tag == choice.tag)
                    .ok_or_else(|| error(ErrorCode::InvalidRequest))?;
                replay::require_current(choice.expected, before.version)?;
            }
            if !group.iter().any(|t| t.state == TagState::Conflicted) {
                return Err(error(ErrorCode::Conflict));
            }
            if tx.tags(&p.space)?.iter().any(|t| {
                !ids.contains(t.tag.as_str())
                    && t.state != TagState::Deleted
                    && t.names.iter().any(|n| keys.contains(&name_key(n)))
            }) {
                return Err(error(ErrorCode::Conflict));
            }
            tx.resolve_tag_names(&p.space, &p.names)?;
            Ok(TagList {
                tags: p
                    .names
                    .iter()
                    .map(|n| tx.tag_snapshot(&p.space, &n.tag))
                    .collect::<CommandResult<_>>()?,
            })
        },
    )
}
