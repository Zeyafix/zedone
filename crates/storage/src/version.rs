use serde::Serialize;
use sha2::{Digest, Sha256};
use zedone_core::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
    replay::{StateVersion, canonical_json},
};

pub fn state_version(
    scope: &impl Serialize,
    state: &impl Serialize,
) -> CommandResult<StateVersion> {
    let bytes = canonical_json(&("zedone.state.v1", scope, state)).map_err(|_| CommandError {
        code: ErrorCode::Unavailable,
    })?;
    Ok(StateVersion(Sha256::digest(bytes).into()))
}
