//! Transport-independent product contract. Storage and adapters follow in P03.02–05.

pub mod command;
pub mod error;

use utoipa::OpenApi;

/// Schemas only until concrete operations are implemented; this is not a server.
#[derive(OpenApi)]
#[openapi(info(title = "ZeDone API"), components(schemas(error::CommandError)))]
pub struct ApiDoc;
