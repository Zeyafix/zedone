use serde::{Deserialize, Serialize};
use zedone_core::{
    backup_catalog::BackupFolder,
    error::{CommandError, ErrorCode},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FolderSelection {
    pub writable: bool,
    pub default: bool,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum FolderSelectionReply {
    Selected { folder: BackupFolder },
    Cancelled,
}

#[cfg(windows)]
pub async fn choose(
    service: &zedone_adapters::Service,
    key: &str,
    selection: FolderSelection,
) -> Result<FolderSelectionReply, CommandError> {
    let root = if selection.default {
        None
    } else {
        let output=tokio::task::spawn_blocking(|| {
            std::process::Command::new("powershell.exe").args(["-NoProfile","-STA","-WindowStyle","Hidden","-Command",r#"[Console]::OutputEncoding = New-Object System.Text.UTF8Encoding($false); Add-Type -AssemblyName System.Windows.Forms; $dialog = New-Object System.Windows.Forms.FolderBrowserDialog; $dialog.Description = 'ZeDone: выберите папку резервных копий'; $dialog.ShowNewFolderButton = $true; if ($dialog.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) { ConvertTo-Json -Compress -InputObject $dialog.SelectedPath } else { 'null' }; $dialog.Dispose()"#]).output()
        }).await.map_err(|_|CommandError{code:ErrorCode::Unavailable})?.map_err(|_|CommandError{code:ErrorCode::Unavailable})?;
        if !output.status.success() || output.stdout.len() > 32768 {
            return Err(CommandError {
                code: ErrorCode::Unavailable,
            });
        }
        let path: Option<String> =
            serde_json::from_slice(&output.stdout).map_err(|_| CommandError {
                code: ErrorCode::Unavailable,
            })?;
        let Some(path) = path else {
            return Ok(FolderSelectionReply::Cancelled);
        };
        Some(path)
    };
    let folder = service
        .select_backup_folder(key, root, selection.writable)
        .await?;
    Ok(FolderSelectionReply::Selected { folder })
}

#[cfg(not(windows))]
pub async fn choose(
    _service: &zedone_adapters::Service,
    _key: &str,
    _selection: FolderSelection,
) -> Result<FolderSelectionReply, CommandError> {
    Err(CommandError {
        code: ErrorCode::Unavailable,
    })
}
