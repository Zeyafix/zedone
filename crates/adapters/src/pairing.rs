//! Bounded host-only pairing transport. It cannot serve the normal command API.
//! Hosts supply sockets; no external listener or renderer operation is enabled.
use crate::{
    channel::{self, Fingerprint, Identity},
    secrets::{Secret, SecretStore, Slot},
};
use axum::{
    Json, Router,
    body::Bytes,
    extract::{DefaultBodyLimit, State},
    http::HeaderMap,
    routing::post,
};
use http_body_util::{BodyExt, Full, Limited};
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::Semaphore,
};
use zedone_core::{
    access_commands::KeyVerifier,
    command::Command,
    error::{CommandError, ErrorCode},
    invitations::InvitationVerifier,
    pairing::{
        self, CancelPairing, InspectPairing, PairingSnapshot, PendingVerifier, RequestPairing,
        UnlinkPairing,
    },
    replay::StateVersion,
    wire::{CommandRequest, CommandResponse, decode_request},
};
use zedone_storage::Storage;

mod offer;
pub use offer::{Offer, PendingSession};

const MAX_PAIRING_BYTES: usize = 4096;
const EXCHANGE_TIMEOUT: Duration = Duration::from_secs(10);

fn digest(domain: &[u8], bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(domain);
    hash.update(bytes);
    hash.finalize().into()
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(text: &str) -> Result<[u8; 32], ErrorCode> {
    crate::decode_key(text).map_err(|_| ErrorCode::InvalidRequest)
}

#[derive(Clone)]
pub struct PairingHost {
    storage: Storage,
    connections: Arc<Semaphore>,
}
impl PairingHost {
    pub fn new(storage: Storage) -> Self {
        Self {
            storage,
            connections: Arc::new(Semaphore::new(8)),
        }
    }
    /// One bounded HTTP/1 connection after TLS verification. The private router
    /// cannot accidentally be mounted on the existing plaintext host listener.
    pub async fn serve<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        identity: &Identity,
        stream: S,
    ) -> Result<(), ErrorCode> {
        let _permit = self
            .connections
            .try_acquire()
            .map_err(|_| ErrorCode::Unavailable)?;
        let acceptor = identity.acceptor().map_err(|_| ErrorCode::Unavailable)?;
        let tls = channel::accept(&acceptor, stream)
            .await
            .map_err(|_| ErrorCode::AccessDenied)?;
        let router = Router::new()
            .route("/v1/pairing/request", post(submit))
            .route("/v1/pairing/inspect", post(inspect))
            .route("/v1/pairing/cancel", post(cancel))
            .route("/v1/pairing/unlink", post(unlink))
            .layer(DefaultBodyLimit::max(MAX_PAIRING_BYTES))
            .with_state(self.clone());
        tokio::time::timeout(
            EXCHANGE_TIMEOUT,
            hyper::server::conn::http1::Builder::new()
                .keep_alive(false)
                .max_buf_size(8192)
                .serve_connection(TokioIo::new(tls), TowerToHyperService::new(router)),
        )
        .await
        .map_err(|_| ErrorCode::OutcomeUnknown)?
        .map_err(|_| ErrorCode::OutcomeUnknown)
    }
    async fn dispatch(
        &self,
        operation: &'static str,
        headers: HeaderMap,
        body: Result<Bytes, axum::extract::rejection::BytesRejection>,
    ) -> CommandResponse<PairingSnapshot> {
        let result = async {
            if headers.contains_key("origin")
                || headers.get_all("authorization").iter().count() != 1
            {
                return Err(CommandError {
                    code: ErrorCode::AccessDenied,
                });
            }
            let key = headers
                .get("authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.strip_prefix("Pending "))
                .and_then(|h| unhex(h).ok())
                .ok_or(CommandError {
                    code: ErrorCode::AccessDenied,
                })?;
            if headers.get_all("content-type").iter().count() != 1
                || headers.get("content-type").and_then(|h| h.to_str().ok())
                    != Some("application/json")
            {
                return Err(CommandError {
                    code: ErrorCode::InvalidRequest,
                });
            }
            let body = body.map_err(|_| CommandError {
                code: ErrorCode::InvalidRequest,
            })?;
            let pending = PendingVerifier(digest(b"zedone/pending-key/v1\0", &key));
            match operation {
                "request" => {
                    #[derive(serde::Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct Submit {
                        command: CommandRequest<RequestPairing>,
                        code: String,
                    }
                    let input: Submit =
                        serde_json::from_slice(&body).map_err(|_| CommandError {
                            code: ErrorCode::InvalidRequest,
                        })?;
                    let code = InvitationVerifier(digest(
                        b"zedone/invitation-code/v1\0",
                        &unhex(&input.code).map_err(|code| CommandError { code })?,
                    ));
                    // An authentication rejection is a successful transaction:
                    // its failed-attempt counter must not be rolled back.
                    self.storage
                        .timed_transact(move |tx, clock| {
                            Ok(pairing::request(tx, clock, &input.command, code, pending)?)
                        })
                        .await
                        .map_err(CommandError::from)?
                        .ok_or(CommandError {
                            code: ErrorCode::AccessDenied,
                        })
                }
                "inspect" => {
                    let r = decode_request::<InspectPairing>(&body)?;
                    self.storage
                        .timed_transact(move |tx, _| Ok(pairing::peer_inspect(tx, &r, pending)?))
                        .await
                        .map_err(CommandError::from)
                }
                "cancel" => {
                    let r = decode_request::<CancelPairing>(&body)?;
                    self.storage
                        .timed_transact(move |tx, _| Ok(pairing::peer_cancel(tx, &r, pending)?))
                        .await
                        .map_err(CommandError::from)
                }
                "unlink" => {
                    let r = decode_request::<UnlinkPairing>(&body)?;
                    self.storage
                        .timed_transact(move |tx, _| Ok(pairing::peer_unlink(tx, &r, pending)?))
                        .await
                        .map_err(CommandError::from)
                }
                _ => Err(CommandError {
                    code: ErrorCode::InvalidRequest,
                }),
            }
        }
        .await;
        result.into()
    }
}
type Body = Result<Bytes, axum::extract::rejection::BytesRejection>;
type Reply = (
    [(&'static str, &'static str); 2],
    Json<CommandResponse<PairingSnapshot>>,
);
fn reply(response: CommandResponse<PairingSnapshot>) -> Reply {
    (
        [
            ("cache-control", "no-store"),
            ("x-content-type-options", "nosniff"),
        ],
        Json(response),
    )
}
async fn submit(State(host): State<PairingHost>, h: HeaderMap, b: Body) -> Reply {
    reply(host.dispatch("request", h, b).await)
}
async fn inspect(State(host): State<PairingHost>, h: HeaderMap, b: Body) -> Reply {
    reply(host.dispatch("inspect", h, b).await)
}
async fn cancel(State(host): State<PairingHost>, h: HeaderMap, b: Body) -> Reply {
    reply(host.dispatch("cancel", h, b).await)
}
async fn unlink(State(host): State<PairingHost>, h: HeaderMap, b: Body) -> Reply {
    reply(host.dispatch("unlink", h, b).await)
}

