pub mod access;
pub mod access_commands;
pub mod card_history;
pub mod cards;
pub mod cleanup;
pub mod columns;
pub mod command;
pub mod error;
pub mod invitations;
pub mod ordering;
pub mod pairing;
pub mod qr;
pub mod replay;
mod schema_descriptions;
pub mod search;
pub mod spaces;
pub mod tags;
mod text;
pub mod wire;

use schema_descriptions::SchemaDescriptions;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "ZeDone API"),
    paths(access_commands::inspect, access_commands::revoke,
        access_commands::issue, access_commands::replace, access_commands::list,
        invitations::create, invitations::inspect, invitations::cancel, invitations::list,
        pairing::inspect, pairing::cancel, pairing::list, pairing::confirm, pairing::reject, pairing::unlink,
        spaces::create, spaces::read, spaces::list, spaces::rename,
        spaces::trash, spaces::restore,
        columns::create, columns::read, columns::list, columns::rename,
        columns::trash, columns::restore, ordering::move_column, ordering::move_card,
        cards::create, cards::read, cards::history, cards::list, cards::edit,
        cards::set_completed, cards::trash, cards::restore,
        cards::begin_editing, cards::autosave, cards::finish_editing, cards::restore_text,
        tags::create, tags::read, tags::list, tags::rename, tags::delete, tags::set_card_tag,
        search::search, cleanup::inspect_trash, cleanup::purge_trash, cleanup::inspect_history, cleanup::clear_history),
    modifiers(&Security, &SchemaDescriptions),
    components(schemas(
        error::CommandError,
        replay::StateVersion,
        wire::IpcInvocation,
        pairing::RequestPairing,
        wire::CommandRequest<pairing::RequestPairing>,
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
