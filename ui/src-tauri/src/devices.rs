//! Native composition; high-level operations, never a secret-store dispatcher.
use std::sync::Arc;
use tauri::{Runtime, State};
use zedone_adapters::{device_host::DeviceHost, network::SecureListener};
use zedone_core::{
    error::{CommandError, ErrorCode},
    replay::Execution,
    wire::CommandResponse,
};

pub struct Devices<R: Runtime> {
    pub host: Arc<DeviceHost<crate::secrets::AndroidSecrets<R>>>,
    listener: tokio::sync::Mutex<Option<SecureListener>>,
}
impl<R: Runtime> Devices<R> {
    pub fn new(
        app: &tauri::AppHandle<R>,
        service: zedone_adapters::Service,
        directory: &std::path::Path,
    ) -> Result<Self, ErrorCode> {
        let namespace = zedone_adapters::device_host::namespace(directory)?;
        let store = crate::secrets::AndroidSecrets::new(app, namespace)
            .map_err(|_| ErrorCode::Unavailable)?;
        Ok(Self {
            host: Arc::new(DeviceHost::new(service, store)),
            listener: tokio::sync::Mutex::new(None),
        })
    }
}
fn result(value: Result<serde_json::Value, ErrorCode>) -> CommandResponse<serde_json::Value> {
    value
        .map(|result| Execution {
            result,
            replayed: false,
        })
        .map_err(|code| CommandError { code })
        .into()
}
#[tauri::command]
pub async fn device_host_start(
    state: State<'_, Devices<tauri::Wry>>,
    address: String,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(result(
        async {
            let mut listening = state.listener.lock().await;
            state.host.authorize(&credential).await?;
            if listening.is_some() {
                return Err(ErrorCode::InvalidRequest);
            }
            let (listener, pin) = state
                .host
                .start(
                    address.parse().map_err(|_| ErrorCode::InvalidRequest)?,
                    &credential,
                )
                .await?;
            let value = serde_json::json!({"address":listener.address,"pin":pin.0});
            *listening = Some(listener);
            Ok(value)
        }
        .await,
    ))
}
#[tauri::command]
pub async fn device_host_stop(
    state: State<'_, Devices<tauri::Wry>>,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(result(
        async {
            let mut listening = state.listener.lock().await;
            state.host.authorize(&credential).await?;
            if let Some(listener) = listening.take() {
                listener.shutdown().await;
            }
            Ok(serde_json::json!({"listening":false}))
        }
        .await,
    ))
}
#[tauri::command]
pub async fn device_session(
    state: State<'_, Devices<tauri::Wry>>,
    operation: String,
    parameters: String,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(state
        .host
        .dispatch(&operation, parameters.as_bytes(), &credential)
        .await)
}
