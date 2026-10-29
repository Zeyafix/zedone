//! Android transport only; models and the operation belong to the shared core.

#![deny(unsafe_code)]

#[cfg(target_os = "android")]
#[tauri::command]
fn echo(request: zedone_prototype_core::EchoRequest) -> zedone_prototype_core::EchoResponse {
    zedone_prototype_core::echo(request)
}

#[cfg(target_os = "android")]
#[tauri::command]
async fn storage(
    request: zedone_prototype_core::storage::StorageRequest,
    state: tauri::State<'_, zedone_prototype_core::storage::Storage>,
) -> Result<
    zedone_prototype_core::storage::StorageResponse,
    zedone_prototype_core::storage::StorageError,
> {
    state.execute(request).await
}

#[cfg(target_os = "android")]
#[allow(unsafe_code)]
mod entry_point {
    // SAFETY: only Tauri's generated JNI/bootstrap uses this exception (ADR-0024).
    // Package/library names are fixed by the checked Android project; no raw
    // pointers or business operations are implemented in this module.
    #[tauri::mobile_entry_point]
    pub fn run() {
        use tauri::Manager;
        tauri::Builder::default()
            .setup(|app| {
                let directory = app.path().app_local_data_dir()?.join("p01-05-synthetic");
                app.manage(zedone_prototype_core::storage::Storage::new(directory));
                Ok(())
            })
            .invoke_handler(tauri::generate_handler![super::echo, super::storage])
            .run(tauri::generate_context!())
            // Startup cannot continue without its platform event loop/WebView.
            .expect("Android prototype failed to start");
    }
}
