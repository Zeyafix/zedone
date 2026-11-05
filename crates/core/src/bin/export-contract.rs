use std::io::{self, Write};

use utoipa::OpenApi;
use zedone_core::ApiDoc;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let document = ApiDoc::openapi().to_pretty_json()?;
    let mut output = io::stdout().lock();
    output.write_all(document.as_bytes())?;
    output.write_all(b"\n")?;
    Ok(())
}
