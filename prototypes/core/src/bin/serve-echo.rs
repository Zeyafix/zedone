//! Loopback-only synthetic HTTP adapter; no product data or permissions.
#![forbid(unsafe_code)]

use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use zedone_prototype_core::storage::{Storage, StorageRequest};
use zedone_prototype_core::{EchoRequest, echo};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let port: u16 = args.next().unwrap_or_else(|| "4317".into()).parse()?;
    let directory = args
        .next()
        .unwrap_or_else(|| "target/synthetic-storage".into());
    if args.next().is_some() {
        return Err("Expected a loopback port and an optional prototype directory".into());
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let app = Router::new()
        .route(
            "/prototype/echo",
            post(async |Json(request): Json<EchoRequest>| Json(echo(request))),
        )
        .route(
            "/prototype/storage",
            post(
                async |State(storage): State<Storage>, Json(request): Json<StorageRequest>| {
                    storage
                        .execute(request)
                        .await
                        .map(Json)
                        .map_err(|error| (StatusCode::SERVICE_UNAVAILABLE, Json(error)))
                },
            ),
        )
        .with_state(Storage::new(directory.into()));
    println!("http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
