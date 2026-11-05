use serde::{Serialize, de::DeserializeOwned};
use utoipa::ToSchema;

use crate::error::CommandError;

/// One Rust definition connects a request, its operation name and its output.
///
/// HTTP, CLI and IPC use these same types. Implementations and authorization live
/// in the core, never in this trait or in an adapter. No product commands exist yet.
pub trait Command: DeserializeOwned + Serialize + ToSchema {
    const NAME: &'static str;
    type Output: Serialize + ToSchema;
}

/// Internal handler result, not the serialized Rust `Result` wire representation.
///
/// A successful mutation must mean that its local transaction is committed.
/// `OutcomeUnknown` must never be presented as a rejection or a saved result.
/// Command IDs, preconditions and durable replay are implemented in P03.04.
pub type CommandResult<T> = Result<T, CommandError>;
