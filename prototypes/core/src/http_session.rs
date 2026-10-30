//! Explicit, memory-only access to synthetic A. Local control is never routed remotely.

use crate::storage::{Space, Storage, StorageRequest};
use axum::http::{HeaderValue, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
use tokio::{sync::oneshot, task::JoinHandle};
use utoipa::ToSchema;

mod listener;

#[derive(Clone, Copy, Default, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TestAccess {
    #[default]
    Read,
    Write,
}

#[derive(Deserialize, Serialize, ToSchema)]
#[serde(tag = "action", rename_all = "snake_case", deny_unknown_fields)]
pub enum HttpSessionRequest {
    Start {
        address: String,
        port: u16,
        #[serde(default)]
        access: TestAccess,
    },
    Status,
    Stop,
}

#[derive(Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct HttpSessionStatus {
    pub endpoint: Option<String>,
    pub token: Option<String>,
    pub access: Option<TestAccess>,
    /// The existing session is waiting to rebind its original endpoint.
    pub recovering: bool,
}

#[derive(Clone)]
pub(crate) struct TestGrant {
    pub bearer: HeaderValue,
    active: Arc<AtomicBool>,
    access: TestAccess,
}

impl TestGrant {
    pub fn is_active(&self) -> bool {
        self.active.load(Ordering::SeqCst)
    }

    pub fn authorize(&self, request: &StorageRequest) -> Result<(), StatusCode> {
        if !self.is_active() {
            return Err(StatusCode::UNAUTHORIZED);
        }
        let (space, writes) = match request {
            StorageRequest::Read { space, .. } | StorageRequest::Snapshot { space } => {
                (*space, false)
            }
            StorageRequest::Seed { space } | StorageRequest::Delete { space } => (*space, true),
            StorageRequest::Restore { snapshot } => (snapshot.space, true),
            StorageRequest::MoveCard { expected, .. }
            | StorageRequest::MoveColumn { expected, .. } => (expected.space, true),
        };
        if space != Space::A || (writes && !matches!(self.access, TestAccess::Write)) {
            return Err(StatusCode::FORBIDDEN);
        }
        Ok(())
    }
}

struct Running {
    status: HttpSessionStatus,
    grant: TestGrant,
    stop: oneshot::Sender<()>,
    task: JoinHandle<()>,
    recovering: Arc<AtomicBool>,
}

#[derive(Default)]
struct SessionState {
    suspended: bool,
    running: Option<Running>,
}

#[derive(Default)]
pub struct HttpSession(Mutex<SessionState>);

impl HttpSession {
    pub async fn execute(
        &self,
        request: HttpSessionRequest,
        storage: Storage,
    ) -> Result<HttpSessionStatus, String> {
        let task = {
            let mut state = self
                .0
                .lock()
                .map_err(|_| "HTTP session state unavailable")?;
            match request {
                HttpSessionRequest::Start {
                    address,
                    port,
                    access,
                } => {
                    if state.suspended {
                        return Err("Resume the APK before starting test access".into());
                    }
                    if state
                        .running
                        .as_ref()
                        .is_some_and(|running| !running.task.is_finished())
                    {
                        return Err("Stop the existing session before changing access".into());
                    }
                    let ip: IpAddr = address
                        .parse()
                        .map_err(|_| "Enter a concrete loopback or Tailscale IP address")?;
                    let allowed = match ip {
                        IpAddr::V4(ip) => {
                            ip.is_loopback()
                                || (ip.octets()[0] == 100 && (64..=127).contains(&ip.octets()[1]))
                        }
                        IpAddr::V6(ip) => {
                            ip.is_loopback() || ip.segments()[..3] == [0xfd7a, 0x115c, 0xa1e0]
                        }
                    };
                    if !allowed {
                        return Err(
                            "Wildcard, LAN and public listeners are disabled in this prototype"
                                .into(),
                        );
                    }
                    let listener = listener::bind(SocketAddr::new(ip, port))
                        .map_err(|_| "Cannot bind the selected local address and port")?;
                    let endpoint = format!(
                        "http://{}",
                        listener.local_addr().map_err(|_| "Listener unavailable")?
                    );
                    let recovering = Arc::new(AtomicBool::new(false));
                    let listener = listener::RecoveringListener::new(listener, recovering.clone())
                        .map_err(|_| "Listener unavailable")?;
                    let mut bytes = [0u8; 32];
                    getrandom::fill(&mut bytes).map_err(|_| "Secure test key unavailable")?;
                    let token: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                    let grant = TestGrant {
                        bearer: HeaderValue::from_str(&format!("Bearer {token}"))
                            .map_err(|_| "Invalid test key")?,
                        active: Arc::new(AtomicBool::new(true)),
                        access,
                    };
                    let status = HttpSessionStatus {
                        endpoint: Some(endpoint),
                        token: Some(token),
                        access: Some(access),
                        recovering: false,
                    };
                    let app = crate::http::limited_router(storage, grant.clone());
                    let (stop, stopped) = oneshot::channel();
                    let active = grant.active.clone();
                    let task = tokio::spawn(async move {
                        let _ = axum::serve(listener, app)
                            .with_graceful_shutdown(async {
                                let _ = stopped.await;
                            })
                            .await;
                        active.store(false, Ordering::SeqCst);
                    });
                    state.running = Some(Running {
                        status: status.clone(),
                        grant,
                        stop,
                        task,
                        recovering,
                    });
                    return Ok(status);
                }
                HttpSessionRequest::Status => {
                    return Ok(state
                        .running
                        .as_ref()
                        .filter(|running| running.grant.is_active() && !running.task.is_finished())
                        .map(|running| HttpSessionStatus {
                            recovering: running.recovering.load(Ordering::SeqCst),
                            ..running.status.clone()
                        })
                        .unwrap_or_default());
                }
                HttpSessionRequest::Stop => Self::stop(&mut state),
            }
        };
        if let Some(task) = task {
            tokio::time::timeout(std::time::Duration::from_secs(6), task)
                .await
                .map_err(|_| "Test access revoked; server drain timed out")?
                .map_err(|_| "Test access revoked; server task failed")?;
        }
        Ok(HttpSessionStatus::default())
    }

    /// Called synchronously by the native window lifecycle, without relying on JS.
    pub fn suspend(&self, suspended: bool) {
        let mut state = self
            .0
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        state.suspended = suspended;
        if suspended {
            Self::stop(&mut state);
        }
    }

    fn stop(state: &mut SessionState) -> Option<JoinHandle<()>> {
        if let Some(running) = state.running.take() {
            running.grant.active.store(false, Ordering::SeqCst);
            let _ = running.stop.send(());
            // Graceful shutdown drains already admitted operations. A lost reply
            // or suspension does not roll back an admitted SQLite write.
            return Some(running.task);
        }
        None
    }
}

impl Drop for HttpSession {
    fn drop(&mut self) {
        self.suspend(true);
    }
}

#[cfg(test)]
mod tests;
