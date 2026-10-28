use std::io::{self, Write};
use utoipa::OpenApi;
use zedone_prototype_core::ApiDoc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json = ApiDoc::openapi().to_pretty_json()?;
    writeln!(io::stdout().lock(), "{json}")?;
    Ok(())
}
