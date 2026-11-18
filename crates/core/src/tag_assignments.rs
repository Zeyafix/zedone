use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    cards::CardSnapshot,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    tags::{SetCardTag, TagState, TagStore},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TagAssignment {
    pub tag: String,
    pub values: Vec<bool>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardTags {
    pub card: CardSnapshot,
    pub assignments: Vec<TagAssignment>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectCardTags {
    pub space: String,
    pub card: String,
}
impl Command for InspectCardTags {
    const NAME: &'static str = "card.inspect_tags";
    type Output = CardTags;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveCardTag {
    pub space: String,
    pub card: String,
    pub tag: String,
    pub present: bool,
    pub expected: StateVersion,
    pub expected_tag: StateVersion,
}
impl Command for ResolveCardTag {
    const NAME: &'static str = "card.resolve_tag";
    type Output = CardSnapshot;
}
fn invalid() -> CommandError {
    CommandError {
        code: ErrorCode::InvalidRequest,
    }
}
fn validate<C: Command>(request: &CommandRequest<C>, space: &str, card: &str) -> CommandResult<()> {
    if request.operation != C::NAME || !valid_id(space) || !valid_id(card) {
        return Err(invalid());
    }
    Ok(())
}
#[utoipa::path(post, path = "/v1/commands/card.inspect_tags", operation_id = "card_inspect_tags", request_body = CommandRequest<InspectCardTags>, responses((status = 200, body = CommandResponse<CardTags>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectCardTags>,
) -> CommandResult<Execution<CardTags>> {
    let p = &request.parameters;
    validate(request, &p.space, &p.card)?;
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
            Ok(CardTags {
                card: tx.card_snapshot(&p.space, &p.card)?,
                assignments: tx.card_tag_assignments(&p.space, &p.card)?,
            })
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/card.resolve_tag", operation_id = "card_resolve_tag", request_body = CommandRequest<ResolveCardTag>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn resolve(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ResolveCardTag>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, &p.card)?;
    if !valid_id(&p.tag) {
        return Err(invalid());
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
            let card = tx.card_snapshot(&p.space, &p.card)?;
            replay::require_current(p.expected, card.version)?;
            let tag = tx.tag_snapshot(&p.space, &p.tag)?;
            replay::require_current(p.expected_tag, tag.version)?;
            if tag.state != TagState::Active
                || !card.deletion_groups.is_empty()
                || !tx
                    .card_tag_assignments(&p.space, &p.card)?
                    .iter()
                    .any(|a| a.tag == tag.tag && a.values == [false, true])
            {
                return Err(CommandError {
                    code: ErrorCode::Conflict,
                });
            }
            for state in std::iter::once(tx.space_snapshot(&p.space)?.state).chain(
                card.columns
                    .iter()
                    .map(|column| tx.column_snapshot(&p.space, column).map(|c| c.state))
                    .collect::<CommandResult<Vec<_>>>()?,
            ) {
                match state {
                    SpaceState::Active => {}
                    SpaceState::Conflicted => {
                        return Err(CommandError {
                            code: ErrorCode::Conflict,
                        });
                    }
                    SpaceState::Trashed => return Err(invalid()),
                }
            }
            tx.set_card_tag(&SetCardTag {
                space: p.space.clone(),
                card: p.card.clone(),
                tag: p.tag.clone(),
                present: p.present,
                expected: p.expected,
                expected_tag: p.expected_tag,
            })?;
            tx.card_snapshot(&p.space, &p.card)
        },
    )
}
