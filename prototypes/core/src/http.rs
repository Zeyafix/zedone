//! One HTTP adapter for desktop and APK; remote grants are synthetic, not product rights.

use axum::{Json, Router, extract::State, http::StatusCode, middleware, routing::post};

use crate::http_session::TestGrant;
use crate::{
    EchoRequest, echo,
    invitation::{InvitationRequest, invitation},
    qr::{QrRequest, qr},
    storage::{Storage, StorageRequest},
};

pub fn router(storage: Storage) -> Router {
    routes(storage, None)
}

pub(crate) fn limited_router(storage: Storage, grant: TestGrant) -> Router {
    let auth = grant.clone();
    routes(storage, Some(grant)).layer(middleware::from_fn(
        move |request: axum::extract::Request, next: middleware::Next| {
            let auth = auth.clone();
            async move {
                if !auth.is_active() || request.headers().get("authorization") != Some(&auth.bearer)
                {
                    return Err(StatusCode::UNAUTHORIZED);
                }
                // This is an agent API, with no browser cross-origin access.
                if request.headers().contains_key("origin") {
                    return Err(StatusCode::FORBIDDEN);
                }
                tokio::time::timeout(std::time::Duration::from_secs(5), next.run(request))
                    .await
                    .map_err(|_| StatusCode::GATEWAY_TIMEOUT)
            }
        },
    ))
}

fn routes(storage: Storage, grant: Option<TestGrant>) -> Router {
    Router::new()
        .route("/prototype/invitation", post(async |Json(request): Json<InvitationRequest>| {
            invitation(request).map(Json).map_err(|error| (StatusCode::UNPROCESSABLE_ENTITY, Json(error)))
        }))
        .route("/prototype/qr", post(async |Json(request): Json<QrRequest>| {
            qr(request).map(Json).map_err(|error| (StatusCode::UNPROCESSABLE_ENTITY, Json(error)))
        }))
        .route("/prototype/echo", post(async |Json(request): Json<EchoRequest>| Json(echo(request))))
        .route("/prototype/storage", post(move |State(storage): State<Storage>, Json(request): Json<StorageRequest>| {
            let grant = grant.clone();
            async move {
                if let Some(grant) = grant {
                    grant.authorize(&request).map_err(|status| (status, Json(crate::storage::StorageError {
                        message: "Test access permits only the selected operations in synthetic space A".into(),
                    })))?;
                }
                storage.execute(request).await.map(Json).map_err(|error| (StatusCode::SERVICE_UNAVAILABLE, Json(error)))
            }
        }))
        .with_state(storage)
}
