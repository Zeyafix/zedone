use serde::Serialize;
use sha2::{Digest, Sha256};
use zedone_core::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
    replay::{StateVersion, canonical_json},
};

/// Fixed-size token derived from a versioned, identity-bound semantic projection.
/// Scope must include projection format, space, object kind and object ID.
/// State must include all checked data/variants/membership AND their causal
/// identities (including removed membership), in stable order to reject ABA,
/// excluding unrelated objects and technical history/progress. Domain handlers
/// define the projection; CRDT heads alone violate these rules. No token cache.
pub fn state_version(
    scope: &impl Serialize,
    state: &impl Serialize,
) -> CommandResult<StateVersion> {
    let bytes = canonical_json(&("zedone.state.v1", scope, state)).map_err(|_| CommandError {
        code: ErrorCode::Unavailable,
    })?;
    Ok(StateVersion(Sha256::digest(bytes).into()))
}
