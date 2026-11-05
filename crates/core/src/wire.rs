//! Shared JSON contract for HTTP, CLI and IPC adapters.
//! Credentials travel separately; decoding never authenticates or runs a command.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{CommandId, Execution},
};

/// Tauri framing preserves the original JSON string, including duplicate
/// fields for the concrete decoder to reject. Credentials are a separate IPC
/// argument, never part of the command or a saved client intention.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct IpcInvocation {
    pub operation: String,
    pub request: String,
}

/// Preserve this entire request, including the ID, for every delivery of an
/// intention. New intentions require new client-generated cryptographic IDs.
/// Concrete command types define all targets, parameters and original versions.
/// The Value specialization in OpenAPI describes only this envelope, not an
/// implemented operation or a substitute for concrete command validation.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandRequest<C> {
    #[schema(schema_with = crate::replay::byte_array_schema::<16>)]
    pub id: CommandId,
    pub operation: String,
    pub parameters: C,
}

/// All adapters must decode into the operation's concrete Rust command type.
/// The operation name is checked against that type, never used as an unchecked
/// permission, handler or authenticated identity supplied by the caller.
pub fn decode_request<C: Command>(bytes: &[u8]) -> CommandResult<CommandRequest<C>> {
    let invalid = || CommandError {
        code: ErrorCode::InvalidRequest,
    };
    let request: CommandRequest<C> = serde_json::from_slice(bytes).map_err(|_| invalid())?;
    if request.id == [0; 16] || C::NAME.is_empty() || request.operation != C::NAME {
        return Err(invalid());
    }
    Ok(request)
}

/// Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.
/// A success confirms the original completion, not the current object state.
/// Construct success only AFTER the storage transaction commits. Cancellation,
/// a lost response or uncertain commit must never be converted to success.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum CommandResponse<T> {
    Ok(CommandSuccess<T>),
    Error(CommandFailure),
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandSuccess<T> {
    pub status: SuccessStatus,
    pub result: T,
    pub replayed: bool,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CommandFailure {
    pub status: ErrorStatus,
    pub error: CommandError,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum SuccessStatus {
    Ok,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ErrorStatus {
    Error,
}

impl<T> From<CommandResult<Execution<T>>> for CommandResponse<T> {
    fn from(completed: CommandResult<Execution<T>>) -> Self {
        match completed {
            Ok(execution) => Self::Ok(CommandSuccess {
                status: SuccessStatus::Ok,
                result: execution.result,
                replayed: execution.replayed,
            }),
            Err(error) => Self::Error(CommandFailure {
                status: ErrorStatus::Error,
                error,
            }),
        }
    }
}
