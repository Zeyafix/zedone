//! Loopback-only synthetic HTTP adapter; no storage or product permissions.
#![forbid(unsafe_code)]

use axum::{Json, Router, routing::post};
use zedone_prototype_core::{EchoRequest, echo};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let port: u16 = args.next().unwrap_or_else(|| "4317".into()).parse()?;
    if args.next().is_some() {
        return Err("Expected at most one argument: a loopback port".into());
    }
    let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await?;
    let app = Router::new().route(
        "/prototype/echo",
        post(async |Json(request): Json<EchoRequest>| Json(echo(request))),
    );
    println!("http://{}", listener.local_addr()?);
    axum::serve(listener, app).await?;
    Ok(())
}
