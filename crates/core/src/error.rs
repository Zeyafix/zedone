use std::fmt;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Shared machine-readable errors, without credentials, SQL or platform details.
///
/// Codes describe semantics, not HTTP statuses or retry policy. A stale request
/// requires a fresh read and a new explicit action. An unknown outcome requires
/// checking/replaying the original command, never inventing a new command ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    InvalidRequest,
    AccessDenied,
    NotFound,
    StaleState,
    CommandMismatch,
    /// The operation is unavailable and is known not to have committed.
    Unavailable,
    /// Whether the operation committed has not been established.
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
