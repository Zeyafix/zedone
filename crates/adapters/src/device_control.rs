//! Shared shell control. Credentials and policy remain in DeviceHost/Service.
use crate::{device_host::DeviceHost, network::SecureListener, secrets::SecretStore};
use std::{future::Future, pin::Pin, sync::Arc};
use zedone_core::{
    error::{CommandError, ErrorCode},
    replay::Execution,
    wire::CommandResponse,
};
type Response = CommandResponse<serde_json::Value>;
pub trait DeviceControl: Send + Sync {
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
}
impl<S: SecretStore + 'static> Controller<S> {
    pub fn new(host: Arc<DeviceHost<S>>) -> Self {
        Self {
            host,
            listener: tokio::sync::Mutex::new(None),
        }
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
impl<S: SecretStore + 'static> DeviceControl for Controller<S> {
    fn dispatch<'a>(
        &'a self,
        operation: &'a str,
        body: &'a [u8],
        key: &'a str,
    ) -> Pin<Box<dyn Future<Output = Response> + Send + 'a>> {
        Box::pin(async move {
            if operation.starts_with("session.") || operation.starts_with("offer.") {
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
