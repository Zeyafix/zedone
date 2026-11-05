//! Local CLI uses the same command service. Secrets are read from a local key
//! file, never a command-line argument; the exact request arrives on stdin.
use std::{
    io::{Read, Write},
    path::Path,
};
use zedone_adapters::{MAX_REQUEST_BYTES, Service};
use zedone_core::{
    error::{CommandError, ErrorCode},
    wire::CommandResponse,
};

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let response = run()
        .await
        .unwrap_or_else(|code| Err(CommandError { code }).into());
    // No diagnostics include input, credentials, SQL or host paths.
    if serde_json::to_writer(std::io::stdout(), &response).is_err()
        || std::io::stdout().write_all(b"\n").is_err()
    {
        std::process::exit(2);
    }
    if matches!(response, CommandResponse::Error(_)) {
        std::process::exit(1);
    }
}

async fn run() -> Result<CommandResponse<serde_json::Value>, ErrorCode> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 6
        || args[0] != "--database"
        || args[2] != "--key-file"
        || args[4] != "--operation"
    {
        return Err(ErrorCode::InvalidRequest);
    }
    if !Path::new(&args[1]).is_file() {
        return Err(ErrorCode::Unavailable);
    }
    let mut key = String::new();
    std::fs::File::open(&args[3])
        .map_err(|_| ErrorCode::AccessDenied)?
        .take(66)
        .read_to_string(&mut key)
        .map_err(|_| ErrorCode::AccessDenied)?;
    let mut body = Vec::new();
    std::io::stdin()
        .take((MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|_| ErrorCode::InvalidRequest)?;
    let storage = zedone_storage::Storage::open(&args[1])
        .await
        .map_err(|e| CommandError::from(e).code)?;
    let response = Service::new(storage.clone())
        .dispatch(&args[5], &body, key.trim_end_matches(['\r', '\n']))
        .await;
    // A later shutdown failure cannot turn a confirmed commit into the
    // "known not committed" unavailable error.
    let _ = storage.shutdown().await;
    Ok(response)
}
