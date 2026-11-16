use std::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
#[schema(
    description = "Shared machine-readable errors, without credentials, SQL or platform details.\n\nCodes describe semantics, not HTTP statuses or retry policy. A stale request\nrequires a fresh read and a new explicit action. An unknown outcome requires\nchecking/replaying the original command, never inventing a new command ID."
)]
pub enum ErrorCode {
    InvalidRequest,
    AccessDenied,
    NotFound,
    StaleState,
    Conflict,
    CommandMismatch,
    Unavailable,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandError {
    pub code: ErrorCode,
}

impl fmt::Display for CommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self.code)
    }
}

impl std::error::Error for CommandError {}
