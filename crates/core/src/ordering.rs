//! Relative placement shared by every transport. Clients send identities, never ranks.
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    cards::{CardSnapshot, CardStore},
    columns::ColumnSnapshot,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    wire::{CommandRequest, CommandResponse},
};

/// Legacy integer positions remain valid in historical documents and receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum Rank {
    Legacy(u64),
    Path(Vec<RankPart>),
}
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RankPart {
    pub digit: u32,
    pub id: String,
}
// Once the upper bound has diverged, use spare digits before extending a path.
// Repeated appends must not add one component per object.
fn after(left: &[RankPart], id: &str) -> Vec<RankPart> {
    let mut path = Vec::new();
    for part in left {
        if let Some(digit) = part.digit.checked_add(1) {
            path.push(RankPart {
                digit,
                id: id.into(),
            });
            return path;
        }
        path.push(part.clone());
    }
    path.push(RankPart {
        digit: u32::MAX / 2,
        id: id.into(),
    });
    path
}
impl Rank {
    pub fn key(&self, id: &str) -> Vec<RankPart> {
        match self {
            Self::Legacy(digit) => vec![
                RankPart {
                    digit: (*digit >> 32) as u32,
                    id: String::new(),
                },
                RankPart {
                    digit: *digit as u32,
                    id: id.into(),
                },
            ],
            Self::Path(path) => path.clone(),
        }
    }
    pub fn valid(&self) -> bool {
        match self {
            Self::Legacy(_) => true,
            Self::Path(path) => {
                !path.is_empty()
                    && path.iter().all(|p| valid_id(&p.id) || p.id.is_empty())
                    && path.last().is_some_and(|p| valid_id(&p.id))
            }
        }
    }
    /// Dense lexicographic path; insertion never rewrites another object.
    pub fn between(
        left: Option<&[RankPart]>,
        right: Option<&[RankPart]>,
        id: &str,
    ) -> CommandResult<Self> {
        if !valid_id(id) || left.zip(right).is_some_and(|(l, r)| l >= r) {
            return Err(error(ErrorCode::InvalidRequest));
        }
        let l = left.unwrap_or_default();
        let r = right.unwrap_or_default();
        let mut path = Vec::new();
        let mut i = 0;
        loop {
            match (l.get(i), r.get(i)) {
                (Some(a), Some(b)) if a == b => path.push(a.clone()),
                (None, Some(b)) if b.digit == 0 && b.id.is_empty() => path.push(b.clone()),
                (a, Some(b)) if b.digit.saturating_sub(a.map_or(0, |a| a.digit)) > 1 => {
                    let low = a.map_or(0, |a| a.digit);
                    path.push(RankPart {
                        digit: low + (b.digit - low) / 2,
                        id: id.into(),
                    });
                    break;
                }
                (None, Some(_)) => {
                    path.push(RankPart {
                        digit: 0,
                        id: String::new(),
                    });
                    path.push(RankPart {
                        digit: u32::MAX / 2,
                        id: id.into(),
                    });
                    break;
                }
                _ => {
                    if r.get(i).is_some()
                        && let Some(part) = l.get(i)
                    {
                        // Preserve the lower divergent component to stay below
                        // the upper bound, then allocate in its free suffix.
                        path.push(part.clone());
                        path.extend(after(&l[i + 1..], id));
                    } else {
                        path.extend(after(&l[i.min(l.len())..], id));
                    }
                    break;
                }
            }
            i += 1;
        }
        if left.is_some_and(|l| l >= path.as_slice()) || right.is_some_and(|r| path.as_slice() >= r)
        {
            return Err(error(ErrorCode::InvalidRequest));
        }
        Ok(Self::Path(path))
    }
}
/// A whole register: concurrent moves cannot mix a column with another rank.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardPlacement {
    pub column: String,
    pub rank: Rank,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveCard {
    pub space: String,
    pub card: String,
    pub column: String,
    /// Insert before this active card in the destination; null appends.
    pub before: Option<String>,
    pub expected: StateVersion,
}
impl Command for MoveCard {
    const NAME: &'static str = "card.move";
    type Output = CardSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct MoveColumn {
    pub space: String,
    pub column: String,
    /// Insert before this active column; null appends.
    pub before: Option<String>,
    pub expected: StateVersion,
}
impl Command for MoveColumn {
    const NAME: &'static str = "column.move";
    type Output = ColumnSnapshot;
}

pub trait OrderStore: CardStore {
    fn place_card(&mut self, space: &str, card: &str, place: &CardPlacement) -> CommandResult<()>;
    fn place_column(&mut self, space: &str, column: &str, rank: &Rank) -> CommandResult<()>;
}
fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn active(state: SpaceState) -> CommandResult<()> {
    match state {
        SpaceState::Active => Ok(()),
        SpaceState::Conflicted => Err(error(ErrorCode::Conflict)),
        SpaceState::Trashed => Err(error(ErrorCode::InvalidRequest)),
    }
}
fn validate(space: &str, target: &str, before: Option<&str>) -> CommandResult<()> {
    if !valid_id(space) || !valid_id(target) || before.is_some_and(|s| !valid_id(s) || s == target)
    {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
/// Resolve the relative intention in the current serialized transaction. Other
/// cards may change without invalidating the target's own causal version.
fn rank(
    items: Vec<(String, Rank)>,
    target: &str,
    before: Option<&str>,
    same_parent: bool,
) -> CommandResult<Option<Rank>> {
    let mut items: Vec<_> = items
        .into_iter()
        .map(|(id, rank)| (rank.key(&id), id))
        .collect();
    items.sort();
    let current_next = items
        .iter()
        .position(|(_, id)| id == target)
        .map(|i| items.get(i + 1).map(|(_, id)| id.as_str()));
    if same_parent && current_next == Some(before) {
        return Ok(None);
    }
    items.retain(|(_, id)| id != target);
    let index = match before {
        None => items.len(),
        Some(id) => items
            .iter()
            .position(|(_, item)| item == id)
            .ok_or_else(|| error(ErrorCode::InvalidRequest))?,
    };
    Rank::between(
        index.checked_sub(1).map(|i| items[i].0.as_slice()),
        items.get(index).map(|v| v.0.as_slice()),
        target,
    )
    .map(Some)
}

#[utoipa::path(post, path = "/v1/commands/card.move", operation_id = "card_move", request_body = CommandRequest<MoveCard>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn move_card(
    tx: &mut impl OrderStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<MoveCard>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    validate(&p.space, &p.card, p.before.as_deref())?;
    if request.operation != MoveCard::NAME || !valid_id(&p.column) {
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
            active(card.state)?;
            active(tx.space_snapshot(&p.space)?.state)?;
            active(tx.column_snapshot(&p.space, &card.columns[0])?.state)?;
            active(tx.column_snapshot(&p.space, &p.column)?.state)?;
            let mut items = Vec::new();
            for c in tx.cards(&p.space)? {
                if c.columns == [p.column.as_str()]
                    && c.positions.len() == 1
                    && c.placements.len() <= 1
                    && c.deletion_groups.is_empty()
                {
                    items.push((c.card, c.positions[0].clone()));
                }
            }
            if let Some(rank) = rank(
                items,
                &p.card,
                p.before.as_deref(),
                card.columns == [p.column.as_str()],
            )? {
                tx.place_card(
                    &p.space,
                    &p.card,
                    &CardPlacement {
                        column: p.column.clone(),
                        rank,
                    },
                )?;
            }
            tx.card_snapshot(&p.space, &p.card)
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/column.move", operation_id = "column_move", request_body = CommandRequest<MoveColumn>, responses((status = 200, body = CommandResponse<ColumnSnapshot>)), security(("access_key" = [])))]
pub fn move_column(
    tx: &mut impl OrderStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<MoveColumn>,
) -> CommandResult<Execution<ColumnSnapshot>> {
    let p = &request.parameters;
    validate(&p.space, &p.column, p.before.as_deref())?;
    if request.operation != MoveColumn::NAME {
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
            active(column.state)?;
            if column.names.len() != 1 || column.positions.len() != 1 {
                return Err(error(ErrorCode::Conflict));
            }
            active(tx.space_snapshot(&p.space)?.state)?;
            let mut items = Vec::new();
            for c in tx.columns(&p.space)? {
                if c.positions.len() == 1
                    && c.deletion_groups.is_empty()
                    && c.state != SpaceState::Trashed
                {
                    items.push((c.column, c.positions[0].clone()));
                }
            }
            if let Some(rank) = rank(items, &p.column, p.before.as_deref(), true)? {
                tx.place_column(&p.space, &p.column, &rank)?;
            }
            tx.column_snapshot(&p.space, &p.column)
        },
    )
}

#[cfg(test)]
mod tests;
