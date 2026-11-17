use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    cards::{CardSnapshot, CardStore},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    ordering::{CardPlacement, OrderStore},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    wire::{CommandRequest, CommandResponse},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PlacementChoice {
    pub placement: CardPlacement,
    pub before: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardPlacements {
    pub card: CardSnapshot,
    pub choices: Vec<PlacementChoice>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectCardPlacements {
    pub space: String,
    pub card: String,
}
impl Command for InspectCardPlacements {
    const NAME: &'static str = "card.inspect_placements";
    type Output = CardPlacements;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ResolveCardPlacement {
    pub space: String,
    pub card: String,
    pub placement: CardPlacement,
    pub expected: StateVersion,
}
impl Command for ResolveCardPlacement {
    const NAME: &'static str = "card.resolve_placement";
    type Output = CardSnapshot;
}
pub trait PlacementStore: OrderStore {
    fn resolve_card_placement(
        &mut self,
        space: &str,
        card: &str,
        place: &CardPlacement,
    ) -> CommandResult<()>;
}

fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn validate<C: Command>(request: &CommandRequest<C>, space: &str, card: &str) -> CommandResult<()> {
    if request.operation != C::NAME || !valid_id(space) || !valid_id(card) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
fn places(card: &CardSnapshot) -> Vec<CardPlacement> {
    if !card.placements.is_empty() {
        card.placements.clone()
    } else if card.columns.len() == 1 {
        card.positions
            .iter()
            .map(|rank| CardPlacement {
                column: card.columns[0].clone(),
                rank: rank.clone(),
            })
            .collect()
    } else {
        Vec::new()
    }
}

#[utoipa::path(post, path = "/v1/commands/card.inspect_placements", operation_id = "card_inspect_placements", request_body = CommandRequest<InspectCardPlacements>, responses((status = 200, body = CommandResponse<CardPlacements>)), security(("access_key" = [])))]
pub fn inspect(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectCardPlacements>,
) -> CommandResult<Execution<CardPlacements>> {
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
            let card = tx.card_snapshot(&p.space, &p.card)?;
            let cards = tx.cards(&p.space)?;
            let choices = places(&card)
                .into_iter()
                .map(|placement| {
                    let key = placement.rank.key(&card.card);
                    let before = cards
                        .iter()
                        .filter(|other| {
                            other.card != card.card
                                && other.deletion_groups.is_empty()
                                && other.columns == [placement.column.as_str()]
                                && other.positions.len() == 1
                                && other.placements.len() <= 1
                                && other.positions[0].key(&other.card) > key
                        })
                        .min_by_key(|other| (other.positions[0].key(&other.card), &other.card))
                        .map(|other| other.card.clone());
                    PlacementChoice { placement, before }
                })
                .collect();
            Ok(CardPlacements { card, choices })
        },
    )
}

#[utoipa::path(post, path = "/v1/commands/card.resolve_placement", operation_id = "card_resolve_placement", request_body = CommandRequest<ResolveCardPlacement>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn resolve(
    tx: &mut impl PlacementStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ResolveCardPlacement>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, &p.card)?;
    if !valid_id(&p.placement.column) || !p.placement.rank.valid() {
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
            let card = tx.card_snapshot(&p.space, &p.card)?;
            replay::require_current(p.expected, card.version)?;
            let variants = places(&card);
            if variants.len() < 2 || !card.deletion_groups.is_empty() {
                return Err(error(ErrorCode::Conflict));
            }
            if !variants.contains(&p.placement) {
                return Err(error(ErrorCode::InvalidRequest));
            }
            for state in [
                tx.space_snapshot(&p.space)?.state,
                tx.column_snapshot(&p.space, &p.placement.column)?.state,
            ] {
                match state {
                    SpaceState::Active => {}
                    SpaceState::Conflicted => return Err(error(ErrorCode::Conflict)),
                    SpaceState::Trashed => return Err(error(ErrorCode::InvalidRequest)),
                }
            }
            tx.resolve_card_placement(&p.space, &p.card, &p.placement)?;
            tx.card_snapshot(&p.space, &p.card)
        },
    )
}
