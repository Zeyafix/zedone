//! Editing sessions and history boundaries shared by human and agent clients.
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    access::Clock,
    cards::CardSnapshot,
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::StateVersion,
};

pub const SERIES_IDLE_MS: i64 = 30_000;

pub(crate) fn now_ms(clock: &impl Clock) -> CommandResult<i64> {
    clock
        .now()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .ok_or(CommandError {
            code: ErrorCode::Unavailable,
        })
}

/// The command ID becomes the session ID. Begin anew after a client restart;
/// retry an uncertain begin with its original ID before making a new intention.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BeginCardEditing {
    pub space: String,
    pub card: String,
    pub expected: StateVersion,
}
impl Command for BeginCardEditing {
    const NAME: &'static str = "card.begin_editing";
    type Output = CardSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct AutosaveCard {
    pub space: String,
    pub card: String,
    pub session: String,
    pub title: String,
    pub description: String,
    pub expected: StateVersion,
}
impl Command for AutosaveCard {
    const NAME: &'static str = "card.autosave";
    type Output = CardSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct FinishCardEditing {
    pub space: String,
    pub card: String,
    pub session: String,
    pub expected: StateVersion,
}
impl Command for FinishCardEditing {
    const NAME: &'static str = "card.finish_editing";
    type Output = CardSnapshot;
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RestoreCardText {
    pub space: String,
    pub card: String,
    pub text_version: String,
    pub expected: StateVersion,
}
impl Command for RestoreCardText {
    const NAME: &'static str = "card.restore_text";
    type Output = CardSnapshot;
}
