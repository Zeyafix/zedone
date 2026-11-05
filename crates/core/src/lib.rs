//! Transport-independent product contract and authorization policy.
//! Storage implements persistence separately; adapters share the concrete core handlers.

pub mod access;
pub mod access_commands;
pub mod command;
pub mod error;
pub mod replay;
pub mod wire;

use utoipa::OpenApi;

/// Rust is the authoritative contract for the bounded access commands.
#[derive(OpenApi)]
#[openapi(
    info(title = "ZeDone API"),
    paths(access_commands::inspect, access_commands::revoke),
    modifiers(&Security),
    components(schemas(
        error::CommandError,
        replay::StateVersion,
        wire::IpcInvocation,
        wire::CommandRequest<serde_json::Value>,
        wire::CommandResponse<serde_json::Value>
    ))
)]
pub struct ApiDoc;

struct Security;
impl utoipa::Modify for Security {
    fn modify(&self, document: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
        document
            .components
            .as_mut()
            .expect("schemas")
            .add_security_scheme(
                "access_key",
                SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
            );
    }
}
