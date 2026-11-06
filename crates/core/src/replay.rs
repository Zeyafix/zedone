//! Core policy inside one trusted, serializable write transaction. No transport
//! may supply the handler, access requirements, authenticated subject or version.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use utoipa::ToSchema;

use crate::{
    access::{AccessTransaction, Clock, Credential, RequiredAccess, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
};

/// A logical intention, preserved across all automatic/manual deliveries.
/// Clients must allocate a fresh, unpredictable 128-bit ID for a new intention.
pub type CommandId = [u8; 16];

/// Opaque digest of the authoritative, scoped semantic state, not CRDT heads.
/// Its projection includes identity, all relevant data/variants and their causal
/// identities, so changing and restoring a value cannot revive an old version.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct StateVersion(pub [u8; 32]);

impl utoipa::PartialSchema for StateVersion {
    fn schema() -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        byte_array_schema::<32>().into()
    }
}
impl ToSchema for StateVersion {}

/// JSON byte arrays have exact lengths and integer elements in the u8 range.
/// Explicit bounds keep the schema as strict as Serde's fixed-size arrays.
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

/// Stored receipt, never current data and never a source of authorization.
#[derive(Debug, PartialEq, Eq)]
pub struct RecordedCommand {
    pub request: Vec<u8>,
    pub result: Vec<u8>,
}

/// The implementation serializes grant changes, receipt lookup, freshness,
/// mutation and receipt insertion. It rolls back on any error; success must not
/// escape to a client until all of these have committed atomically.
pub trait ReplayTransaction: AccessTransaction {
    fn recorded(&mut self, key: &[u8]) -> CommandResult<Option<RecordedCommand>>;
    fn record(&mut self, key: &[u8], command: &RecordedCommand) -> CommandResult<()>;
}

/// `result` describes the original completion, never a replacement current view.
#[derive(Debug, PartialEq, Eq)]
pub struct Execution<T> {
    pub result: T,
    pub replayed: bool,
}

fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}

/// Stable map order and JSON value semantics, independent of transport bytes.
/// Typed requests must include all targets, parameters and original conditions.
pub fn canonical_json(value: &impl Serialize) -> CommandResult<Vec<u8>> {
    let value = serde_json::to_value(value).map_err(|_| error(ErrorCode::InvalidRequest))?;
    serde_json::to_vec(&value).map_err(|_| error(ErrorCode::InvalidRequest))
}

/// Trusted core handlers validate immutable input, determine requirements and
/// call this AFTER queue admission. Only a new execution calls `operation`,
/// which must check the original version against authoritative state before
/// mutation. It cannot publish external effects. Errors are not cached.
/// Receipts have no automatic expiry; missing/lost history cannot be treated as
/// permission to re-execute an old ID after a future restore or cleanup.
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

/// The authenticated subject is supplied only by the guard, never by command
/// JSON. Creation uses it to grant access in the same transaction as the data.
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
        // Delimiter-safe and namespaced away from raw infrastructure receipts.
        // Operation is intentionally excluded: changing it must be a mismatch.
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
