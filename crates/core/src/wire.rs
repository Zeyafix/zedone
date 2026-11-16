use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{CommandId, Execution},
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(
    description = "Tauri framing preserves the original JSON string, including duplicate\nfields for the concrete decoder to reject. Credentials are a separate IPC\nargument, never part of the command or a saved client intention."
)]
pub struct IpcInvocation {
    pub operation: String,
    pub request: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
#[schema(
    description = "Preserve this entire request, including the ID, for every delivery of an\nintention. New intentions require new client-generated cryptographic IDs.\nConcrete command types define all targets, parameters and original versions.\nThe Value specialization in OpenAPI describes only this envelope, not an\nimplemented operation or a substitute for concrete command validation."
)]
pub struct CommandRequest<C> {
    #[schema(schema_with = crate::replay::byte_array_schema::<16>)]
    pub id: CommandId,
    pub operation: String,
    pub parameters: C,
}

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

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(untagged)]
#[schema(
    description = "Transport JSON is explicitly tagged, never Rust Result's Ok/Err encoding.\nA success confirms the original completion, not the current object state.\nConstruct success only AFTER the storage transaction commits. Cancellation,\na lost response or uncertain commit must never be converted to success."
)]
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
