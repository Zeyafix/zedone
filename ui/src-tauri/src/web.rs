//! Embedded same-origin UI. No directory traversal, CORS, or implicit access.
use crate::Shell;
use axum::{
    Router,
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::get,
};
use std::sync::Arc;

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self'; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

pub fn router(shell: Shell, address: std::net::SocketAddr) -> Router {
    let origin = format!("http://{address}");
    let authority = address.to_string();
    let service = shell.service.clone();
    let assets = Router::new()
        .route(
            "/",
            get(|| async {
                asset(
                    "text/html; charset=utf-8",
                    include_bytes!("../../dist/index.html"),
                )
            }),
        )
        .route(
            "/app.js",
            get(|| async {
                asset(
                    "text/javascript; charset=utf-8",
                    include_bytes!("../../dist/app.js"),
                )
            }),
        )
        .route(
            "/index.css",
            get(|| async {
                asset(
                    "text/css; charset=utf-8",
                    include_bytes!("../../dist/index.css"),
                )
            }),
        )
        .route("/host/session", get(session))
        .with_state(Arc::new(shell));
    let app = if let Some(service) = service {
        assets.merge(zedone_adapters::http::router_with_origin(
            service,
            Some(origin.clone()),
        ))
    } else {
        assets
    };
    app.layer(middleware::from_fn(move |request: Request, next: Next| {
        let origin = origin.clone();
        let authority = authority.clone();
        async move {
            let headers = request.headers();
            let denied = headers.get_all("host").iter().count() != 1
                || headers.get("host").and_then(|h| h.to_str().ok()) != Some(authority.as_str())
                || headers.get_all("origin").iter().count() > 1
                || headers
                    .get("origin")
                    .is_some_and(|h| h.to_str().ok() != Some(origin.as_str()))
                || headers
                    .get("sec-fetch-site")
                    .is_some_and(|h| h != "same-origin" && h != "none");
            let mut response = if denied {
                Response::builder()
                    .status(StatusCode::FORBIDDEN)
                    .body(Body::empty())
                    .expect("fixed response")
            } else {
                next.run(request).await
            };
            for (name, value) in [
                ("cache-control", "no-store"),
                ("content-security-policy", CSP),
                ("x-content-type-options", "nosniff"),
                ("referrer-policy", "no-referrer"),
                ("cross-origin-resource-policy", "same-origin"),
                ("x-frame-options", "DENY"),
            ] {
                response
                    .headers_mut()
                    .insert(name, HeaderValue::from_static(value));
            }
            response
        }
    }))
}

async fn session(State(shell): State<Arc<Shell>>) -> axum::Json<crate::Session> {
    axum::Json(shell.session())
}
fn asset(content_type: &'static str, bytes: &'static [u8]) -> Response {
    Response::builder()
        .header("content-type", content_type)
        .body(Body::from(bytes))
        .expect("fixed response")
}
