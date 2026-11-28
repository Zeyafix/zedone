use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
};
use zedone_core::{
    error::{CommandError, ErrorCode},
    wire::CommandResponse,
};

use crate::Service;

pub fn router(service: Service) -> Router {
    router_with_origin(service, None)
}

#[derive(Clone)]
struct HttpState {
    service: Option<Service>,
    maintenance: Option<std::sync::Arc<crate::maintenance::Maintenance>>,
    browser_origin: Option<String>,
}

pub fn router_with_origin(service: Service, browser_origin: Option<String>) -> Router {
    Router::new()
        .route("/v1/commands/{operation}", post(dispatch))
        .fallback(|| async { failure(ErrorCode::InvalidRequest) })
        .method_not_allowed_fallback(|| async { failure(ErrorCode::InvalidRequest) })
        .layer(DefaultBodyLimit::max(crate::MAX_EXCHANGE_BYTES))
        .with_state(HttpState {
            service: Some(service),
            maintenance: None,
            browser_origin,
        })
}

pub fn maintenance_router(
    maintenance: std::sync::Arc<crate::maintenance::Maintenance>,
    browser_origin: Option<String>,
) -> Router {
    Router::new()
        .route("/v1/commands/{operation}", post(dispatch))
        .fallback(|| async { failure(ErrorCode::InvalidRequest) })
        .method_not_allowed_fallback(|| async { failure(ErrorCode::InvalidRequest) })
        .layer(DefaultBodyLimit::max(crate::MAX_EXCHANGE_BYTES))
        .with_state(HttpState {
            service: None,
            maintenance: Some(maintenance),
            browser_origin,
        })
}

async fn dispatch(
    operation: Result<Path<String>, axum::extract::rejection::PathRejection>,
    State(service): State<HttpState>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let Ok(Path(operation)) = operation else {
        return failure(ErrorCode::InvalidRequest);
    };
    command(service, headers, body, &operation).await
}

async fn command(
    state: HttpState,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
    operation: &str,
) -> Response {
    let origins = headers.get_all("origin").iter().collect::<Vec<_>>();
    if (!origins.is_empty()
        && (origins.len() != 1 || origins[0].to_str().ok() != state.browser_origin.as_deref()))
        || headers.get_all("authorization").iter().count() != 1
    {
        return failure(ErrorCode::AccessDenied);
    }
    let Some(key) = headers
        .get("authorization")
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
    else {
        return failure(ErrorCode::AccessDenied);
    };
    if headers.get_all("content-type").iter().count() != 1
        || headers
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.split(';').next())
            .map(str::trim)
            != Some("application/json")
    {
        return failure(ErrorCode::InvalidRequest);
    }
    let Ok(body) = body else {
        return failure(ErrorCode::InvalidRequest);
    };
    if let Some(maintenance) = state.maintenance {
        if operation == "maintenance.execute" {
            return respond(maintenance.dispatch(operation, &body, key).await);
        }
        if let Some(service) = maintenance.service() {
            return respond(service.dispatch(operation, &body, key).await);
        }
        return failure(ErrorCode::Unavailable);
    }
    match state.service {
        Some(service) => respond(service.dispatch(operation, &body, key).await),
        None => failure(ErrorCode::Unavailable),
    }
}

fn failure(code: ErrorCode) -> Response {
    respond(Err(CommandError { code }).into())
}
fn respond(response: CommandResponse<serde_json::Value>) -> Response {
    (
        StatusCode::OK,
        [
            ("cache-control", "no-store"),
            ("x-content-type-options", "nosniff"),
        ],
        Json(response),
    )
        .into_response()
}
