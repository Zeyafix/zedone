//! The platform composition root. No bootstrap or authority derived from UI.
#![deny(unsafe_code)]

use serde::{Deserialize, Serialize};
use std::{io::Read, path::Path};
use zedone_adapters::Service;

pub mod web;

// Only a pre-provisioned local credential is read. Issuance, OS credential
// facilities, and initial confirmation remain P06, never a shell command.
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
}

impl Shell {
    pub fn new(service: Option<Service>, credential_path: std::path::PathBuf) -> Self {
        Self {
            service,
            credential_path,
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
        zedone_storage::Storage::open(directory.join("zedone.sqlite"))
            .await
            .ok()
            .map(Service::new)
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
    // SAFETY: the only exception is generated JNI/bootstrap (ADR-0024/0042).
    // Platform event loop only; domain/storage/adapters retain forbid(unsafe_code).
    #[tauri::mobile_entry_point]
    pub fn run() {
        use tauri::Manager;
        tauri::Builder::default()
            .setup(|app| {
                let directory = app.path().app_local_data_dir()?.join("product");
                app.manage(tauri::async_runtime::block_on(super::open_shell(
                    &directory,
                )));
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![
                super::shell_session,
                super::zedone_command
            ])
            .run(tauri::generate_context!())
            .expect("ZeDone platform startup failed");
    }
}
