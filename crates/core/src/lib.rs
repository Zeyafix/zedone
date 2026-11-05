//! Transport-independent product contract and authorization policy.
//! Storage implements persistence separately; public adapters follow in P03.05.

pub mod access;
pub mod command;
pub mod error;
pub mod replay;
pub mod wire;

use utoipa::OpenApi;

/// Schemas only until concrete operations are implemented; this is not a server.
#[derive(OpenApi)]
#[openapi(
    info(title = "ZeDone API"),
    components(schemas(
        error::CommandError,
        replay::StateVersion,
        wire::CommandRequest<serde_json::Value>,
        wire::CommandResponse<serde_json::Value>
    ))
)]
pub struct ApiDoc;