async fn exchange<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    stream: S,
    pin: Fingerprint,
    operation: &str,
    key: &[u8],
    bytes: Secret,
) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
    // No code or key is sent until the independently supplied pin is verified.
    let tls = channel::connect(stream, pin)
        .await
        .map_err(|_| ErrorCode::AccessDenied)?;
    let mut authorization = axum::http::HeaderValue::from_str(&format!("Pending {}", hex(key)))
        .map_err(|_| ErrorCode::InvalidRequest)?;
    authorization.set_sensitive(true);
    let request = hyper::Request::builder()
        .method("POST")
        .uri(format!("/v1/pairing/{operation}"))
        .header("host", "zedone.invalid")
        .header("content-type", "application/json")
        .header("authorization", authorization)
        .header("connection", "close")
        .body(Full::new(Bytes::copy_from_slice(&bytes)))
        .map_err(|_| ErrorCode::InvalidRequest)?;
    tokio::time::timeout(EXCHANGE_TIMEOUT, async {
        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(tls))
            .await
            .map_err(|_| ErrorCode::OutcomeUnknown)?;
        let send = async {
            let response = sender
                .send_request(request)
                .await
                .map_err(|_| ErrorCode::OutcomeUnknown)?;
            if response.status() != hyper::StatusCode::OK {
                return Err(ErrorCode::OutcomeUnknown);
            }
            let body = Limited::new(response.into_body(), MAX_PAIRING_BYTES)
                .collect()
                .await
                .map_err(|_| ErrorCode::OutcomeUnknown)?
                .to_bytes();
            serde_json::from_slice(&body).map_err(|_| ErrorCode::OutcomeUnknown)
        };
        let (result, _) = tokio::join!(send, connection);
        result
    })
    .await
    .map_err(|_| ErrorCode::OutcomeUnknown)?
}

#[cfg(test)]
mod tests;
