use crate::Service;

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

#[cfg(all(target_os = "android", feature = "tauri"))]
pub fn tauri_handler<R: tauri::Runtime>()
-> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static {
    tauri::generate_handler![zedone_command]
}
