#![deny(unsafe_code)]

#[cfg(all(target_os = "android", not(feature = "android-shell")))]
compile_error!("Android builds require --features android-shell; use scripts/build-ui-android.mjs");

use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};
use zedone_adapters::Service;

pub mod backup_folders;
#[cfg(target_os = "android")]
pub mod devices;
#[cfg(target_os = "android")]
mod files;
#[cfg(all(target_os = "android", feature = "pairing-acceptance"))]
mod pairing_acceptance;
#[cfg(target_os = "android")]
pub mod secrets;
pub mod web;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LocalCredential {
    subject: String,
    key: String,
}

#[derive(Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Session {
    Unconfigured,
    Unavailable,
    Configured { credential: LocalCredential },
}

pub struct Shell {
    pub service: Option<Service>,
    credential_path: std::path::PathBuf,
    pub devices: Option<std::sync::Arc<dyn zedone_adapters::device_control::DeviceControl>>,
    pub maintenance: Option<std::sync::Arc<zedone_adapters::maintenance::Maintenance>>,
    resumed_devices: std::sync::Mutex<
        Option<(
            u64,
            std::sync::Arc<dyn zedone_adapters::device_control::DeviceControl>,
        )>,
    >,
    _maintenance_listener: Option<zedone_adapters::maintenance::LocalListener>,
}

impl Shell {
    pub fn new(service: Option<Service>, credential_path: std::path::PathBuf) -> Self {
        #[cfg(windows)]
        let devices = service.as_ref().and_then(|service| {
            let directory = credential_path.parent()?;
            let namespace = zedone_adapters::device_host::namespace(directory).ok()?;
            let store = zedone_adapters::secrets::windows::WindowsSecrets::new(
                namespace,
                &directory.join("host-locks"),
            );
            Some(std::sync::Arc::new(
                zedone_adapters::device_control::Controller::new(std::sync::Arc::new(
                    zedone_adapters::device_host::DeviceHost::new(service.clone(), store),
                ))
                .with_automatic(credential_path.clone()),
            )
                as std::sync::Arc<
                    dyn zedone_adapters::device_control::DeviceControl,
                >)
        });
        #[cfg(not(windows))]
        let devices = None;
        Self {
            service,
            credential_path,
            devices,
            maintenance: None,
            resumed_devices: std::sync::Mutex::new(None),
            _maintenance_listener: None,
        }
    }

    pub fn current_service(&self) -> Option<Service> {
        match &self.maintenance {
            Some(value) => value.service(),
            None => self.service.clone(),
        }
    }
    pub fn maintenance_session(&self) -> Option<zedone_storage::emergency::LocalSession> {
        self.maintenance.as_ref()?.local_session().ok()
    }
    pub fn generation(&self) -> u64 {
        self.maintenance.as_ref().map_or(0, |m| m.generation())
    }
    pub fn device_control(
        &self,
    ) -> Option<std::sync::Arc<dyn zedone_adapters::device_control::DeviceControl>> {
        if self.maintenance.is_none() {
            return self.devices.clone();
        }
        #[cfg(windows)]
        {
            let generation = self.generation();
            let mut saved = self.resumed_devices.lock().ok()?;
            if let Some((prior, value)) = &*saved
                && *prior == generation
                && self.current_service().is_some()
            {
                return Some(value.clone());
            }
            let service = self.current_service()?;
            let directory = self.credential_path.parent()?;
            let namespace = zedone_adapters::device_host::namespace(directory).ok()?;
            let store = zedone_adapters::secrets::windows::WindowsSecrets::new(
                namespace,
                &directory.join("host-locks"),
            );
            let control = std::sync::Arc::new(
                zedone_adapters::device_control::Controller::new(std::sync::Arc::new(
                    zedone_adapters::device_host::DeviceHost::new(service, store),
                ))
                .with_automatic(self.credential_path.clone()),
            )
                as std::sync::Arc<dyn zedone_adapters::device_control::DeviceControl>;
            self.maintenance.as_ref()?.register_device(control.clone());
            *saved = Some((generation, control.clone()));
            Some(control)
        }
        #[cfg(not(windows))]
        {
            let _ = &self.resumed_devices;
            None
        }
    }
    pub async fn dispatch(
        &self,
        operation: &str,
        body: &[u8],
        key: &str,
    ) -> zedone_core::wire::CommandResponse<serde_json::Value> {
        if operation == "maintenance.execute"
            && let Some(value) = &self.maintenance
        {
            return value.dispatch(operation, body, key).await;
        }
        if let Some(service) = self.current_service() {
            return service.dispatch(operation, body, key).await;
        }
        Err(zedone_core::error::CommandError {
            code: zedone_core::error::ErrorCode::Unavailable,
        })
        .into()
    }

