use serde::{Deserialize, Serialize, de::DeserializeOwned};
use utoipa::ToSchema;

use crate::{
    access::{AccessTransaction, Clock, Credential, RequiredAccess, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
};

pub type CommandId = [u8; 16];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StateVersion(pub [u8; 32]);

impl utoipa::PartialSchema for StateVersion {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        byte_array_schema::<32>().into()
    }
}
impl ToSchema for StateVersion {}

pub(crate) fn byte_array_schema<const N: usize>() -> utoipa::openapi::schema::Array {
    use utoipa::openapi::schema::{ArrayBuilder, ObjectBuilder, Type};
    ArrayBuilder::new()
        .items(
            ObjectBuilder::new()
                .schema_type(Type::Integer)
                .minimum(Some(0))
                .maximum(Some(255)),
        )
        .min_items(Some(N))
        .max_items(Some(N))
        .build()
}

pub fn require_current(expected: StateVersion, current: StateVersion) -> CommandResult<()> {
    if expected == current {
        Ok(())
    } else {
        Err(error(ErrorCode::StaleState))
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RecordedCommand {
    pub request: Vec<u8>,
    pub result: Vec<u8>,
}

pub trait ReplayTransaction: AccessTransaction {
    fn recorded(&mut self, key: &[u8]) -> CommandResult<Option<RecordedCommand>>;
    fn record(&mut self, key: &[u8], command: &RecordedCommand) -> CommandResult<()>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct Execution<T> {
    pub result: T,
    pub replayed: bool,
}

fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}

pub fn canonical_json(value: &impl Serialize) -> CommandResult<Vec<u8>> {
    let value = serde_json::to_value(value).map_err(|_| error(ErrorCode::InvalidRequest))?;
    serde_json::to_vec(&value).map_err(|_| error(ErrorCode::InvalidRequest))
}

pub fn execute<C, T>(
    transaction: &mut T,
    credential: &Credential<'_>,
    clock: &impl Clock,
    required: &[RequiredAccess<'_>],
    id: CommandId,
    command: &C,
    operation: impl FnOnce(&mut T) -> CommandResult<C::Output>,
) -> CommandResult<Execution<C::Output>>
where
    C: Command,
    C::Output: DeserializeOwned,
    T: ReplayTransaction,
{
    execute_as(
        transaction,
        credential,
        clock,
        required,
        id,
        command,
        |tx, _| operation(tx),
    )
}

pub fn execute_as<C, T>(
    transaction: &mut T,
    credential: &Credential<'_>,
    clock: &impl Clock,
    required: &[RequiredAccess<'_>],
    id: CommandId,
    command: &C,
    operation: impl FnOnce(&mut T, &str) -> CommandResult<C::Output>,
) -> CommandResult<Execution<C::Output>>
where
    C: Command,
    C::Output: DeserializeOwned,
    T: ReplayTransaction,
{
    if id == [0; 16] || C::NAME.is_empty() {
        return Err(error(ErrorCode::InvalidRequest));
    }
    let request = canonical_json(&(1, C::NAME, command))?;
    with_authorized(transaction, credential, required, clock, |tx, subject| {
        let key = canonical_json(&("zedone.command.v1", subject, id))?;
        if let Some(recorded) = tx.recorded(&key)? {
            if recorded.request != request {
                return Err(error(ErrorCode::CommandMismatch));
            }
            let result = serde_json::from_slice(&recorded.result)
                .map_err(|_| error(ErrorCode::Unavailable))?;
            return Ok(Execution {
                result,
                replayed: true,
            });
        }
        let result = operation(tx, subject)?;
        let bytes = serde_json::to_vec(&result).map_err(|_| error(ErrorCode::Unavailable))?;
        tx.record(
            &key,
            &RecordedCommand {
                request,
                result: bytes,
            },
        )?;
        Ok(Execution {
            result,
            replayed: false,
        })
    })
}
