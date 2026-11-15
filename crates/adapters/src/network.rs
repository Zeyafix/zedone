//! Bounded pinned TLS transport. No plaintext fallback, redirects or trust-on-first-use.
use crate::{
    MAX_REQUEST_BYTES,
    channel::{self, Fingerprint, Identity},
    secrets::Secret,
};
use axum::{Router, body::Bytes};
use http_body_util::{BodyExt, Full, Limited};
use hyper_util::{rt::TokioIo, service::TowerToHyperService};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::oneshot,
    task::{JoinHandle, JoinSet},
};
use zedone_core::{error::ErrorCode, wire::CommandResponse};

pub async fn connect(address: SocketAddr) -> Result<TcpStream, ErrorCode> {
    tokio::time::timeout(Duration::from_secs(5), TcpStream::connect(address))
        .await
        .map_err(|_| ErrorCode::Unavailable)?
        .map_err(|_| ErrorCode::Unavailable)
}

pub async fn command(
    address: SocketAddr,
    pin: Fingerprint,
    path: &str,
    key: &[u8],
    body: &[u8],
) -> Result<CommandResponse<serde_json::Value>, ErrorCode> {
    if body.len() > MAX_REQUEST_BYTES
        || !path.starts_with("/v1/")
        || path.contains(['\r', '\n', '?', '#'])
    {
        return Err(ErrorCode::InvalidRequest);
    }
    let tls = channel::connect(connect(address).await?, pin)
        .await
        .map_err(|_| ErrorCode::AccessDenied)?;
    let key = Secret::new(
        key.iter()
            .flat_map(|b| format!("{b:02x}").into_bytes())
            .collect(),
    );
    let mut authorization =
        hyper::header::HeaderValue::from_bytes(&[b"Bearer ".as_slice(), key.as_slice()].concat())
            .map_err(|_| ErrorCode::InvalidRequest)?;
    authorization.set_sensitive(true);
    let request = hyper::Request::builder()
        .method("POST")
        .uri(path)
        .header("host", "zedone.invalid")
        .header("authorization", authorization)
        .header("content-type", "application/json")
        .header("connection", "close")
        .body(Full::new(Bytes::copy_from_slice(body)))
        .map_err(|_| ErrorCode::InvalidRequest)?;
    tokio::time::timeout(Duration::from_secs(10), async {
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
            let bytes = Limited::new(response.into_body(), 8 * 1024 * 1024)
                .collect()
                .await
                .map_err(|_| ErrorCode::OutcomeUnknown)?
                .to_bytes();
            serde_json::from_slice(&bytes).map_err(|_| ErrorCode::OutcomeUnknown)
        };
        let (result, _) = tokio::join!(send, connection);
        result
    })
    .await
    .map_err(|_| ErrorCode::OutcomeUnknown)?
}

pub struct SecureListener {
    pub address: SocketAddr,
    stop: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}
impl SecureListener {
    pub async fn start(
        address: SocketAddr,
        identity: Arc<Identity>,
        router: Router,
    ) -> Result<Self, ErrorCode> {
        // Validate identity before publishing the listener.
        let acceptor = identity.acceptor().map_err(|_| ErrorCode::Unavailable)?;
        let listener = TcpListener::bind(address)
            .await
            .map_err(|_| ErrorCode::Unavailable)?;
        let address = listener.local_addr().map_err(|_| ErrorCode::Unavailable)?;
        let (stop, mut stopping) = oneshot::channel();
        let task = tokio::spawn(async move {
            let mut tasks = JoinSet::new();
            loop {
                tokio::select! {
                    _ = &mut stopping => break,
                    _ = tasks.join_next(), if !tasks.is_empty() => (),
                    accepted = listener.accept() => {
                        let Ok((stream, _)) = accepted else { break };
                        if tasks.len() >= 8 { continue; }
                        let acceptor = acceptor.clone();
                        let router = router.clone();
                        tasks.spawn(async move {
                            let _ = tokio::time::timeout(Duration::from_secs(15), async {
                                let Ok(tls) = channel::accept(&acceptor, stream).await else { return };
                                let _ = hyper::server::conn::http1::Builder::new().keep_alive(false).max_buf_size(8192)
                                    .serve_connection(TokioIo::new(tls), TowerToHyperService::new(router)).await;
                            }).await;
                        });
                    }
                }
            }
            // Finish admitted work; do not abandon an in-flight storage commit.
            while tasks.join_next().await.is_some() {}
        });
        Ok(Self {
            address,
            stop: Some(stop),
            task: Some(task),
        })
    }
    pub async fn shutdown(mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = task.await;
        }
    }
}
impl Drop for SecureListener {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
    }
}