    pub fn session(&self) -> Session {
        if self.current_service().is_none() {
            return Session::Unavailable;
        }
        let mut text = String::new();
        let mut file = match std::fs::File::open(&self.credential_path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Session::Unconfigured;
            }
            Err(_) => return Session::Unavailable,
        };
        if file.by_ref().take(4097).read_to_string(&mut text).is_err() || text.len() > 4096 {
            return Session::Unavailable;
        }
        let Ok(credential) = serde_json::from_str::<LocalCredential>(&text) else {
            return Session::Unavailable;
        };
        let hex = |value: &str, length| {
            value.len() == length
                && value
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
        };
        if !hex(&credential.subject, 32) || !hex(&credential.key, 64) {
            return Session::Unavailable;
        }
        Session::Configured { credential }
    }
}

pub async fn open_shell(directory: &Path) -> Shell {
    let mut failure = None;
    let service = if std::fs::create_dir_all(directory).is_ok() {
        match zedone_storage::Storage::open(directory.join("zedone.sqlite")).await {
            Ok(storage) => match storage.recover_editing_sessions().await {
                Ok(()) => Some(Service::new(storage).with_daily_backups()),
                Err(error) => {
                    failure = Some(format!("{error}"));
                    let _ = storage.shutdown().await;
                    None
                }
            },
            Err(error) => {
                failure = Some(format!("{error}"));
                None
            }
        }
    } else {
        None
    };
    let mut shell = Shell::new(service.clone(), directory.join("local-session.json"));
    match zedone_adapters::maintenance::Maintenance::open(
        directory.join("zedone.sqlite"),
        service,
        failure,
    )
    .await
    {
        Ok(controller) => {
            if let Some(device) = shell.devices.take() {
                controller.register_device(device.clone());
                *shell.resumed_devices.lock().expect("new lock") = Some((0, device));
            }
            shell._maintenance_listener = controller.start_loopback().await.ok();
            shell.maintenance = Some(controller);
        }
        Err(_) => {
            shell.service = None;
            shell.devices = None;
        }
    }
    shell
}

#[cfg(target_os = "android")]
#[tauri::command]
fn shell_session(state: tauri::State<'_, Shell>) -> Session {
    state.session()
}

#[cfg(target_os = "android")]
#[tauri::command]
fn maintenance_session(
    state: tauri::State<'_, Shell>,
) -> Option<zedone_storage::emergency::LocalSession> {
    state.maintenance_session()
}

#[cfg(target_os = "android")]
#[tauri::command]
async fn zedone_command(
    state: tauri::State<'_, Shell>,
    invocation: zedone_core::wire::IpcInvocation,
    credential: String,
) -> Result<String, ()> {
    Ok(serde_json::to_string(
        &state
            .dispatch(
                &invocation.operation,
                invocation.request.as_bytes(),
                &credential,
            )
            .await,
    )
    .expect("serializable response"))
}

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
mod entry_point {
    #[tauri::mobile_entry_point]
    pub fn run() {
        use tauri::Manager;
        tauri::Builder::default()
            .plugin(tauri_plugin_barcode_scanner::init())
            .plugin(super::secrets::plugin())
            .plugin(super::files::plugin())
            .plugin(
                tauri_plugin_opener::Builder::new()
                    .open_js_links_on_click(false)
                    .build(),
            )
            .setup(|app| {
                let base = app.path().app_local_data_dir()?;
                #[cfg(feature = "pairing-acceptance")]
                tauri::async_runtime::block_on(super::pairing_acceptance::prepare_agent(&base))
                    .map_err(|_| "agent acceptance preparation failed")?;
                let directory = base.join("product");
                #[cfg(feature = "pairing-acceptance")]
                let directory = super::pairing_acceptance::ui_directory(&base).unwrap_or(directory);
                let shell = tauri::async_runtime::block_on(super::open_shell(&directory));
                app.manage(tauri::async_runtime::block_on(async {
                    super::devices::Devices::new(app.handle(), &directory, &shell)
                }));
                app.manage(shell);
                #[cfg(feature = "pairing-acceptance")]
                super::pairing_acceptance::start(app.handle().clone());
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                super::shell_session,
                super::maintenance_session,
                super::zedone_command,
                super::files::snapshot_file_open,
                super::files::snapshot_file_save,
                super::files::backup_folder_select,
                super::files::maintenance_folder_select,
                super::devices::device_host_start,
                super::devices::device_host_stop,
                super::devices::device_session
            ])
            .run(tauri::generate_context!())
            .expect("ZeDone platform startup failed");
    }
}
