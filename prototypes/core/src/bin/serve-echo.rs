//! Loopback synthetic adapter. Limited sessions are controlled only through local stdin.
#![forbid(unsafe_code)]

use zedone_prototype_core::{
    http,
    http_session::{HttpSession, HttpSessionRequest},
    storage::Storage,
};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let first = args.next().unwrap_or_else(|| "4317".into());
    let directory = args
        .next()
        .unwrap_or_else(|| "target/synthetic-storage".into());
    if args.next().is_some() {
        return Err(
            "Expected a loopback port (or --limited) and an optional prototype directory".into(),
        );
    }
    let storage = Storage::new(directory.into());
    if first == "--limited" {
        use tokio::io::AsyncBufReadExt;
        let session = HttpSession::default();
        let mut lines = tokio::io::BufReader::new(tokio::io::stdin()).lines();
        while let Some(line) = lines.next_line().await? {
            let result = match serde_json::from_str::<HttpSessionRequest>(&line) {
                Ok(request) => session.execute(request, storage.clone()).await,
                Err(_) => Err("Invalid local session request".to_owned()),
            };
            println!("{}", serde_json::to_string(&result)?);
        }
    } else {
        let listener =
            tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, first.parse::<u16>()?))
                .await?;
        println!("http://{}", listener.local_addr()?);
        axum::serve(listener, http::router(storage)).await?;
    }
    Ok(())
}
