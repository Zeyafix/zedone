use std::sync::Arc;
use tauri::{Runtime, State};
use zedone_adapters::{
    device_control::{Controller, DeviceControl},
    device_host::DeviceHost,
};
use zedone_core::{error::ErrorCode, wire::CommandResponse};

pub struct Devices<R: Runtime> {
    control: Controller<crate::secrets::AndroidSecrets<R>>,
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
        let host = Arc::new(DeviceHost::new(service, store));
        Ok(Self {
            control: Controller::new(host).with_automatic(directory.join("local-session.json")),
        })
    }
}
#[tauri::command]
pub async fn device_host_start(
    state: State<'_, Devices<tauri::Wry>>,
    address: String,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(state
        .control
        .dispatch(
            "host.start",
            serde_json::json!({"address":address})
                .to_string()
                .as_bytes(),
            &credential,
        )
        .await)
}
#[tauri::command]
pub async fn device_host_stop(
    state: State<'_, Devices<tauri::Wry>>,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(state
        .control
        .dispatch("host.stop", b"{}", &credential)
        .await)
}
#[tauri::command]
pub async fn device_session(
    state: State<'_, Devices<tauri::Wry>>,
    operation: String,
    parameters: String,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(state
        .control
        .dispatch(&operation, parameters.as_bytes(), &credential)
        .await)
}
