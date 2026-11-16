mod offers;
#[cfg(test)]
#[path = "device_host_tests.rs"]
mod tests;
use crate::{
    MAX_REQUEST_BYTES, Service, channel,
    network::{self, SecureListener},
    pairing::{Offer, PairingHost, SessionLifecycle},
    secrets::{Secret, SecretStore},
};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, Path, State},
    http::HeaderMap,
    routing::post,
};
use std::{net::SocketAddr, path::Path as FilePath, sync::Arc};
use zedone_core::{
    access::{Credential, RequiredAccess, with_authorized},
    error::{CommandError, ErrorCode},
    replay::{Execution, StateVersion},
    wire::CommandResponse,
};

pub struct DeviceHost<S> {
    service: Service,
    store: Arc<S>,
    sessions: SessionLifecycle<Arc<S>>,
    operations: tokio::sync::Mutex<()>,
}

impl<S: SecretStore + 'static> DeviceHost<S> {
    pub fn new(service: Service, store: S) -> Self {
        let store = Arc::new(store);
        Self {
            service,
            sessions: SessionLifecycle::new(store.clone()),
            store,
            operations: tokio::sync::Mutex::new(()),
        }
    }
    pub async fn authorize(&self, key: &str) -> Result<(), ErrorCode> {
        let key = crate::decode_key(key).map_err(|e| e.code)?;
        self.service
            .storage
            .timed_transact(move |tx, clock| {
                Ok(with_authorized(
                    tx,
                    &Credential(&key),
                    &[RequiredAccess::ManageDevices],
                    clock,
                    |_, _| Ok(()),
                )?)
            })
            .await
            .map_err(|e| CommandError::from(e).code)
    }

    pub async fn start(
        self: &Arc<Self>,
        address: SocketAddr,
        key: &str,
    ) -> Result<(SecureListener, channel::Fingerprint), ErrorCode> {
        self.authorize(key).await?;
        let identity = match channel::load_identity(&self.store)
            .await
            .map_err(|_| ErrorCode::Unavailable)?
        {
            Some(value) => value,
            None => channel::create_identity(&self.store)
                .await
                .map_err(|_| ErrorCode::Unavailable)?,
        };
        let pin = identity.fingerprint();
        Ok((
            SecureListener::start(address, Arc::new(identity), self.router()).await?,
            pin,
        ))
    }
    pub fn router(self: &Arc<Self>) -> Router {
        PairingHost::new(self.service.storage.clone())
            .router()
            .merge(crate::http::router(self.service.clone()))
            .merge(
                Router::new()
                    .route("/v1/host/{operation}", post(dispatch::<S>))
                    .layer(DefaultBodyLimit::max(MAX_REQUEST_BYTES))
                    .with_state(self.clone()),
            )
    }

    pub async fn peer_command(
        &self,
        operation: &str,
        body: &[u8],
    ) -> Result<CommandResponse<serde_json::Value>, ErrorCode> {
        let _guard = self.operations.lock().await;
        if operation.is_empty()
            || !operation
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c == b'.' || c == b'_')
        {
            return Err(ErrorCode::InvalidRequest);
        }
        let session = self
            .sessions
            .connected()
            .await?
            .ok_or(ErrorCode::AccessDenied)?;
        network::command(
            session.address.ok_or(ErrorCode::Unavailable)?,
            session.fingerprint(),
            &format!("/v1/commands/{operation}"),
            session.credential().0,
            body,
        )
        .await
    }

    pub async fn dispatch(
        &self,
        operation: &str,
        body: &[u8],
        key: &str,
    ) -> CommandResponse<serde_json::Value> {
        if matches!(operation, "offer.create" | "offer.read") {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Empty {}
            if serde_json::from_slice::<Empty>(body).is_err() {
                return failure(ErrorCode::InvalidRequest);
            }
            return self.outgoing_offer(operation == "offer.create", key).await;
        }
        let result = async {
            let _guard = self.operations.lock().await;
            self.authorize(key).await?;
            if body.len() > MAX_REQUEST_BYTES {
                return Err(ErrorCode::InvalidRequest);
            }
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Empty {}
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Prepare {
                address: SocketAddr,
                offer: String,
                label: String,
            }
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Change {
                id: [u8; 16],
                expected: StateVersion,
            }
            fn decode<T: serde::de::DeserializeOwned>(body: &[u8]) -> Result<T, ErrorCode> {
                serde_json::from_slice(body).map_err(|_| ErrorCode::InvalidRequest)
            }
            let value = match operation {
                "session.prepare" => {
                    let input: Prepare = decode(body)?;
                    let secret = Secret::new(input.offer.into_bytes());
                    if input.address.port() == 0 || input.address.ip().is_unspecified() {
                        return Err(ErrorCode::InvalidRequest);
                    }
                    self.sessions
                        .create_at(Offer::decode(&secret)?, input.label, Some(input.address))
                        .await?;
                    self.sessions.state().await?
                }
                "session.status" => {
                    let _: Empty = decode(body)?;
                    self.sessions.state().await?
                }
                "session.submit" | "session.refresh" => {
                    let _: Empty = decode(body)?;
                    let stream = network::connect(self.sessions.address().await?).await?;
                    let response = if operation == "session.submit" {
                        self.sessions.submit(stream).await?
                    } else {
                        self.sessions.refresh(stream).await?
                    };
                    return serde_json::to_value(response).map_err(|_| ErrorCode::OutcomeUnknown);
                }
                "session.cancel" | "session.unlink" => {
                    let input: Change = decode(body)?;
                    if input.id == [0; 16] {
                        return Err(ErrorCode::InvalidRequest);
                    }
                    let stream = network::connect(self.sessions.address().await?).await?;
                    let response = if operation == "session.cancel" {
                        self.sessions
                            .cancel(stream, input.id, input.expected)
                            .await?
                    } else {
                        self.sessions
                            .unlink(stream, input.id, input.expected)
                            .await?
                    };
                    return serde_json::to_value(response).map_err(|_| ErrorCode::OutcomeUnknown);
                }
                _ => return Err(ErrorCode::InvalidRequest),
            };
            serde_json::to_value(CommandResponse::from(Ok(Execution {
                result: value,
                replayed: false,
            })))
            .map_err(|_| ErrorCode::OutcomeUnknown)
        }
        .await;
        match result {
            Ok(value) => {
                serde_json::from_value(value).unwrap_or_else(|_| failure(ErrorCode::OutcomeUnknown))
            }
            Err(code) => failure(code),
        }
    }
}
fn failure(code: ErrorCode) -> CommandResponse<serde_json::Value> {
    Err(CommandError { code }).into()
}
async fn dispatch<S: SecretStore + 'static>(
    Path(operation): Path<String>,
    State(host): State<Arc<DeviceHost<S>>>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> (
    [(&'static str, &'static str); 2],
    Json<CommandResponse<serde_json::Value>>,
) {
    let response = async {
        if headers.contains_key("origin") || headers.get_all("authorization").iter().count() != 1 {
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
            || headers.get("content-type").and_then(|h| h.to_str().ok()) != Some("application/json")
        {
            return failure(ErrorCode::InvalidRequest);
        }
        let Ok(body) = body else {
            return failure(ErrorCode::InvalidRequest);
        };
        host.dispatch(&operation, &body, key).await
    }
    .await;
    (
        [
            ("cache-control", "no-store"),
            ("x-content-type-options", "nosniff"),
        ],
        Json(response),
    )
}

pub fn namespace(directory: &FilePath) -> Result<[u8; 32], ErrorCode> {
    use std::io::{Read, Write};
    let path = directory.join("host-namespace-v1");
    let load = || -> Result<[u8; 32], ErrorCode> {
        let mut bytes = Vec::new();
        std::fs::File::open(&path)
            .map_err(|_| ErrorCode::Unavailable)?
            .take(33)
            .read_to_end(&mut bytes)
            .map_err(|_| ErrorCode::Unavailable)?;
        bytes.try_into().map_err(|_| ErrorCode::Unavailable)
    };
    match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
    {
        Ok(mut file) => {
            let mut bytes = [0; 32];
            getrandom::fill(&mut bytes).map_err(|_| ErrorCode::Unavailable)?;
            file.write_all(&bytes)
                .and_then(|_| file.sync_all())
                .map_err(|_| ErrorCode::Unavailable)?;
            load()
        }
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => load(),
        Err(_) => Err(ErrorCode::Unavailable),
    }
}
