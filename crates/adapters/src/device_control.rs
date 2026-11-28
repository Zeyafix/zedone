use crate::{device_host::DeviceHost, network::SecureListener, secrets::SecretStore};
use std::{future::Future, pin::Pin, sync::Arc};
use zedone_core::{
    error::{CommandError, ErrorCode},
    replay::Execution,
    wire::CommandResponse,
};
type Response = CommandResponse<serde_json::Value>;
pub trait DeviceControl: Send + Sync {
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async {})
    }
    fn dispatch<'a>(
        &'a self,
        operation: &'a str,
        body: &'a [u8],
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>>;
}
pub struct Controller<S> {
    pub host: Arc<DeviceHost<S>>,
    listener: tokio::sync::Mutex<Option<(SecureListener, crate::channel::Fingerprint)>>,
    automatic: Option<tokio::task::JoinHandle<()>>,
}
impl<S: SecretStore + 'static> Controller<S> {
    pub fn new(host: Arc<DeviceHost<S>>) -> Self {
        Self {
            host,
            listener: tokio::sync::Mutex::new(None),
            automatic: None,
        }
    }
    pub fn with_automatic(mut self, path: std::path::PathBuf) -> Self {
        let host = Arc::downgrade(&self.host);
        self.automatic = Some(tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(5));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let Some(host) = host.upgrade() else {
                    break;
                };
                let path = path.clone();
                let key = tokio::task::spawn_blocking(move || {
                    use std::io::Read;
                    let mut bytes = crate::secrets::Secret::new(Vec::new());
                    std::fs::File::open(path)
                        .ok()?
                        .take(4097)
                        .read_to_end(&mut bytes)
                        .ok()?;
                    if bytes.len() > 4096 {
                        return None;
                    }
                    #[derive(serde::Deserialize)]
                    #[serde(deny_unknown_fields)]
                    struct Session {
                        subject: String,
                        key: String,
                    }
                    let key = if let Ok(value) = serde_json::from_slice::<Session>(&bytes) {
                        if value.subject.len() != 32 {
                            return None;
                        }
                        value.key
                    } else {
                        std::str::from_utf8(&bytes)
                            .ok()?
                            .trim_end_matches(['\r', '\n'])
                            .to_owned()
                    };
                    crate::decode_key(&key).ok()?;
                    Some(crate::secrets::Secret::new(key.into_bytes()))
                })
                .await
                .ok()
                .flatten();
                if let Some(key) = key.and_then(|s| std::str::from_utf8(&s).ok().map(str::to_owned))
                {
                    let _ = host.exchange_automatic(&key).await;
                }
            }
        }));
        self
    }
    async fn control(
        &self,
        operation: &str,
        body: &[u8],
        key: &str,
    ) -> Result<serde_json::Value, ErrorCode> {
        if body.len() > crate::MAX_REQUEST_BYTES {
            return Err(ErrorCode::InvalidRequest);
        }
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Empty {}
        #[derive(serde::Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Start {
            address: std::net::SocketAddr,
        }
        let mut active = self.listener.lock().await;
        self.host.authorize(key).await?;
        match operation {
            "host.start" => {
                let input: Start =
                    serde_json::from_slice(body).map_err(|_| ErrorCode::InvalidRequest)?;
                if let Some((listener, _)) = &*active {
                    if listener.address != input.address {
                        return Err(ErrorCode::InvalidRequest);
                    }
                } else {
                    *active = Some(self.host.start(input.address, key).await?);
                }
            }
            "host.stop" => {
                let _: Empty =
                    serde_json::from_slice(body).map_err(|_| ErrorCode::InvalidRequest)?;
                if let Some((listener, _)) = active.take() {
                    listener.shutdown().await;
                }
            }
            "host.status" => {
                let _: Empty =
                    serde_json::from_slice(body).map_err(|_| ErrorCode::InvalidRequest)?;
            }
            _ => return Err(ErrorCode::InvalidRequest),
        }
        Ok(match &*active {
            Some((listener, pin)) => {
                serde_json::json!({"listening":true,"address":listener.address,"pin":pin.0})
            }
            None => serde_json::json!({"listening":false}),
        })
    }
}
impl<S> Drop for Controller<S> {
    fn drop(&mut self) {
        if let Some(task) = self.automatic.take() {
            task.abort();
        }
    }
}
impl<S: SecretStore + 'static> DeviceControl for Controller<S> {
    fn shutdown(&self) -> Pin<Box<dyn Future<Output = ()> + Send + '_>> {
        Box::pin(async move {
            if let Some(task) = &self.automatic {
                task.abort();
            }
            if let Some((listener, _)) = self.listener.lock().await.take() {
                listener.shutdown().await;
            }
        })
    }
    fn dispatch<'a>(
        &'a self,
        operation: &'a str,
        body: &'a [u8],
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            if operation.starts_with("session.")
                || operation.starts_with("offer.")
                || operation.starts_with("exchange.")
            {
                return self.host.dispatch(operation, body, key).await;
            }
            self.control(operation, body, key)
                .await
                .map(|result| Execution {
                    result,
                    replayed: false,
                })
                .map_err(|code| CommandError { code })
                .into()
        })
    }
}
