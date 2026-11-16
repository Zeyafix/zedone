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
    if std::env::args().nth(1).as_deref() == Some("--listen") {
        match listen().await {
            Ok(()) => return,
            Err(code) => {
                let _: Result<_, _> = serde_json::to_writer(
                    std::io::stdout(),
                    &CommandResponse::<serde_json::Value>::from(Err(CommandError { code })),
                );
                std::process::exit(1);
            }
        }
    }
    let response = run()
        .await
        .unwrap_or_else(|code| Err(CommandError { code }).into());
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
    if args.first().map(String::as_str) == Some("--endpoint") {
        return remote(&args).await;
    }
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
    let _ = storage.shutdown().await;
    Ok(response)
}

fn read_key(path: &str) -> Result<String, ErrorCode> {
    let mut value = String::new();
    std::fs::File::open(path)
        .map_err(|_| ErrorCode::AccessDenied)?
        .take(66)
        .read_to_string(&mut value)
        .map_err(|_| ErrorCode::AccessDenied)?;
    Ok(value.trim_end_matches(['\r', '\n']).to_owned())
}
async fn remote(args: &[String]) -> Result<CommandResponse<serde_json::Value>, ErrorCode> {
    if args.len() != 8
        || args[2] != "--pin-file"
        || args[4] != "--key-file"
        || args[6] != "--operation"
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let decode = |text: String| -> Result<[u8; 32], ErrorCode> {
        if text.len() != 64 {
            return Err(ErrorCode::InvalidRequest);
        }
        let mut value = [0; 32];
        for (i, pair) in text.as_bytes().as_chunks::<2>().0.iter().enumerate() {
            value[i] = u8::from_str_radix(
                std::str::from_utf8(pair).map_err(|_| ErrorCode::InvalidRequest)?,
                16,
            )
            .map_err(|_| ErrorCode::InvalidRequest)?;
        }
        Ok(value)
    };
    let pin = zedone_adapters::channel::Fingerprint(decode(read_key(&args[3])?)?);
    let key = zedone_adapters::secrets::Secret::new(decode(read_key(&args[5])?)?.to_vec());
    let operation = &args[7];
    if operation.is_empty()
        || !operation
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c == b'.' || c == b'_')
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let mut body = zedone_adapters::secrets::Secret::new(Vec::new());
    std::io::stdin()
        .take((MAX_REQUEST_BYTES + 1) as u64)
        .read_to_end(&mut body)
        .map_err(|_| ErrorCode::InvalidRequest)?;
    let kind = if operation.starts_with("session.") || operation.starts_with("offer.") {
        "host"
    } else {
        "commands"
    };
    zedone_adapters::network::command(
        args[1].parse().map_err(|_| ErrorCode::InvalidRequest)?,
        pin,
        &format!("/v1/{kind}/{operation}"),
        &key,
        &body,
    )
    .await
}
async fn listen() -> Result<(), ErrorCode> {
    #[cfg(not(windows))]
    return Err(ErrorCode::Unavailable);
    #[cfg(windows)]
    {
        let args: Vec<_> = std::env::args().skip(1).collect();
        if args.len() != 6
            || args[0] != "--listen"
            || args[2] != "--database"
            || args[4] != "--key-file"
        {
            return Err(ErrorCode::InvalidRequest);
        }
        let path = Path::new(&args[3]);
        if !path.is_file() {
            return Err(ErrorCode::Unavailable);
        }
        let directory = path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        let namespace = zedone_adapters::device_host::namespace(directory)?;
        let store = zedone_adapters::secrets::windows::WindowsSecrets::new(namespace, directory);
        let storage = zedone_storage::Storage::open(path)
            .await
            .map_err(|_| ErrorCode::Unavailable)?;
        storage
            .recover_editing_sessions()
            .await
            .map_err(|_| ErrorCode::Unavailable)?;
        let host = std::sync::Arc::new(zedone_adapters::device_host::DeviceHost::new(
            Service::new(storage.clone()),
            store,
        ));
        let (listener, pin) = host
            .start(
                args[1].parse().map_err(|_| ErrorCode::InvalidRequest)?,
                &read_key(&args[5])?,
            )
            .await?;
        let output = serde_json::json!({"address":listener.address,"pin":pin.0.iter().map(|b|format!("{b:02x}")).collect::<String>()});
        serde_json::to_writer(std::io::stdout(), &output).map_err(|_| ErrorCode::Unavailable)?;
        std::io::stdout()
            .write_all(b"\n")
            .and_then(|_| std::io::stdout().flush())
            .map_err(|_| ErrorCode::Unavailable)?;
        let _ = tokio::signal::ctrl_c().await;
        listener.shutdown().await;
        let _ = storage.shutdown().await;
        Ok(())
    }
}
