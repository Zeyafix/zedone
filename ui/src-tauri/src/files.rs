use serde::Deserialize;
use tauri::{Manager, Runtime, plugin::TauriPlugin};
use zedone_core::snapshots::MAX_ARCHIVE_BYTES;

struct Files<R: Runtime>(tauri::plugin::PluginHandle<R>);

#[derive(Deserialize, serde::Serialize)]
pub struct FileReply {
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    archive: Option<String>,
}

async fn access<R: Runtime>(
    app: tauri::AppHandle<R>,
    archive: Option<String>,
) -> Result<FileReply, ()> {
    if archive.as_ref().is_some_and(|hex| {
        hex.is_empty()
            || hex.len() > MAX_ARCHIVE_BYTES * 2
            || !hex.len().is_multiple_of(2)
            || !hex
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    }) {
        return Err(());
    }
    app.state::<Files<R>>()
        .0
        .run_mobile_plugin_async(
            "select",
            serde_json::json!({"archive": archive, "limit": MAX_ARCHIVE_BYTES}),
        )
        .await
        .map_err(|_| ())
}

#[tauri::command]
pub async fn snapshot_file_open(app: tauri::AppHandle) -> Result<FileReply, ()> {
    access(app, None).await
}

#[tauri::command]
pub async fn snapshot_file_save(app: tauri::AppHandle, archive: String) -> Result<FileReply, ()> {
    access(app, Some(archive)).await
}

pub fn plugin<R: Runtime>() -> TauriPlugin<R> {
    tauri::plugin::Builder::new("snapshot-files")
        .setup(|app, api| {
            app.manage(Files(api.register_android_plugin(
                "app.zedone.local",
                "SnapshotFilesPlugin",
            )?));
            Ok(())
        })
        .invoke_handler(|invoke| {
            invoke.resolver.reject("host operation unavailable");
            true
        })
        .build()
}
