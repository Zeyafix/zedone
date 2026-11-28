use crate::Shell;
use axum::{
    Router,
    body::Body,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::Response,
    routing::{get, post},
};
use std::sync::Arc;

const CSP: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' http: https:; object-src 'none'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

pub fn router(shell: Shell, address: std::net::SocketAddr) -> Router {
    let origin = format!("http://{address}");
    let authority = address.to_string();
    let service = shell.service.clone();
    let maintenance = shell.maintenance.clone();
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
        .route("/host/maintenance-session", get(maintenance_session))
        .route("/host/maintenance-folder", post(maintenance_folder))
        .route("/host/devices/{operation}", post(devices))
        .route("/host/backup-folder", post(backup_folder))
        .layer(DefaultBodyLimit::max(zedone_adapters::MAX_REQUEST_BYTES))
        .with_state(Arc::new(shell));
    let app = if let Some(maintenance) = maintenance {
        assets.merge(zedone_adapters::http::maintenance_router(
            maintenance,
            Some(origin.clone()),
        ))
    } else if let Some(service) = service {
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
async fn maintenance_session(
    State(shell): State<Arc<Shell>>,
) -> axum::Json<Option<zedone_storage::emergency::LocalSession>> {
    axum::Json(shell.maintenance_session())
}

async fn maintenance_folder(
    State(shell): State<Arc<Shell>>,
    headers: HeaderMap,
) -> Result<axum::Json<Option<String>>, StatusCode> {
    if headers.get_all("authorization").iter().count() != 1 {
        return Err(StatusCode::FORBIDDEN);
    }
    let key = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(StatusCode::FORBIDDEN)?;
    shell
        .maintenance
        .as_ref()
        .ok_or(StatusCode::SERVICE_UNAVAILABLE)?
        .owner_access(key)
        .await
        .map_err(|_| StatusCode::FORBIDDEN)?;
    Ok(axum::Json(
        crate::backup_folders::select_path()
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
    ))
}

async fn backup_folder(
    State(shell): State<Arc<Shell>>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> axum::Json<zedone_core::wire::CommandResponse<serde_json::Value>> {
    use zedone_core::{
        error::{CommandError, ErrorCode},
        wire::CommandResponse,
    };
    let result = async {
        if headers.get_all("authorization").iter().count() != 1
            || headers.get_all("content-type").iter().count() != 1
            || headers.get("content-type").and_then(|h| h.to_str().ok()) != Some("application/json")
        {
            return Err(CommandError {
                code: ErrorCode::InvalidRequest,
            });
        }
        let key = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(CommandError {
                code: ErrorCode::AccessDenied,
            })?;
        let crate::Session::Configured { credential } = shell.session() else {
            return Err(CommandError {
                code: ErrorCode::AccessDenied,
            });
        };
        if key != credential.key {
            return Err(CommandError {
                code: ErrorCode::AccessDenied,
            });
        }
        let selection = serde_json::from_slice(&body.map_err(|_| CommandError {
            code: ErrorCode::InvalidRequest,
        })?)
        .map_err(|_| CommandError {
            code: ErrorCode::InvalidRequest,
        })?;
        let service = shell.current_service().ok_or(CommandError {
            code: ErrorCode::Unavailable,
        })?;
        let check=serde_json::to_vec(&serde_json::json!({"id":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"operation":"backups.folders","parameters":{}})).map_err(|_|CommandError{code:ErrorCode::Unavailable})?;
        if !matches!(service.dispatch("backups.folders",&check,key).await,CommandResponse::Ok(_)) {return Err(CommandError{code:ErrorCode::AccessDenied});}
        let reply = crate::backup_folders::choose(&service, key, selection).await?;
        Ok(zedone_core::replay::Execution {
            result: serde_json::to_value(reply).map_err(|_| CommandError {
                code: ErrorCode::Unavailable,
            })?,
            replayed: false,
        })
    }
    .await;
    axum::Json(CommandResponse::from(result))
}
async fn devices(
    State(shell): State<Arc<Shell>>,
    Path(operation): Path<String>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> axum::Json<zedone_core::wire::CommandResponse<serde_json::Value>> {
    use zedone_core::{
        error::{CommandError, ErrorCode},
        wire::CommandResponse,
    };
    let failure = |code| CommandResponse::from(Err(CommandError { code }));
    if headers.get_all("authorization").iter().count() != 1 {
        return axum::Json(failure(ErrorCode::AccessDenied));
    }
    let Some(key) = headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    else {
        return axum::Json(failure(ErrorCode::AccessDenied));
    };
    if headers.get_all("content-type").iter().count() != 1
        || headers.get("content-type").and_then(|v| v.to_str().ok()) != Some("application/json")
    {
        return axum::Json(failure(ErrorCode::InvalidRequest));
    }
    let Ok(body) = body else {
        return axum::Json(failure(ErrorCode::InvalidRequest));
    };
    axum::Json(match shell.device_control() {
        Some(devices) => devices.dispatch(&operation, &body, key).await,
        None => failure(ErrorCode::Unavailable),
    })
}
fn asset(content_type: &'static str, bytes: &'static [u8]) -> Response {
    Response::builder()
        .header("content-type", content_type)
        .body(Body::from(bytes))
        .expect("fixed response")
}
