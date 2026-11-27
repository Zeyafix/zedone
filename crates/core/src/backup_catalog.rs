use crate::{
    access::{Clock, Credential, RequiredAccess, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, ReplayTransaction, StateVersion},
    snapshots::SpaceArchive,
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupFolder {
    pub id: String,
    pub label: String,
    pub writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupFolders {
    pub folders: Vec<BackupFolder>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupLocation {
    pub folder: String,
    pub version: StateVersion,
    pub is_default: bool,
    pub cleanup_pending: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogCopy {
    pub id: String,
    pub space: String,
    pub names: Vec<String>,
    pub kind: String,
    pub created_at_unix_ms: u64,
    pub digest: StateVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupCatalog {
    pub copies: Vec<CatalogCopy>,
    pub unreadable: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogFolder {
    pub folder: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogRead {
    pub folder: String,
    pub copy: String,
    pub expected: StateVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogDelete {
    pub folder: String,
    pub copy: String,
    pub expected: StateVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ChangeBackupLocation {
    pub folder: String,
    pub expected: StateVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectBackupFolders {}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectBackupLocation {}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CleanupBackupLocation {
    pub expected: StateVersion,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct BackupDeleted {
    pub deleted: bool,
    pub cleanup_pending: bool,
}

pub trait CatalogStore: ReplayTransaction {
    fn backup_forget(
        &mut self,
        subject: &str,
        p: &ForgetBackupFolder,
    ) -> CommandResult<BackupFolders>;
    fn authorize_folder(&mut self, subject: &str, folder: &str, write: bool) -> CommandResult<()>;
    fn backup_folders(
        &mut self,
        subject: &str,
        p: &InspectBackupFolders,
    ) -> CommandResult<BackupFolders>;
    fn backup_location(
        &mut self,
        subject: &str,
        p: &InspectBackupLocation,
    ) -> CommandResult<BackupLocation>;
    fn backup_catalog(&mut self, subject: &str, p: &CatalogFolder) -> CommandResult<BackupCatalog>;
    fn backup_read(&mut self, subject: &str, p: &CatalogRead) -> CommandResult<SpaceArchive>;
    fn backup_delete(&mut self, subject: &str, p: &CatalogDelete) -> CommandResult<BackupDeleted>;
    fn backup_change(
        &mut self,
        subject: &str,
        p: &ChangeBackupLocation,
    ) -> CommandResult<BackupLocation>;
    fn backup_cleanup(
        &mut self,
        subject: &str,
        p: &CleanupBackupLocation,
    ) -> CommandResult<BackupLocation>;
}

impl Command for InspectBackupFolders {
    const NAME: &'static str = "backups.folders";
    type Output = BackupFolders;
}
#[utoipa::path(post, path = "/v1/commands/backups.folders", operation_id = "backups_folders", request_body = CommandRequest<InspectBackupFolders>, responses((status = 200, body = CommandResponse<BackupFolders>)), security(("access_key" = [])))]
pub fn folders(
    tx: &mut impl CatalogStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectBackupFolders>,
) -> CommandResult<Execution<BackupFolders>> {
    if request.operation != InspectBackupFolders::NAME || request.id == [0; 16] {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, subject| {
            Ok(Execution {
                result: tx.backup_folders(subject, &request.parameters)?,
                replayed: false,
            })
        },
    )
}

impl Command for InspectBackupLocation {
    const NAME: &'static str = "backups.location.inspect";
    type Output = BackupLocation;
}
#[utoipa::path(post, path = "/v1/commands/backups.location.inspect", operation_id = "backups_location_inspect", request_body = CommandRequest<InspectBackupLocation>, responses((status = 200, body = CommandResponse<BackupLocation>)), security(("access_key" = [])))]
pub fn location(
    tx: &mut impl CatalogStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<InspectBackupLocation>,
) -> CommandResult<Execution<BackupLocation>> {
    if request.operation != InspectBackupLocation::NAME || request.id == [0; 16] {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, subject| {
            Ok(Execution {
                result: tx.backup_location(subject, &request.parameters)?,
                replayed: false,
            })
        },
    )
}

impl Command for CatalogFolder {
    const NAME: &'static str = "backups.catalog";
    type Output = BackupCatalog;
}
#[utoipa::path(post, path = "/v1/commands/backups.catalog", operation_id = "backups_catalog", request_body = CommandRequest<CatalogFolder>, responses((status = 200, body = CommandResponse<BackupCatalog>)), security(("access_key" = [])))]
pub fn catalog(
    tx: &mut impl CatalogStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CatalogFolder>,
) -> CommandResult<Execution<BackupCatalog>> {
    if request.operation != CatalogFolder::NAME || request.id == [0; 16] {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, subject| {
            tx.authorize_folder(subject, &request.parameters.folder, false)?;
            Ok(Execution {
                result: tx.backup_catalog(subject, &request.parameters)?,
                replayed: false,
            })
        },
    )
}

impl Command for CatalogRead {
    const NAME: &'static str = "backups.catalog.read";
    type Output = SpaceArchive;
}
#[utoipa::path(post, path = "/v1/commands/backups.catalog.read", operation_id = "backups_catalog_read", request_body = CommandRequest<CatalogRead>, responses((status = 200, body = CommandResponse<SpaceArchive>)), security(("access_key" = [])))]
pub fn read(
    tx: &mut impl CatalogStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CatalogRead>,
) -> CommandResult<Execution<SpaceArchive>> {
    if request.operation != CatalogRead::NAME || request.id == [0; 16] {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, subject| {
            tx.authorize_folder(subject, &request.parameters.folder, false)?;
            Ok(Execution {
                result: tx.backup_read(subject, &request.parameters)?,
                replayed: false,
            })
        },
    )
}

impl Command for CatalogDelete {
    const NAME: &'static str = "backups.catalog.delete";
    type Output = BackupDeleted;
}
#[utoipa::path(post, path = "/v1/commands/backups.catalog.delete", operation_id = "backups_catalog_delete", request_body = CommandRequest<CatalogDelete>, responses((status = 200, body = CommandResponse<BackupDeleted>)), security(("access_key" = [])))]
pub fn delete(
    tx: &mut impl CatalogStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CatalogDelete>,
) -> CommandResult<Execution<BackupDeleted>> {
    if request.operation != CatalogDelete::NAME || request.id == [0; 16] {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, subject| {
            tx.authorize_folder(subject, &request.parameters.folder, true)?;
            replay::execute_as(
                tx,
                credential,
                clock,
                &[RequiredAccess::ActiveGrant],
                request.id,
                &request.parameters,
                |tx, subject| tx.backup_delete(subject, &request.parameters),
            )
        },
    )
}

impl Command for ChangeBackupLocation {
    const NAME: &'static str = "backups.location.change";
    type Output = BackupLocation;
}
#[utoipa::path(post, path = "/v1/commands/backups.location.change", operation_id = "backups_location_change", request_body = CommandRequest<ChangeBackupLocation>, responses((status = 200, body = CommandResponse<BackupLocation>)), security(("access_key" = [])))]
pub fn change(
    tx: &mut impl CatalogStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ChangeBackupLocation>,
) -> CommandResult<Execution<BackupLocation>> {
    if request.operation != ChangeBackupLocation::NAME || request.id == [0; 16] {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, subject| {
            tx.authorize_folder(subject, &request.parameters.folder, true)?;
            replay::execute_as(
                tx,
                credential,
                clock,
                &[RequiredAccess::ActiveGrant],
                request.id,
                &request.parameters,
                |tx, subject| tx.backup_change(subject, &request.parameters),
            )
        },
    )
}

impl Command for CleanupBackupLocation {
    const NAME: &'static str = "backups.location.cleanup";
    type Output = BackupLocation;
}
#[utoipa::path(post, path = "/v1/commands/backups.location.cleanup", operation_id = "backups_location_cleanup", request_body = CommandRequest<CleanupBackupLocation>, responses((status = 200, body = CommandResponse<BackupLocation>)), security(("access_key" = [])))]
pub fn cleanup(
    tx: &mut impl CatalogStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CleanupBackupLocation>,
) -> CommandResult<Execution<BackupLocation>> {
    if request.operation != CleanupBackupLocation::NAME || request.id == [0; 16] {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    with_authorized(
        tx,
        credential,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, _subject| {
            replay::execute_as(
                tx,
                credential,
                clock,
                &[RequiredAccess::ActiveGrant],
                request.id,
                &request.parameters,
                |tx, subject| tx.backup_cleanup(subject, &request.parameters),
            )
        },
    )
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ForgetBackupFolder {
    pub folder: String,
}
impl Command for ForgetBackupFolder {
    const NAME: &'static str = "backups.folder.forget";
    type Output = BackupFolders;
}
#[utoipa::path(post,path="/v1/commands/backups.folder.forget",operation_id="backups_folder_forget",request_body=CommandRequest<ForgetBackupFolder>,responses((status=200,body=CommandResponse<BackupFolders>)),security(("access_key"=[])))]
pub fn forget(
    tx: &mut impl CatalogStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ForgetBackupFolder>,
) -> CommandResult<Execution<BackupFolders>> {
    if request.operation != ForgetBackupFolder::NAME {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        });
    }
    replay::execute_as(
        tx,
        credential,
        clock,
        &[RequiredAccess::ActiveGrant],
        request.id,
        &request.parameters,
        |tx, subject| tx.backup_forget(subject, &request.parameters),
    )
}
