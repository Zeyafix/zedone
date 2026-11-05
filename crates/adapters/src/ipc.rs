use crate::Service;

/// The same strict JSON bytes as HTTP/CLI; do not deserialize to Value in a
/// transport (that would erase duplicate fields before the Rust decoder).
pub async fn invoke(service: &Service, operation: &str, request: &str, credential: &str) -> String {
    serde_json::to_string(
        &service
            .dispatch(operation, request.as_bytes(), credential)
            .await,
    )
    .expect("JSON response contains only serializable Rust data")
}

#[cfg(all(target_os = "android", feature = "tauri"))]
#[tauri::command]
pub async fn zedone_command(
    state: tauri::State<'_, Service>,
    invocation: zedone_core::wire::IpcInvocation,
    credential: String,
) -> Result<String, ()> {
    let service = state.inner().clone();
    Ok(invoke(
        &service,
        &invocation.operation,
        &invocation.request,
        &credential,
    )
    .await)
}

/// Attach to the P05 host after manage(Service); no app/window or trusted
/// bootstrap is created by this adapter. The generated IPC bridge is compiled
/// for both Android ABIs by product-check --all-features.
#[cfg(all(target_os = "android", feature = "tauri"))]
pub fn tauri_handler<R: tauri::Runtime>()
-> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![zedone_command]
}
