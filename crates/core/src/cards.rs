//! Card policy shared by every transport, inside the authorized transaction.
use crate::card_history::{
    self, AutosaveCard, BeginCardEditing, FinishCardEditing, RestoreCardText,
};
use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    columns::ColumnStore,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardTextVersion {
    pub id: String,
    pub subject: String,
    /// Local causal sequence; independent branches may share a sequence.
    pub sequence: u64,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardSnapshot {
    pub space: String,
    pub card: String,
    pub titles: Vec<String>,
    pub descriptions: Vec<String>,
    pub columns: Vec<String>,
    pub positions: Vec<crate::ordering::Rank>,
    /// Whole variants after a move; legacy documents and receipts omit these.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub placements: Vec<crate::ordering::CardPlacement>,
    pub completed: Vec<bool>,
    pub tags: Vec<String>,
    pub state: SpaceState,
    pub deletion_groups: Vec<String>,
    pub version: StateVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardHistory {
    pub space: String,
    pub card: String,
    pub versions: Vec<CardTextVersion>,
    pub next: Option<String>,
    pub version: StateVersion,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadCardHistory {
    pub space: String,
    pub card: String,
    pub after: Option<String>,
    pub limit: u16,
    pub expected: Option<StateVersion>,
}
impl Command for ReadCardHistory {
    const NAME: &'static str = "card.history";
    type Output = CardHistory;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CardList {
    pub cards: Vec<CardSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateCard {
    pub space: String,
    pub card: String,
    pub column: String,
    pub title: String,
    pub description: String,
}
impl Command for CreateCard {
    const NAME: &'static str = "card.create";
    type Output = CardSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadCard {
    pub space: String,
    pub card: String,
}
impl Command for ReadCard {
    const NAME: &'static str = "card.read";
    type Output = CardSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListCards {
    pub space: String,
    pub include_trashed: bool,
}
impl Command for ListCards {
    const NAME: &'static str = "card.list";
    type Output = CardList;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct EditCard {
    pub space: String,
    pub card: String,
    pub title: String,
    pub description: String,
    pub expected: StateVersion,
}
impl Command for EditCard {
    const NAME: &'static str = "card.edit";
    type Output = CardSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetCardCompleted {
    pub space: String,
    pub card: String,
    pub completed: bool,
    pub expected: StateVersion,
}
impl Command for SetCardCompleted {
    const NAME: &'static str = "card.set_completed";
    type Output = CardSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TrashCard {
    pub space: String,
    pub card: String,
    pub expected: StateVersion,
}
impl Command for TrashCard {
    const NAME: &'static str = "card.trash";
    type Output = CardSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreCard {
    pub space: String,
    pub card: String,
    pub expected: StateVersion,
}
impl Command for RestoreCard {
    const NAME: &'static str = "card.restore";
    type Output = CardSnapshot;
}

pub trait CardStore: ColumnStore {
    fn begin_card_editing(
        &mut self,
        space: &str,
        card: &str,
        subject: &str,
        session: &str,
    ) -> CommandResult<()>;
    fn autosave_card(
        &mut self,
        parameters: &AutosaveCard,
        subject: &str,
        command: &str,
        now_ms: i64,
    ) -> CommandResult<()>;
    fn finish_card_editing(
        &mut self,
        space: &str,
        card: &str,
        subject: &str,
        session: &str,
    ) -> CommandResult<()>;
    fn settle_card_history(&mut self, space: &str, card: &str, now_ms: i64) -> CommandResult<()>;
    fn restore_card_text(
        &mut self,
        space: &str,
        card: &str,
        version: &str,
        subject: &str,
        command: &str,
    ) -> CommandResult<()>;
    fn create_card(
        &mut self,
        parameters: &CreateCard,
        subject: &str,
        command: &str,
    ) -> CommandResult<CardSnapshot>;
    fn card_snapshot(&mut self, space: &str, card: &str) -> CommandResult<CardSnapshot>;
    fn cards(&mut self, space: &str) -> CommandResult<Vec<CardSnapshot>>;
    fn card_history(&mut self, space: &str, card: &str) -> CommandResult<CardHistory>;
    fn edit_card(
        &mut self,
        parameters: &EditCard,
        subject: &str,
        command: &str,
    ) -> CommandResult<()>;
    fn set_card_lifecycle(&mut self, space: &str, card: &str, lifecycle: &str)
    -> CommandResult<()>;
    fn set_card_completed(&mut self, space: &str, card: &str, completed: bool)
    -> CommandResult<()>;
}
fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn validate<C: Command>(
    request: &CommandRequest<C>,
    space: &str,
    card: Option<&str>,
) -> CommandResult<()> {
    if request.operation != C::NAME || !valid_id(space) || card.is_some_and(|s| !valid_id(s)) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
fn title(value: &str) -> CommandResult<()> {
    if value.trim().is_empty() {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
fn active(state: SpaceState) -> CommandResult<()> {
    match state {
        SpaceState::Active => Ok(()),
        SpaceState::Conflicted => Err(error(ErrorCode::Conflict)),
        SpaceState::Trashed => Err(error(ErrorCode::InvalidRequest)),
    }
}
pub(crate) fn parents(tx: &mut impl CardStore, space: &str, column: &str) -> CommandResult<()> {
    active(tx.space_snapshot(space)?.state)?;
    active(tx.column_snapshot(space, column)?.state)
}
fn required(space: &str, permission: SpacePermission) -> [RequiredAccess<'_>; 1] {
    [RequiredAccess::Space {
        id: space,
        permission,
    }]
}
fn command_id(id: &[u8; 16]) -> String {
    id.iter().map(|b| format!("{b:02x}")).collect()
}

#[utoipa::path(post, path = "/v1/commands/card.create", operation_id = "card_create", request_body = CommandRequest<CreateCard>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn create(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CreateCard>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.card))?;
    if !valid_id(&p.column) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    title(&p.title)?;
    replay::execute_as(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx, subject| {
            parents(tx, &p.space, &p.column)?;
            tx.create_card(p, subject, &command_id(&request.id))
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/card.read", operation_id = "card_read", request_body = CommandRequest<ReadCard>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn read(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ReadCard>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.card))?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::Read),
        request.id,
        p,
        |tx| tx.card_snapshot(&p.space, &p.card),
    )
}

#[utoipa::path(post, path = "/v1/commands/card.history", operation_id = "card_history", request_body = CommandRequest<ReadCardHistory>, responses((status = 200, body = CommandResponse<CardHistory>)), security(("access_key" = [])))]
pub fn history(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ReadCardHistory>,
) -> CommandResult<Execution<CardHistory>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.card))?;
    if !(1..=100).contains(&p.limit) || (p.after.is_some() && p.expected.is_none()) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::Read),
        request.id,
        p,
        |tx| {
            tx.settle_card_history(&p.space, &p.card, card_history::now_ms(clock)?)?;
            let mut history = tx.card_history(&p.space, &p.card)?;
            if let Some(expected) = p.expected {
                replay::require_current(expected, history.version)?;
            }
            let start = match &p.after {
                None => 0,
                Some(after) => {
                    history
                        .versions
                        .iter()
                        .position(|v| &v.id == after)
                        .ok_or_else(|| error(ErrorCode::InvalidRequest))?
                        + 1
                }
            };
            let end = (start + usize::from(p.limit)).min(history.versions.len());
            history.next = if end < history.versions.len() {
                Some(history.versions[end - 1].id.clone())
            } else {
                None
            };
            history.versions = history.versions.drain(start..end).collect();
            Ok(history)
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/card.list", operation_id = "card_list", request_body = CommandRequest<ListCards>, responses((status = 200, body = CommandResponse<CardList>)), security(("access_key" = [])))]
pub fn list(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ListCards>,
) -> CommandResult<Execution<CardList>> {
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
            let mut cards = tx.cards(&p.space)?;
            cards.retain(|c| p.include_trashed || c.state != SpaceState::Trashed);
            cards.sort_by(|a, b| {
                a.columns
                    .cmp(&b.columns)
                    .then(
                        a.positions
                            .iter()
                            .map(|r| r.key(&a.card))
                            .cmp(b.positions.iter().map(|r| r.key(&b.card))),
                    )
                    .then(a.card.cmp(&b.card))
            });
            Ok(CardList { cards })
        },
    )
}
enum Change<'a> {
    BeginEditing,
    Autosave(&'a AutosaveCard),
    FinishEditing(&'a str),
    RestoreText(&'a str),
    Edit(&'a EditCard),
    SetCompleted(bool),
    Trash,
    Restore,
}
fn change<C: Command<Output = CardSnapshot>>(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<C>,
    target: (&str, &str, StateVersion),
    action: Change<'_>,
) -> CommandResult<Execution<CardSnapshot>> {
    let (space, card, expected) = target;
    validate(request, space, Some(card))?;
    replay::execute_as(
        tx,
        credential,
        clock,
        &required(space, SpacePermission::ReadWrite),
        request.id,
        &request.parameters,
        |tx, subject| {
            let before = tx.card_snapshot(space, card)?;
            replay::require_current(expected, before.version)?;
            if before.state == SpaceState::Conflicted
                || before.titles.len() != 1
                || before.descriptions.len() != 1
                || before.columns.len() != 1
                || before.positions.len() != 1
                || before.completed.len() != 1
            {
                return Err(error(ErrorCode::Conflict));
            }
            parents(tx, space, &before.columns[0])?;
            let command = command_id(&request.id);
            match action {
                Change::BeginEditing if before.state == SpaceState::Active => {
                    tx.begin_card_editing(space, card, subject, &command)?;
                }
                Change::Autosave(p) if before.state == SpaceState::Active => {
                    tx.autosave_card(p, subject, &command, card_history::now_ms(clock)?)?;
                }
                Change::FinishEditing(session) if before.state == SpaceState::Active => {
                    tx.finish_card_editing(space, card, subject, session)?;
                }
                Change::RestoreText(version) if before.state == SpaceState::Active => {
                    tx.restore_card_text(space, card, version, subject, &command)?;
                }
                Change::Edit(p) if before.state == SpaceState::Active => {
                    tx.edit_card(p, subject, &command)?
                }
                Change::SetCompleted(completed) if before.state == SpaceState::Active => {
                    if before.completed != [completed] {
                        tx.set_card_completed(space, card, completed)?;
                    }
                }
                Change::Trash if before.state == SpaceState::Active => {
                    tx.set_card_lifecycle(space, card, &format!("trash:{subject}:{command}"))?
                }
                Change::Restore if before.state == SpaceState::Trashed => {
                    tx.set_card_lifecycle(space, card, "active")?
                }
                _ => return Err(error(ErrorCode::InvalidRequest)),
            }
            tx.card_snapshot(space, card)
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/card.edit", operation_id = "card_edit", request_body = CommandRequest<EditCard>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn edit(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<EditCard>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    title(&p.title)?;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.card, p.expected),
        Change::Edit(p),
    )
}
#[utoipa::path(post, path = "/v1/commands/card.set_completed", operation_id = "card_set_completed", request_body = CommandRequest<SetCardCompleted>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn set_completed(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<SetCardCompleted>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.card, p.expected),
        Change::SetCompleted(p.completed),
    )
}
#[utoipa::path(post, path = "/v1/commands/card.trash", operation_id = "card_trash", request_body = CommandRequest<TrashCard>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn trash(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<TrashCard>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.card, p.expected),
        Change::Trash,
    )
}
#[utoipa::path(post, path = "/v1/commands/card.restore", operation_id = "card_restore", request_body = CommandRequest<RestoreCard>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn restore(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RestoreCard>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.card, p.expected),
        Change::Restore,
    )
}

#[utoipa::path(post, path = "/v1/commands/card.begin_editing", operation_id = "card_begin_editing", request_body = CommandRequest<BeginCardEditing>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn begin_editing(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<BeginCardEditing>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;

    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.card, p.expected),
        Change::BeginEditing,
    )
}

#[utoipa::path(post, path = "/v1/commands/card.autosave", operation_id = "card_autosave", request_body = CommandRequest<AutosaveCard>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn autosave(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<AutosaveCard>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    title(&p.title)?;
    if !valid_id(&p.session) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.card, p.expected),
        Change::Autosave(p),
    )
}

#[utoipa::path(post, path = "/v1/commands/card.finish_editing", operation_id = "card_finish_editing", request_body = CommandRequest<FinishCardEditing>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn finish_editing(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<FinishCardEditing>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    if !valid_id(&p.session) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.card, p.expected),
        Change::FinishEditing(&p.session),
    )
}

#[utoipa::path(post, path = "/v1/commands/card.restore_text", operation_id = "card_restore_text", request_body = CommandRequest<RestoreCardText>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn restore_text(
    tx: &mut impl CardStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RestoreCardText>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    let parts: Vec<_> = p.text_version.split(':').collect();
    if parts.len() != 2 || !parts.iter().all(|part| valid_id(part)) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    change(
        tx,
        credential,
        clock,
        request,
        (&p.space, &p.card, p.expected),
        Change::RestoreText(&p.text_version),
    )
}
