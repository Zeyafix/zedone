use crate::{
    Storage, StorageError, StorageResult, WriteContext, backup_io as io,
    snapshot::{SnapshotLimits, SpaceSnapshot},
};
use automerge::{Automerge, ROOT, ReadDoc};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use zedone_core::{
    access::{Credential, RequiredAccess, with_authorized},
    backup_catalog::*,
    command::CommandResult,
    error::{CommandError, ErrorCode},
    replay::{StateVersion, require_current},
    snapshots::{MAX_ARCHIVE_BYTES, MAX_ARCHIVE_CHANGES, SpaceArchive},
};

mod migration;
const KEY: &[u8] = b"\0zedone/internal/backup-location/v1";
const CAP: &str = "\0zedone/internal/backup-folder/v1/";
const LIMITS: SnapshotLimits = SnapshotLimits {
    max_bytes: MAX_ARCHIVE_BYTES,
    max_changes: MAX_ARCHIVE_CHANGES,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Capability {
    subject: String,
    id: String,
    root: String,
    label: String,
    writable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRecord {
    name: String,
    size: usize,
    digest: [u8; 32],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CopyRecord {
    id: String,
    files: Vec<FileRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Transfer {
    source: String,
    target: String,
    copies: Vec<CopyRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Deletion {
    root: String,
    copy: CopyRecord,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Location {
    #[serde(deserialize_with = "required_option")]
    current: Option<String>,
    generation: u64,
    #[serde(deserialize_with = "required_option")]
    pending: Option<Transfer>,
    deletions: Vec<Deletion>,
}

fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}

fn unavailable() -> StorageError {
    StorageError::Unavailable
}
fn rejected(code: ErrorCode) -> StorageError {
    CommandError { code }.into()
}
fn load<T: DeserializeOwned>(c: &Connection, key: &[u8]) -> StorageResult<Option<T>> {
    let row: Option<(Vec<u8>,Vec<u8>)> = c
        .query_row(
            "SELECT substr(request,1,128),substr(result,1,4194305) FROM command_results WHERE key=?1",
            [key],
            |r| Ok((r.get(0)?,r.get(1)?)),
        )
        .optional()?;
    row.map(|(request, bytes)| {
        if request != KEY || bytes.len() > 4194304 {
            return Err(StorageError::InvalidDatabase);
        }
        serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidDatabase)
    })
    .transpose()
}
fn save(c: &Connection, key: &[u8], value: &impl Serialize) -> StorageResult<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| unavailable())?;
    if bytes.len() > 4194304 {
        return Err(unavailable());
    }
    c.execute("INSERT INTO command_results VALUES (?1,?2,?3) ON CONFLICT(key) DO UPDATE SET request=excluded.request,result=excluded.result", (key, KEY, bytes))?;
    Ok(())
}
fn location(c: &Connection) -> StorageResult<Location> {
    let loc: Location = load(c, KEY)?.unwrap_or_default();
    if let Some(root) = &loc.current {
        valid_root(Path::new(root))?;
    }
    if loc.deletions.len() > 64 {
        return Err(StorageError::InvalidDatabase);
    }
    for deletion in &loc.deletions {
        valid_root(Path::new(&deletion.root))?;
        valid_record(&deletion.copy)?;
    }
    if let Some(pending) = &loc.pending {
        valid_root(Path::new(&pending.source))?;
        valid_root(Path::new(&pending.target))?;
        let active = loc
            .current
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or(default_folder(c)?);
        if loc.generation == 0
            || pending.copies.len() > 10000
            || Path::new(&pending.target) != active
            || pending.source == pending.target
        {
            return Err(StorageError::InvalidDatabase);
        }
        let mut ids = std::collections::BTreeSet::new();
        for copy in &pending.copies {
            valid_record(copy)?;
            if !ids.insert(&copy.id) {
                return Err(StorageError::InvalidDatabase);
            }
        }
    }
    Ok(loc)
}

fn valid_root(root: &Path) -> StorageResult<()> {
    let text = root.to_str().ok_or(StorageError::InvalidDatabase)?;
    let valid = if let Some(hex) = text.strip_prefix("saf-") {
        !hex.is_empty()
            && hex.len() <= 16384
            && hex.len().is_multiple_of(2)
            && hex.bytes().all(|b| b.is_ascii_hexdigit())
    } else {
        root.is_absolute()
            && !text.contains('\0')
            && !root
                .components()
                .any(|p| matches!(p, std::path::Component::ParentDir))
    };
    if !valid {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}
fn valid_record(record: &CopyRecord) -> StorageResult<()> {
    if !recognized(&record.id) || record.files.is_empty() || record.files.len() > 10002 {
        return Err(StorageError::InvalidDatabase);
    }
    let mut names = std::collections::BTreeSet::new();
    for file in &record.files {
        let permitted = if record.id == migration::INDEX {
            file.name == migration::INDEX
        } else if migration::name(&record.id) {
            matches!(file.name.as_str(), "database.sqlite" | "manifest.json")
                || migration::snapshot_name(&file.name)
        } else {
            matches!(file.name.as_str(), "space.bin" | "commit.json")
        };
        if !permitted || !io::valid_name(&file.name) || file.size == 0 || !names.insert(&file.name)
        {
            return Err(StorageError::InvalidDatabase);
        }
    }
    Ok(())
}
pub(crate) fn default_folder(c: &Connection) -> StorageResult<PathBuf> {
    let db = std::fs::canonicalize(c.path().ok_or_else(unavailable)?).map_err(|_| unavailable())?;
    let name = db.file_name().ok_or_else(unavailable)?.to_string_lossy();
    Ok(db.with_file_name(format!("{name}.backups")))
}
pub(crate) fn folder(c: &Connection) -> StorageResult<PathBuf> {
    Ok(location(c)?
        .current
        .map(PathBuf::from)
        .unwrap_or(default_folder(c)?))
}
pub(crate) fn writable(c: &Connection) -> StorageResult<()> {
    if location(c)?.pending.is_some() {
        return Err(unavailable());
    }
    Ok(())
}
fn root_text(root: &Path) -> StorageResult<String> {
    root.to_str().map(str::to_owned).ok_or_else(unavailable)
}
fn cap_key(id: &str) -> Vec<u8> {
    format!("{CAP}{id}").into_bytes()
}
fn capability(c: &Connection, subject: &str, id: &str, write: bool) -> StorageResult<Capability> {
    if !crate::columns::valid_id(id) {
        return Err(rejected(ErrorCode::AccessDenied));
    }
    let cap: Capability = load(c, &cap_key(id))?.ok_or(rejected(ErrorCode::AccessDenied))?;
    valid_root(Path::new(&cap.root))?;
    if cap.id != id || cap.subject != subject || (write && !cap.writable) {
        return Err(rejected(ErrorCode::AccessDenied));
    }
    Ok(cap)
}
fn capabilities(c: &Connection, subject: &str) -> StorageResult<Vec<Capability>> {
    let mut upper = CAP.as_bytes().to_vec();
    upper.push(255);
    let mut q =
        c.prepare("SELECT key FROM command_results WHERE key>=?1 AND key<?2 ORDER BY key")?;
    let keys = q
        .query_map((CAP.as_bytes(), upper), |r| r.get::<_, Vec<u8>>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut result = Vec::new();
    for key in keys {
        let cap: Capability = load(c, &key)?.ok_or(StorageError::InvalidDatabase)?;
        if cap.subject == subject {
            result.push(cap);
        }
    }
    Ok(result)
}
fn location_view(c: &Connection, subject: &str) -> StorageResult<BackupLocation> {
    let loc = location(c)?;
    let root = root_text(&folder(c)?)?;
    let label = capabilities(c, subject)?
        .into_iter()
        .find(|cap| cap.root == root)
        .map(|cap| cap.label)
        .unwrap_or_else(|| "Папка копий этой установки".into());
    Ok(BackupLocation {
        folder: label,
        version: version(&loc)?,
        is_default: loc.current.is_none(),
        cleanup_pending: loc.pending.is_some() || !loc.deletions.is_empty(),
    })
}
fn version(loc: &Location) -> StorageResult<StateVersion> {
    Ok(StateVersion(
        Sha256::digest(
            serde_json::to_vec(&(loc.current.as_ref(), loc.generation))
                .map_err(|_| unavailable())?,
        )
        .into(),
    ))
}

pub(crate) fn recognized(name: &str) -> bool {
    io::valid_name(name)
        && (name.starts_with("zedone-daily-")
            || name.starts_with("zedone-restore-candidate-")
            || migration::name(name)
            || name == migration::INDEX)
}
fn file_records(path: &Path) -> StorageResult<Vec<FileRecord>> {
    if path
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| migration::name(n) || n == migration::INDEX)
    {
        return migration::records(path);
    }
    let mut entries = io::list(path)?;
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    if entries.is_empty()
        || entries.len() > 2
        || entries
            .iter()
            .any(|e| e.directory || !matches!(e.name.as_str(), "space.bin" | "commit.json"))
    {
        return Err(StorageError::InvalidDatabase);
    }
    entries
        .into_iter()
        .map(|e| {
            let bytes = io::read(
                &path.join(&e.name),
                if e.name == "space.bin" {
                    MAX_ARCHIVE_BYTES
                } else {
                    4096
                },
            )?;
            Ok(FileRecord {
                name: e.name,
                size: bytes.len(),
                digest: Sha256::digest(bytes).into(),
            })
        })
        .collect()
}
fn inspect_copy(root: &Path, id: &str) -> StorageResult<(CatalogCopy, CopyRecord)> {
    if let Some((group, file)) = id.split_once('@') {
        if !migration::name(group) || !migration::snapshot_name(file) {
            return Err(rejected(ErrorCode::InvalidRequest));
        }
        let (copies, record) = migration::copies(root, group)?;
        return Ok((
            copies
                .into_iter()
                .find(|c| c.id == id)
                .ok_or(rejected(ErrorCode::NotFound))?,
            record,
        ));
    }
    if !recognized(id) {
        return Err(rejected(ErrorCode::InvalidRequest));
    }
    let path = root.join(id);
    let files = file_records(&path)?;
    let bytes = io::read(&path.join("space.bin"), MAX_ARCHIVE_BYTES)?;
    let snapshot =
        SpaceSnapshot::decode(&bytes, LIMITS).map_err(|_| StorageError::InvalidDatabase)?;
    if snapshot.metadata().created_at_unix_ms
        > zedone_core::access_commands::MAX_EXPIRY_UNIX_MS as u64
    {
        return Err(StorageError::InvalidDatabase);
    }
    if id.starts_with("zedone-daily-") {
        let marker: serde_json::Value =
            serde_json::from_slice(&io::read(&path.join("commit.json"), 4096)?)
                .map_err(|_| StorageError::InvalidDatabase)?;
        if marker["id"] != id
            || marker["space"] != snapshot.metadata().space
            || marker["created_at_unix_ms"] != snapshot.metadata().created_at_unix_ms
            || marker["bytes"] != bytes.len()
            || marker["digest"] != serde_json::json!(<[u8; 32]>::from(Sha256::digest(&bytes)))
        {
            return Err(StorageError::InvalidDatabase);
        }
    }
    let mut graph = Automerge::new();
    graph.apply_changes(snapshot.changes().iter().cloned())?;
    let names = graph
        .get_all(ROOT, "name")?
        .into_iter()
        .filter_map(|(v, _)| v.as_str().map(str::to_owned))
        .collect();
    Ok((
        CatalogCopy {
            id: id.into(),
            space: snapshot.metadata().space.clone(),
            names,
            kind: if id.starts_with("zedone-daily-") {
                "daily"
            } else {
                "recovery"
            }
            .into(),
            created_at_unix_ms: snapshot.metadata().created_at_unix_ms,
            digest: StateVersion(Sha256::digest(&bytes).into()),
        },
        CopyRecord {
            id: id.into(),
            files,
        },
    ))
}
fn push_catalog(
    result: &mut BackupCatalog,
    used: &mut usize,
    copy: CatalogCopy,
) -> StorageResult<()> {
    *used = used
        .checked_add(serde_json::to_vec(&copy).map_err(|_| unavailable())?.len())
        .ok_or_else(unavailable)?;
    if *used > MAX_ARCHIVE_BYTES {
        return Err(unavailable());
    }
    result.copies.push(copy);
    Ok(())
}
fn catalog(c: &Connection, root: &Path) -> StorageResult<BackupCatalog> {
    let mut used = 0;
    let deleted = location(c)?.deletions;
    let mut result = BackupCatalog {
        copies: vec![],
        unreadable: 0,
    };
    for entry in io::list(root)? {
        if !entry.directory
            || !recognized(&entry.name)
            || deleted
                .iter()
                .any(|d| Path::new(&d.root) == root && d.copy.id == entry.name)
        {
            continue;
        }
        if migration::name(&entry.name) {
            match migration::copies(root, &entry.name) {
                Ok((copies, _)) => {
                    for copy in copies {
                        push_catalog(&mut result, &mut used, copy)?;
                    }
                }
                Err(_) => result.unreadable += 1,
            }
            continue;
        }
        match inspect_copy(root, &entry.name) {
            Ok((copy, _)) => push_catalog(&mut result, &mut used, copy)?,
            Err(_) => result.unreadable += 1,
        }
    }
    if serde_json::to_vec(&result)
        .map_err(|_| unavailable())?
        .len()
        > MAX_ARCHIVE_BYTES
    {
        return Err(unavailable());
    }
    result.copies.sort_by(|a, b| {
        a.space
            .cmp(&b.space)
            .then(b.created_at_unix_ms.cmp(&a.created_at_unix_ms))
            .then(a.id.cmp(&b.id))
    });
    Ok(result)
}
fn check_record(root: &Path, record: &CopyRecord) -> StorageResult<()> {
    valid_root(root)?;
    valid_record(record)?;
    let actual = file_records(&root.join(&record.id))?;
    if serde_json::to_vec(&actual).map_err(|_| unavailable())?
        != serde_json::to_vec(&record.files).map_err(|_| unavailable())?
    {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}
fn remove_record(root: &Path, record: &CopyRecord) -> StorageResult<()> {
    valid_root(root)?;
    valid_record(record)?;
    let dir = root.join(&record.id);
    if record.id == migration::INDEX {
        if io::exists(&dir)? {
            let actual = migration::measure(&dir)?;
            let expected = &record.files[0];
            if actual.size != expected.size || actual.digest != expected.digest {
                return Err(StorageError::InvalidDatabase);
            }
            io::remove_file(&dir)?;
        }
        return Ok(());
    }
    if !io::exists(&dir)? {
        return Ok(());
    }
    for entry in io::list(&dir)? {
        let original = record
            .files
            .iter()
            .find(|f| f.name == entry.name)
            .ok_or(StorageError::InvalidDatabase)?;
        if entry.directory {
            return Err(StorageError::InvalidDatabase);
        }
        let actual = migration::measure(&dir.join(&entry.name))?;
        if actual.size != original.size || actual.digest != original.digest {
            return Err(StorageError::InvalidDatabase);
        }
    }
    for file in &record.files {
        io::remove_file(&dir.join(&file.name))?;
    }
    io::remove_dir(&dir)
}
fn cleanup(c: &Connection) -> StorageResult<()> {
    let mut loc = location(c)?;
    if let Some(pending) = &loc.pending {
        for record in &pending.copies {
            check_record(Path::new(&pending.target), record)?;
        }
        for record in &pending.copies {
            remove_record(Path::new(&pending.source), record)?;
        }
        loc.pending = None;
    }
    for deletion in &loc.deletions {
        remove_record(Path::new(&deletion.root), &deletion.copy)?;
    }
    loc.deletions.clear();
    save(c, KEY, &loc)
}

impl Storage {
    pub async fn select_backup_folder(
        &self,
        secret: Vec<u8>,
        root: Option<String>,
        writable: bool,
    ) -> StorageResult<BackupFolder> {
        self.timed_transact(move |ctx, clock| {
            let result = with_authorized(
                ctx,
                &Credential(&secret),
                &[RequiredAccess::ActiveGrant],
                clock,
                |ctx, subject| {
                    let result = (|| {
                        let path = match root {
                            Some(root) if root.starts_with("saf-") => PathBuf::from(root),
                            Some(root) => std::fs::canonicalize(root).map_err(|_| unavailable())?,
                            None => {
                                let root = default_folder(ctx.connection())?;
                                if writable && !io::exists(&root)? {
                                    io::mkdir(&root)?;
                                }
                                root
                            }
                        };
                        io::directory(&path)?;
                        let current = folder(ctx.connection())?;
                        if writable
                            && current == default_folder(ctx.connection())?
                            && !io::exists(&current)?
                        {
                            io::mkdir(&current)?;
                        }
                        let mut roots = vec![path];
                        if writable && !roots.contains(&current) {
                            roots.push(current);
                        }
                        let mut selected = None;
                        let mut existing = capabilities(ctx.connection(), subject)?;
                        for root in roots {
                            if let Some(cap) = existing.iter().find(|cap| {
                                Path::new(&cap.root) == root && cap.writable == writable
                            }) {
                                if selected.is_none() {
                                    selected = Some(BackupFolder {
                                        id: cap.id.clone(),
                                        label: cap.label.clone(),
                                        writable: cap.writable,
                                    });
                                }
                                continue;
                            }
                            if existing.len() >= 64 {
                                return Err(unavailable());
                            }
                            let id = io::random_id()?;
                            let label = if root == default_folder(ctx.connection())? {
                                "Папка по умолчанию".into()
                            } else if root.to_string_lossy().starts_with("saf-") {
                                io::label(&root)?
                            } else {
                                root_text(&root)?
                            };
                            let cap = Capability {
                                subject: subject.into(),
                                id: id.clone(),
                                root: root_text(&root)?,
                                label: label.clone(),
                                writable,
                            };
                            save(ctx.connection(), &cap_key(&id), &cap)?;
                            existing.push(cap);
                            if selected.is_none() {
                                selected = Some(BackupFolder {
                                    id,
                                    label,
                                    writable,
                                });
                            }
                        }
                        selected.ok_or_else(unavailable)
                    })();
                    Ok(ctx.checked(result)?)
                },
            );
            Ok(result?)
        })
        .await
    }
    pub async fn cleanup_backup_location(&self) -> StorageResult<()> {
        self.transact(|ctx| cleanup(ctx.connection())).await
    }
}

impl CatalogStore for WriteContext<'_> {
    fn backup_forget(
        &mut self,
        subject: &str,
        p: &ForgetBackupFolder,
    ) -> CommandResult<BackupFolders> {
        let result = (|| {
            capability(self.connection(), subject, &p.folder, false)?;
            self.connection().execute(
                "DELETE FROM command_results WHERE key=?1",
                [cap_key(&p.folder)],
            )?;
            Ok(())
        })();
        self.checked(result)?;
        self.backup_folders(subject, &InspectBackupFolders {})
    }
    fn authorize_folder(&mut self, subject: &str, id: &str, write: bool) -> CommandResult<()> {
        self.checked(capability(self.connection(), subject, id, write))?;
        Ok(())
    }

    fn backup_folders(
        &mut self,
        subject: &str,
        _: &InspectBackupFolders,
    ) -> CommandResult<BackupFolders> {
        let result = capabilities(self.connection(), subject).map(|caps| BackupFolders {
            folders: caps
                .into_iter()
                .map(|cap| BackupFolder {
                    id: cap.id,
                    label: cap.label,
                    writable: cap.writable,
                })
                .collect(),
        });
        Ok(self.checked(result)?)
    }
    fn backup_location(
        &mut self,
        subject: &str,
        _: &InspectBackupLocation,
    ) -> CommandResult<BackupLocation> {
        Ok(self.checked(location_view(self.connection(), subject))?)
    }
    fn backup_catalog(&mut self, subject: &str, p: &CatalogFolder) -> CommandResult<BackupCatalog> {
        let result = (|| {
            let cap = capability(self.connection(), subject, &p.folder, false)?;
            catalog(self.connection(), Path::new(&cap.root))
        })();
        Ok(self.checked(result)?)
    }
    fn backup_read(&mut self, subject: &str, p: &CatalogRead) -> CommandResult<SpaceArchive> {
        let result = (|| {
            let cap = capability(self.connection(), subject, &p.folder, false)?;
            let (copy, _) = inspect_copy(Path::new(&cap.root), &p.copy)?;
            require_current(p.expected, copy.digest)?;
            let path = if let Some((group, file)) = p.copy.split_once('@') {
                Path::new(&cap.root).join(group).join(file)
            } else {
                Path::new(&cap.root).join(&p.copy).join("space.bin")
            };
            let bytes = io::read(&path, MAX_ARCHIVE_BYTES)?;
            if Sha256::digest(&bytes).as_slice() != p.expected.0 {
                return Err(rejected(ErrorCode::StaleState));
            }
            Ok(SpaceArchive {
                archive: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            })
        })();
        Ok(self.checked(result)?)
    }
    fn backup_delete(&mut self, subject: &str, p: &CatalogDelete) -> CommandResult<BackupDeleted> {
        let result = (|| {
            let cap = capability(self.connection(), subject, &p.folder, true)?;
            let root = Path::new(&cap.root);
            let (copy, record) = inspect_copy(root, &p.copy)?;
            require_current(p.expected, copy.digest)?;
            if copy.kind == "migration" {
                return Err(rejected(ErrorCode::AccessDenied));
            }
            let mut loc = location(self.connection())?;
            if loc.pending.is_some() || loc.deletions.len() >= 64 {
                return Err(unavailable());
            }
            if root == folder(self.connection())? {
                crate::daily_backup::forget_copy(self.connection(), &copy.space, &copy.id)?;
                crate::recovery::forget_copy(self.connection(), &copy.space, &copy.id)?;
            }
            loc.deletions.push(Deletion {
                root: cap.root,
                copy: record,
            });
            save(self.connection(), KEY, &loc)?;
            Ok(BackupDeleted {
                deleted: true,
                cleanup_pending: true,
            })
        })();
        Ok(self.checked(result)?)
    }
    fn backup_change(
        &mut self,
        subject: &str,
        p: &ChangeBackupLocation,
    ) -> CommandResult<BackupLocation> {
        let result = (|| {
            let cap = capability(self.connection(), subject, &p.folder, true)?;
            let source = folder(self.connection())?;
            let target = PathBuf::from(&cap.root);
            let mut loc = location(self.connection())?;
            require_current(p.expected, version(&loc)?)?;
            if !capabilities(self.connection(), subject)?
                .iter()
                .any(|cap| cap.writable && Path::new(&cap.root) == source)
            {
                return Err(rejected(ErrorCode::AccessDenied));
            }
            if loc.pending.is_some() || !loc.deletions.is_empty() {
                return Err(unavailable());
            }
            if source == target {
                return location_view(self.connection(), subject);
            }
            if target.starts_with(&source) || source.starts_with(&target) {
                return Err(rejected(ErrorCode::InvalidRequest));
            }
            io::directory(&source)?;
            io::directory(&target)?;
            let mut copies = Vec::new();
            let mut entries = io::list(&source)?;
            entries.sort_by_key(|entry| entry.name == migration::INDEX);
            for entry in entries {
                if !recognized(&entry.name) {
                    continue;
                }
                let record = if migration::name(&entry.name) {
                    migration::copies(&source, &entry.name)?.1
                } else if entry.name == migration::INDEX {
                    CopyRecord {
                        id: entry.name.clone(),
                        files: migration::records(&source.join(&entry.name))?,
                    }
                } else {
                    if !entry.directory {
                        return Err(StorageError::InvalidDatabase);
                    }
                    inspect_copy(&source, &entry.name)?.1
                };
                migration::transfer_record(&source, &target, &record)?;
                copies.push(record);
            }
            io::sync(&target)?;
            loc.current = if target == default_folder(self.connection())? {
                None
            } else {
                Some(cap.root.clone())
            };
            loc.generation = loc.generation.checked_add(1).ok_or_else(unavailable)?;
            loc.pending = Some(Transfer {
                source: root_text(&source)?,
                target: cap.root,
                copies,
            });
            save(self.connection(), KEY, &loc)?;
            location_view(self.connection(), subject)
        })();
        Ok(self.checked(result)?)
    }
    fn backup_cleanup(
        &mut self,
        subject: &str,
        p: &CleanupBackupLocation,
    ) -> CommandResult<BackupLocation> {
        let result = (|| {
            let loc = location(self.connection())?;
            require_current(p.expected, version(&loc)?)?;
            let root = folder(self.connection())?;
            if !capabilities(self.connection(), subject)?
                .iter()
                .any(|cap| cap.writable && Path::new(&cap.root) == root)
            {
                return Err(rejected(ErrorCode::AccessDenied));
            }
            cleanup(self.connection())?;
            location_view(self.connection(), subject)
        })();
        Ok(self.checked(result)?)
    }
}

#[cfg(test)]
mod tests;
