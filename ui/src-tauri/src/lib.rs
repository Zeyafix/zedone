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
        }
    }

    pub fn session(&self) -> Session {
        if self.service.is_none() {
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
    let service = if std::fs::create_dir_all(directory).is_ok() {
        match zedone_storage::Storage::open(directory.join("zedone.sqlite")).await {
            Ok(storage) => {
                if storage.recover_editing_sessions().await.is_ok() {
                    Some(Service::new(storage).with_daily_backups())
                } else {
                    let _ = storage.shutdown().await;
                    None
                }
            }
            Err(_) => None,
        }
    } else {
        None
    };
    Shell::new(service, directory.join("local-session.json"))
}

#[cfg(target_os = "android")]
#[tauri::command]
fn shell_session(state: tauri::State<'_, Shell>) -> Session {
    state.session()
}

#[cfg(target_os = "android")]
#[tauri::command]
async fn zedone_command(
    state: tauri::State<'_, Shell>,
    invocation: zedone_core::wire::IpcInvocation,
    credential: String,
) -> Result<String, ()> {
    Ok(if let Some(service) = &state.service {
        zedone_adapters::ipc::invoke(
            service,
            &invocation.operation,
            &invocation.request,
            &credential,
        )
        .await
    } else {
        serde_json::to_string(
            &zedone_core::wire::CommandResponse::<serde_json::Value>::from(Err(
                zedone_core::error::CommandError {
                    code: zedone_core::error::ErrorCode::Unavailable,
                },
            )),
        )
        .expect("fixed serializable error")
    })
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
                if let Some(service) = &shell.service {
                    if let Ok(devices) = tauri::async_runtime::block_on(async {
                        super::devices::Devices::new(app.handle(), service.clone(), &directory)
                    }) {
                        app.manage(devices);
                    }
                }
                app.manage(shell);
                #[cfg(feature = "pairing-acceptance")]
                super::pairing_acceptance::start(app.handle().clone());
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                super::shell_session,
                super::zedone_command,
                super::files::snapshot_file_open,
                super::files::snapshot_file_save,
                super::files::backup_folder_select,
                super::devices::device_host_start,
                super::devices::device_host_stop,
                super::devices::device_session
            ])
            .run(tauri::generate_context!())
            .expect("ZeDone platform startup failed");
    }
}
