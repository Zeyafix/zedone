//! Transport framing only. All commands use the same Rust decoder, fixed core
//! handlers and timed storage transaction. No callback or authority from JSON.
pub mod http;
pub mod ipc;

use serde_json::Value;
use zedone_core::{
    access::Credential,
    access_commands::{self, InspectAccess, RevokeAccess},
    error::{CommandError, ErrorCode},
    replay::Execution,
    wire::{CommandResponse, decode_request},
};
use zedone_storage::Storage;

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct Service {
    storage: Storage,
}

impl Service {
    /// Trusted host supplies an already admitted Storage; no request can open
    /// another database, migrate it, bootstrap or grant itself rights.
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    pub async fn dispatch(
        &self,
        operation: &str,
        body: &[u8],
        key: &str,
    ) -> CommandResponse<Value> {
        let invalid = || CommandError {
            code: ErrorCode::InvalidRequest,
        };
        let result = async {
            let secret = decode_key(key)?;
            if body.len() > MAX_REQUEST_BYTES {
                return Err(invalid());
            }
            match operation {
                InspectAccess::NAME => {
                    let request = decode_request::<InspectAccess>(body)?;
                    let execution = self
                        .storage
                        .timed_transact(move |ctx, clock| {
                            Ok(access_commands::inspect(
                                ctx,
                                &Credential(&secret),
                                clock,
                                &request,
                            )?)
                        })
                        .await
                        .map_err(CommandError::from)?;
                    value(execution)
                }
                RevokeAccess::NAME => {
                    let request = decode_request::<RevokeAccess>(body)?;
                    let execution = self
                        .storage
                        .timed_transact(move |ctx, clock| {
                            Ok(access_commands::revoke(
                                ctx,
                                &Credential(&secret),
                                clock,
                                &request,
                            )?)
                        })
                        .await
                        .map_err(CommandError::from)?;
                    value(execution)
                }
                _ => Err(invalid()),
            }
        }
        .await;
        result.into()
    }
}

use zedone_core::command::Command;
fn value<T: serde::Serialize>(execution: Execution<T>) -> Result<Execution<Value>, CommandError> {
    Ok(Execution {
        result: serde_json::to_value(execution.result).map_err(|_| CommandError {
            code: ErrorCode::OutcomeUnknown,
        })?,
        replayed: execution.replayed,
    })
}

/// Wire keys are exact lowercase hex. Never Debug/Serialize/log the decoded
/// secret; authentication hashes it inside the authoritative transaction.
fn decode_key(key: &str) -> Result<[u8; 32], CommandError> {
    let denied = || CommandError {
        code: ErrorCode::AccessDenied,
    };
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(denied());
    }
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&key[index * 2..index * 2 + 2], 16).map_err(|_| denied())?;
    }
    Ok(bytes)
}
