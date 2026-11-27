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
            zedone_storage::backup_io::register_tree_backend(std::sync::Arc::new(BackupTrees(
                app.clone(),
            )))?;
            Ok(())
        })
        .invoke_handler(|invoke| {
            invoke.resolver.reject("host operation unavailable");
            true
        })
        .build()
}

struct BackupTrees<R: Runtime>(tauri::AppHandle<R>);
impl<R: Runtime> zedone_storage::backup_io::TreeBackend for BackupTrees<R> {
    fn execute(
        &self,
        request: zedone_storage::backup_io::TreeRequest,
    ) -> zedone_storage::StorageResult<zedone_storage::backup_io::TreeReply> {
        self.0
            .state::<Files<R>>()
            .0
            .run_mobile_plugin("tree", request)
            .map_err(|_| zedone_storage::StorageError::Unavailable)
    }
}

#[derive(Deserialize)]
struct SelectedTree {
    status: String,
    root: Option<String>,
}

#[tauri::command]
pub async fn backup_folder_select(
    app: tauri::AppHandle,
    state: tauri::State<'_, crate::Shell>,
    selection: crate::backup_folders::FolderSelection,
    credential: String,
) -> Result<crate::backup_folders::FolderSelectionReply, ()> {
    let crate::Session::Configured { credential: local } = state.session() else {
        return Err(());
    };
    if local.key != credential {
        return Err(());
    }
    let service = state.service.as_ref().ok_or(())?;
    let body=serde_json::to_vec(&serde_json::json!({"id":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1],"operation":"backups.folders","parameters":{}})).map_err(|_|())?;
    if !matches!(
        service
            .dispatch("backups.folders", &body, &credential)
            .await,
        zedone_core::wire::CommandResponse::Ok(_)
    ) {
        return Err(());
    }
    let root = if selection.default {
        None
    } else {
        let reply: SelectedTree = app
            .state::<Files<tauri::Wry>>()
            .0
            .run_mobile_plugin_async(
                "selectFolder",
                serde_json::json!({"writable":selection.writable}),
            )
            .await
            .map_err(|_| ())?;
        if reply.status == "cancelled" {
            return Ok(crate::backup_folders::FolderSelectionReply::Cancelled);
        }
        if reply.status != "ok" {
            return Err(());
        }
        Some(reply.root.ok_or(())?)
    };
    let folder = service
        .select_backup_folder(&credential, root, selection.writable)
        .await
        .map_err(|_| ())?;
    Ok(crate::backup_folders::FolderSelectionReply::Selected { folder })
}
