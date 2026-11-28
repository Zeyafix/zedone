use std::sync::{Arc, Mutex};
use tauri::{Runtime, State};
use zedone_adapters::{
    device_control::{Controller, DeviceControl},
    device_host::DeviceHost,
};
use zedone_core::{error::ErrorCode, wire::CommandResponse};

type Control<R> = Arc<Controller<crate::secrets::AndroidSecrets<R>>>;
pub struct Devices<R: Runtime> {
    app: tauri::AppHandle<R>,
    directory: std::path::PathBuf,
    control: Mutex<Option<(u64, Control<R>)>>,
}
impl<R: Runtime> Devices<R> {
    pub fn new(
        app: &tauri::AppHandle<R>,
        directory: &std::path::Path,
        shell: &crate::Shell,
    ) -> Self {
        let value = Self {
            app: app.clone(),
            directory: directory.to_owned(),
            control: Mutex::new(None),
        };
        let _ = value.controller(shell);
        value
    }
    fn controller(&self, shell: &crate::Shell) -> Result<Control<R>, ErrorCode> {
        let service = shell.current_service().ok_or(ErrorCode::Unavailable)?;
        let generation = shell.generation();
        let mut saved = self.control.lock().map_err(|_| ErrorCode::Unavailable)?;
        if let Some((prior, control)) = &*saved
            && *prior == generation
        {
            return Ok(control.clone());
        }
        let namespace = zedone_adapters::device_host::namespace(&self.directory)?;
        let store = crate::secrets::AndroidSecrets::new(&self.app, namespace)
            .map_err(|_| ErrorCode::Unavailable)?;
        let control = Arc::new(
            Controller::new(Arc::new(DeviceHost::new(service, store)))
                .with_automatic(self.directory.join("local-session.json")),
        );
        if let Some(maintenance) = &shell.maintenance {
            maintenance.register_device(control.clone());
        }
        *saved = Some((generation, control.clone()));
        Ok(control)
    }
}
#[tauri::command]
pub async fn device_host_start(
    state: State<'_, Devices<tauri::Wry>>,
    shell: State<'_, crate::Shell>,
    address: String,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(state
        .controller(&shell)
        .map_err(|_| ())?
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
    shell: State<'_, crate::Shell>,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(state
        .controller(&shell)
        .map_err(|_| ())?
        .dispatch("host.stop", b"{}", &credential)
        .await)
}
#[tauri::command]
pub async fn device_session(
    state: State<'_, Devices<tauri::Wry>>,
    shell: State<'_, crate::Shell>,
    operation: String,
    parameters: String,
    credential: String,
) -> Result<CommandResponse<serde_json::Value>, ()> {
    Ok(state
        .controller(&shell)
        .map_err(|_| ())?
        .dispatch(&operation, parameters.as_bytes(), &credential)
        .await)
}
