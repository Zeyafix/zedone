//! Read-only domain search. Receipts and authority use the ordinary command path.
use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission, with_authorized},
    cards::{CardSnapshot, CardStore},
    columns::ColumnSnapshot,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    ordering::RankPart,
    replay::{self, Execution},
    spaces::{SpaceSnapshot, SpaceState, sort_spaces, valid_id},
    text::comparison_key,
    wire::{CommandRequest, CommandResponse},
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SearchScope {
    Space(SearchSpaceScope),
    All(SearchAllScope),
}

// Strict structs keep Serde and generated variant schemas identical.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchSpaceScope {
    pub kind: SearchSpaceKind,
    pub space: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchSpaceKind {
    Space,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchAllScope {
    pub kind: SearchAllKind,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SearchAllKind {
    All,
}
impl SearchScope {
    pub fn space(space: String) -> Self {
        Self::Space(SearchSpaceScope {
            kind: SearchSpaceKind::Space,
            space,
        })
    }
    pub fn all() -> Self {
        Self::All(SearchAllScope {
            kind: SearchAllKind::All,
        })
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchCards {
    pub query: String,
    /// Hosts map the active space to Space; clearing query never changes scope.
    pub scope: SearchScope,
}
impl Command for SearchCards {
    const NAME: &'static str = "card.search";
    type Output = SearchResults;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchHit {
    pub card: CardSnapshot,
    /// These hits precede ordinary board positions inside their space.
    pub placement_conflict: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchSpace {
    /// All current names; len > 1 identifies the leading name-conflict group.
    pub space: SpaceSnapshot,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SearchResults {
    /// Only spaces containing matches, in the same order as space.list.
    pub spaces: Vec<SearchSpace>,
}

fn matches(card: &CardSnapshot, needle: &str) -> bool {
    needle.is_empty()
        || card
            .titles
            .iter()
            .chain(&card.descriptions)
            .any(|value| comparison_key(value).contains(needle))
}

type BoardKey = (u8, Vec<RankPart>, String, Vec<RankPart>, String);
fn board_key(
    space: &SpaceSnapshot,
    card: &CardSnapshot,
    columns: &BTreeMap<String, ColumnSnapshot>,
) -> CommandResult<Option<BoardKey>> {
    if card.state == SpaceState::Trashed {
        return Ok(None);
    }
    let parents = card
        .columns
        .iter()
        .map(|id| {
            columns.get(id).ok_or(CommandError {
                code: ErrorCode::Unavailable,
            })
        })
        .collect::<CommandResult<Vec<_>>>()?;
    if parents.is_empty() {
        return Err(CommandError {
            code: ErrorCode::Unavailable,
        });
    }
    if parents.iter().all(|c| c.state == SpaceState::Trashed) {
        return Ok(None);
    }
    let conflicted = card.columns.len() != 1
        || card.positions.len() != 1
        || card.placements.len() > 1
        || !card.deletion_groups.is_empty()
        || !space.deletion_groups.is_empty()
        || parents
            .iter()
            .any(|c| c.positions.len() != 1 || !c.deletion_groups.is_empty());
    if conflicted {
        // Stable ID order; never choose a disputed column or rank.
        Ok(Some((0, vec![], String::new(), vec![], card.card.clone())))
    } else {
        let column = parents[0];
        Ok(Some((
            1,
            column.positions[0].key(&column.column),
            column.column.clone(),
            card.positions[0].key(&card.card),
            card.card.clone(),
        )))
    }
}

#[utoipa::path(post, path = "/v1/commands/card.search", operation_id = "card_search", request_body = CommandRequest<SearchCards>, responses((status = 200, body = CommandResponse<SearchResults>)), security(("access_key" = [])))]
pub fn search(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<SearchCards>,
) -> CommandResult<Execution<SearchResults>> {
    if request.operation != SearchCards::NAME
        || matches!(&request.parameters.scope, SearchScope::Space(scope) if !valid_id(&scope.space))
    {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    let required = match &request.parameters.scope {
        SearchScope::Space(scope) => RequiredAccess::Space {
            id: &scope.space,
            permission: SpacePermission::Read,
        },
        SearchScope::All(_) => RequiredAccess::ActiveGrant,
    };
    let execution = replay::execute_as(
        tx,
        credential,
        clock,
        &[required],
        request.id,
        &request.parameters,
        |tx, subject| {
            let mut spaces = match &request.parameters.scope {
                SearchScope::Space(scope) => vec![tx.space_snapshot(&scope.space)?],
                SearchScope::All(_) => tx.spaces_for_subject(subject)?,
            };
            spaces.retain(|s| s.state != SpaceState::Trashed);
            sort_spaces(&mut spaces);
            let needle = comparison_key(request.parameters.query.trim());
            let mut results = Vec::new();
            for space in spaces {
                let columns = tx
                    .columns(&space.space)?
                    .into_iter()
                    .map(|c| (c.column.clone(), c))
                    .collect();
                let mut hits = Vec::new();
                for card in tx.cards(&space.space)? {
                    if matches(&card, &needle)
                        && let Some(key) = board_key(&space, &card, &columns)?
                    {
                        hits.push((
                            key.clone(),
                            SearchHit {
                                card,
                                placement_conflict: key.0 == 0,
                            },
                        ));
                    }
                }
                hits.sort_by(|a, b| a.0.cmp(&b.0));
                if !hits.is_empty() {
                    results.push(SearchSpace {
                        space,
                        hits: hits.into_iter().map(|(_, hit)| hit).collect(),
                    });
                }
            }
            Ok(SearchResults { spaces: results })
        },
    )?;
    // A receipt contains names and card texts. Recheck every recorded space if
    // a grant was narrowed since the original all-space search.
    if execution.replayed {
        let required: Vec<_> = execution
            .result
            .spaces
            .iter()
            .map(|s| RequiredAccess::Space {
                id: &s.space.space,
                permission: SpacePermission::Read,
            })
            .collect();
        if !required.is_empty() {
            with_authorized(tx, credential, &required, clock, |_, _| Ok(()))?;
        }
    }
    Ok(execution)
}
