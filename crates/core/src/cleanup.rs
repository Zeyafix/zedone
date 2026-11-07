//! Explicit preview/confirm operations. Every scope is rebuilt inside the writer
//! transaction; the client cannot supply a deletion list or refreshed condition.
use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    cards::{CardSnapshot, CardStore, CardTextVersion},
    columns::ColumnSnapshot,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum TrashSelection {
    All,
    Space,
    Objects {
        columns: Vec<String>,
        cards: Vec<String>,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashPreview {
    pub space: String,
    pub selection: TrashSelection,
    pub includes_space: bool,
    pub columns: Vec<ColumnSnapshot>,
    pub cards: Vec<CardSnapshot>,
    /// Stable object identities to resolve before preparing a new action.
    pub conflicts: Vec<String>,
    pub version: StateVersion,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectTrash {
    pub space: String,
    pub selection: TrashSelection,
}
impl Command for InspectTrash {
    const NAME: &'static str = "trash.inspect";
    type Output = TrashPreview;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PurgeTrash {
    pub space: String,
    pub selection: TrashSelection,
    pub expected: StateVersion,
}
impl Command for PurgeTrash {
    const NAME: &'static str = "trash.purge";
    type Output = TrashPreview;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HistoryEntry {
    pub card: String,
    pub version: CardTextVersion,
    /// Conservatively retain history of every card with unresolved conflicts.
    pub retained: bool,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct HistoryPreview {
    pub space: String,
    /// None means every card in this space, including trash.
    pub card: Option<String>,
    pub entries: Vec<HistoryEntry>,
    /// Logical UTF-8 title/description bytes, not allocated database space.
    pub text_bytes: u64,
    pub removable_bytes: u64,
    pub version: StateVersion,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectHistory {
    pub space: String,
    pub card: Option<String>,
}
impl Command for InspectHistory {
    const NAME: &'static str = "history.inspect";
    type Output = HistoryPreview;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ClearHistory {
    pub space: String,
    pub card: Option<String>,
    pub expected: StateVersion,
}
impl Command for ClearHistory {
    const NAME: &'static str = "history.clear";
    type Output = HistoryPreview;
}

pub trait CleanupStore: CardStore {
    /// Trusted deterministic digest, using the existing storage token format.
    fn cleanup_version(&self, scope: &str, state: &impl Serialize) -> CommandResult<StateVersion>;
    fn purge_objects(&mut self, preview: &TrashPreview) -> CommandResult<()>;
    fn clear_versions(&mut self, space: &str, entries: &[HistoryEntry]) -> CommandResult<()>;
}
fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn validate<C: Command>(
    r: &CommandRequest<C>,
    space: &str,
    card: Option<&str>,
) -> CommandResult<()> {
    if r.operation != C::NAME || !valid_id(space) || card.is_some_and(|c| !valid_id(c)) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
fn required(space: &str, permission: SpacePermission) -> [RequiredAccess<'_>; 1] {
    [RequiredAccess::Space {
        id: space,
        permission,
    }]
}
fn trash_preview(
    tx: &mut impl CleanupStore,
    space: &str,
    selection: &TrashSelection,
) -> CommandResult<TrashPreview> {
    let parent = tx.space_snapshot(space)?;
    let contents = parent
        .contents
        .as_ref()
        .ok_or_else(|| error(ErrorCode::InvalidRequest))?;
    let (mut columns, mut cards) = (Vec::new(), Vec::new());
    let includes_space = matches!(selection, TrashSelection::Space)
        || matches!(selection, TrashSelection::All) && parent.state == SpaceState::Trashed;
    match selection {
        TrashSelection::Space => {
            if parent.state == SpaceState::Active {
                return Err(error(ErrorCode::InvalidRequest));
            }
            columns.extend(contents.columns.iter().cloned());
            cards.extend(contents.cards.iter().cloned());
        }
        TrashSelection::All => {
            columns.extend(
                contents
                    .columns
                    .iter()
                    .filter(|c| includes_space || c.state != SpaceState::Active)
                    .cloned(),
            );
            cards.extend(
                contents
                    .cards
                    .iter()
                    .filter(|c| includes_space || c.state != SpaceState::Active)
                    .cloned(),
            );
        }
        TrashSelection::Objects {
            columns: selected_columns,
            cards: selected_cards,
        } => {
            if selected_columns.is_empty() && selected_cards.is_empty() {
                return Err(error(ErrorCode::InvalidRequest));
            }
            for id in selected_columns {
                if !valid_id(id) || columns.iter().any(|c: &ColumnSnapshot| c.column == *id) {
                    return Err(error(ErrorCode::InvalidRequest));
                }
                let c = contents
                    .columns
                    .iter()
                    .find(|c| c.column == *id)
                    .ok_or_else(|| error(ErrorCode::NotFound))?;
                if c.state == SpaceState::Active {
                    return Err(error(ErrorCode::InvalidRequest));
                }
                columns.push(c.clone());
            }
            for id in selected_cards {
                if !valid_id(id) || cards.iter().any(|c: &CardSnapshot| c.card == *id) {
                    return Err(error(ErrorCode::InvalidRequest));
                }
                let c = contents
                    .cards
                    .iter()
                    .find(|c| c.card == *id)
                    .ok_or_else(|| error(ErrorCode::NotFound))?;
                if c.state == SpaceState::Active {
                    return Err(error(ErrorCode::InvalidRequest));
                }
                cards.push(c.clone());
            }
        }
    }
    // Include earlier trash and conflicts below selected containers. Never use
    // historical memberships to pull a restored/moved card into this scope.
    for card in &contents.cards {
        if card
            .columns
            .iter()
            .any(|id| columns.iter().any(|c| &c.column == id))
            && !cards.iter().any(|c| c.card == card.card)
        {
            cards.push(card.clone());
        }
    }
    columns.sort_by(|a, b| a.column.cmp(&b.column));
    cards.sort_by(|a, b| a.card.cmp(&b.card));
    let mut conflicts = Vec::new();
    if parent.state == SpaceState::Conflicted || includes_space && parent.names.len() != 1 {
        conflicts.push(format!("space:{space}"));
    }
    if includes_space
        && contents
            .tags
            .iter()
            .any(|t| t.state == crate::tags::TagState::Conflicted)
    {
        conflicts.extend(
            contents
                .tags
                .iter()
                .filter(|t| t.state == crate::tags::TagState::Conflicted)
                .map(|t| format!("tag:{}", t.tag)),
        );
    }
    for c in &columns {
        if c.state != SpaceState::Trashed
            || c.names.len() != 1
            || c.positions.len() != 1
            || c.contents.is_none()
        {
            conflicts.push(format!("column:{}", c.column));
        }
    }
    for c in &cards {
        if c.state != SpaceState::Trashed {
            conflicts.push(format!("card:{}", c.card));
        }
    }
    // Whole-space tokens include every tag and causal root; selected scopes use
    // only their actual objects plus enclosing lifecycle, without CRDT heads.
    let ids = cards.iter().map(|c| c.card.clone()).collect::<Vec<_>>();
    let histories = tx
        .card_histories(space, &ids)?
        .into_iter()
        .map(|h| (h.card, h.version))
        .collect::<Vec<_>>();
    let version = tx.cleanup_version(
        "trash.v1",
        &(
            space,
            selection,
            &parent.state,
            &parent.deletion_groups,
            includes_space.then_some(parent.version),
            &columns,
            &cards,
            histories,
            &conflicts,
        ),
    )?;
    Ok(TrashPreview {
        space: space.into(),
        selection: selection.clone(),
        includes_space,
        columns,
        cards,
        conflicts,
        version,
    })
}

#[utoipa::path(post, path = "/v1/commands/trash.inspect", operation_id = "trash_inspect", request_body = CommandRequest<InspectTrash>, responses((status = 200, body = CommandResponse<TrashPreview>)), security(("access_key" = [])))]
pub fn inspect_trash(
    tx: &mut impl CleanupStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<InspectTrash>,
) -> CommandResult<Execution<TrashPreview>> {
    let p = &r.parameters;
    validate(r, &p.space, None)?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::Read),
        r.id,
        p,
        |tx| trash_preview(tx, &p.space, &p.selection),
    )
}
#[utoipa::path(post, path = "/v1/commands/trash.purge", operation_id = "trash_purge", request_body = CommandRequest<PurgeTrash>, responses((status = 200, body = CommandResponse<TrashPreview>)), security(("access_key" = [])))]
pub fn purge_trash(
    tx: &mut impl CleanupStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<PurgeTrash>,
) -> CommandResult<Execution<TrashPreview>> {
    let p = &r.parameters;
    validate(r, &p.space, None)?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::ReadWrite),
        r.id,
        p,
        |tx| {
            let preview = trash_preview(tx, &p.space, &p.selection)?;
            replay::require_current(p.expected, preview.version)?;
            if !preview.conflicts.is_empty() {
                return Err(error(ErrorCode::Conflict));
            }
            tx.purge_objects(&preview)?;
            // This is the confirmed removed scope, not a post-purge current view.
            Ok(preview)
        },
    )
}
fn history_preview(
    tx: &mut impl CleanupStore,
    space: &str,
    card: Option<&str>,
    now_ms: i64,
) -> CommandResult<HistoryPreview> {
    let parent = tx.space_snapshot(space)?;
    if card.is_none() && parent.contents.is_none() {
        return Err(error(ErrorCode::InvalidRequest));
    }
    let mut cards = match card {
        Some(id) => vec![tx.card_snapshot(space, id)?],
        None => tx.cards(space)?,
    };
    cards.sort_by(|a, b| a.card.cmp(&b.card));
    for c in &cards {
        tx.settle_card_history(space, &c.card, now_ms)?;
    }
    let ids = cards.iter().map(|c| c.card.clone()).collect::<Vec<_>>();
    let histories = tx.card_histories(space, &ids)?;
    let mut entries = Vec::new();
    let mut states = Vec::new();
    let (mut text_bytes, mut removable_bytes) = (0, 0);
    for (c, h) in cards.into_iter().zip(histories) {
        states.push((c.card.clone(), c.version, h.version));
        let retained = c.state == SpaceState::Conflicted
            || parent.state == SpaceState::Conflicted
            || parent.contents.as_ref().is_some_and(|contents| {
                contents.columns.iter().any(|column| {
                    c.columns.contains(&column.column)
                        && (column.state == SpaceState::Conflicted
                            || column.names.len() != 1
                            || column.positions.len() != 1)
                })
            });
        for version in h.versions {
            let bytes = (version.title.len() + version.description.len()) as u64;
            text_bytes += bytes;
            if !retained {
                removable_bytes += bytes;
            }
            entries.push(HistoryEntry {
                card: c.card.clone(),
                version,
                retained,
            });
        }
    }
    let version = tx.cleanup_version(
        "history.clear.v1",
        &(
            space,
            card,
            &parent.state,
            &parent.deletion_groups,
            states,
            &entries,
        ),
    )?;
    Ok(HistoryPreview {
        space: space.into(),
        card: card.map(str::to_owned),
        entries,
        text_bytes,
        removable_bytes,
        version,
    })
}
#[utoipa::path(post, path = "/v1/commands/history.inspect", operation_id = "history_inspect", request_body = CommandRequest<InspectHistory>, responses((status = 200, body = CommandResponse<HistoryPreview>)), security(("access_key" = [])))]
pub fn inspect_history(
    tx: &mut impl CleanupStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<InspectHistory>,
) -> CommandResult<Execution<HistoryPreview>> {
    let p = &r.parameters;
    validate(r, &p.space, p.card.as_deref())?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::Read),
        r.id,
        p,
        |tx| {
            history_preview(
                tx,
                &p.space,
                p.card.as_deref(),
                crate::card_history::now_ms(clock)?,
            )
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/history.clear", operation_id = "history_clear", request_body = CommandRequest<ClearHistory>, responses((status = 200, body = CommandResponse<HistoryPreview>)), security(("access_key" = [])))]
pub fn clear_history(
    tx: &mut impl CleanupStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<ClearHistory>,
) -> CommandResult<Execution<HistoryPreview>> {
    let p = &r.parameters;
    validate(r, &p.space, p.card.as_deref())?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::ReadWrite),
        r.id,
        p,
        |tx| {
            let preview = history_preview(
                tx,
                &p.space,
                p.card.as_deref(),
                crate::card_history::now_ms(clock)?,
            )?;
            replay::require_current(p.expected, preview.version)?;
            tx.clear_versions(&p.space, &preview.entries)?;
            Ok(preview)
        },
    )
}
