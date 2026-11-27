use crate::{StorageError, StorageResult};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{Read, Write},
    path::Path,
    sync::{Arc, Mutex},
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entry {
    pub name: String,
    pub directory: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TreeRequest {
    pub root: String,
    pub path: Vec<String>,
    pub operation: String,
    pub bytes: Option<Vec<u8>>,
    pub limit: usize,
    pub offset: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct TreeReply {
    pub entries: Vec<Entry>,
    pub bytes: Vec<u8>,
    pub exists: bool,
}

pub trait TreeBackend: Send + Sync {
    fn execute(&self, request: TreeRequest) -> StorageResult<TreeReply>;
}

static TREE: Mutex<Option<Arc<dyn TreeBackend>>> = Mutex::new(None);

pub fn register_tree_backend(backend: Arc<dyn TreeBackend>) -> StorageResult<()> {
    *TREE.lock().map_err(|_| StorageError::Unavailable)? = Some(backend);
    Ok(())
}
fn backend() -> StorageResult<Arc<dyn TreeBackend>> {
    TREE.lock()
        .map_err(|_| StorageError::Unavailable)?
        .clone()
        .ok_or(StorageError::Unavailable)
}

pub fn label(path: &Path) -> StorageResult<String> {
    if let Some(reply) = external(path, "label", None, 4096) {
        let bytes = reply?.bytes;
        if bytes.len() > 4096 {
            return Err(StorageError::Unavailable);
        }
        return String::from_utf8(bytes).map_err(|_| StorageError::InvalidDatabase);
    }
    path.to_str()
        .map(str::to_owned)
        .ok_or(StorageError::Unavailable)
}

fn io<T>(r: std::io::Result<T>) -> StorageResult<T> {
    r.map_err(|_| StorageError::Unavailable)
}

fn external(
    path: &Path,
    operation: &str,
    bytes: Option<&[u8]>,
    limit: usize,
) -> Option<StorageResult<TreeReply>> {
    let text = path.to_str()?;
    let (root, rest) = text.split_once('/').unwrap_or((text, ""));
    let root = root.strip_prefix("saf-")?;
    Some((|| {
        if root.is_empty() || !root.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(StorageError::InvalidDatabase);
        }
        let path: Vec<String> = rest
            .split('/')
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
            .collect();
        if path.iter().any(|s| !valid_name(s)) {
            return Err(StorageError::InvalidDatabase);
        }
        backend()?.execute(TreeRequest {
            root: root.into(),
            path,
            operation: operation.into(),
            bytes: bytes.map(Vec::from),
            limit,
            offset: 0,
        })
    })())
}

pub fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 180
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'.' | b'_'))
        && name != "."
        && name != ".."
}

pub fn directory(path: &Path) -> StorageResult<()> {
    if let Some(r) = external(path, "directory", None, 0) {
        return r.map(|_| ());
    }
    if io(fs::symlink_metadata(path))?.file_type().is_dir() {
        Ok(())
    } else {
        Err(StorageError::InvalidDatabase)
    }
}

pub fn exists(path: &Path) -> StorageResult<bool> {
    if let Some(r) = external(path, "exists", None, 0) {
        return r.map(|v| v.exists);
    }
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(_) => Err(StorageError::Unavailable),
    }
}

pub fn list(path: &Path) -> StorageResult<Vec<Entry>> {
    if let Some(r) = external(path, "list", None, 0) {
        return r.map(|v| v.entries);
    }
    directory(path)?;
    let mut entries = Vec::new();
    for entry in io(fs::read_dir(path))? {
        if entries.len() >= 10000 {
            return Err(StorageError::Unavailable);
        }
        let entry = io(entry)?;
        let kind = io(entry.file_type())?;
        entries.push(Entry {
            name: entry.file_name().to_string_lossy().into(),
            directory: kind.is_dir(),
        });
    }
    Ok(entries)
}

pub fn read(path: &Path, limit: usize) -> StorageResult<Vec<u8>> {
    if let Some(r) = external(path, "read", None, limit) {
        return r.and_then(|v| {
            if v.bytes.len() <= limit {
                Ok(v.bytes)
            } else {
                Err(StorageError::InvalidDatabase)
            }
        });
    }
    if !io(fs::symlink_metadata(path))?.file_type().is_file() {
        return Err(StorageError::InvalidDatabase);
    }
    let mut bytes = Vec::new();
    io(io(fs::File::open(path))?
        .take(limit as u64 + 1)
        .read_to_end(&mut bytes))?;
    if bytes.len() > limit {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(bytes)
}

pub fn mkdir(path: &Path) -> StorageResult<()> {
    if let Some(r) = external(path, "mkdir", None, 0) {
        return r.map(|_| ());
    }
    io(fs::create_dir(path))?;
    sync(path.parent().ok_or(StorageError::Unavailable)?)
}

pub fn write(path: &Path, bytes: &[u8], create: bool) -> StorageResult<()> {
    if let Some(r) = external(
        path,
        if create { "create" } else { "write" },
        Some(bytes),
        0,
    ) {
        return r.map(|_| ());
    }
    if !create && exists(path)? && !io(fs::symlink_metadata(path))?.file_type().is_file() {
        return Err(StorageError::InvalidDatabase);
    }
    let mut file = io(fs::OpenOptions::new()
        .write(true)
        .create_new(create)
        .create(!create)
        .truncate(!create)
        .open(path))?;
    io(file.write_all(bytes))?;
    io(file.sync_all())?;
    sync(path.parent().ok_or(StorageError::Unavailable)?)
}

pub fn remove_file(path: &Path) -> StorageResult<()> {
    if !exists(path)? {
        return Ok(());
    }
    if let Some(r) = external(path, "remove_file", None, 0) {
        return r.map(|_| ());
    }
    if !io(fs::symlink_metadata(path))?.file_type().is_file() {
        return Err(StorageError::InvalidDatabase);
    }
    io(fs::remove_file(path))?;
    sync(path.parent().ok_or(StorageError::Unavailable)?)
}

pub fn remove_dir(path: &Path) -> StorageResult<()> {
    if !exists(path)? {
        return Ok(());
    }
    if let Some(r) = external(path, "remove_dir", None, 0) {
        return r.map(|_| ());
    }
    directory(path)?;
    io(fs::remove_dir(path))?;
    sync(path.parent().ok_or(StorageError::Unavailable)?)
}

pub fn sync(path: &Path) -> StorageResult<()> {
    if let Some(r) = external(path, "sync", None, 0) {
        return r.map(|_| ());
    }
    crate::restore_backup::sync_directory(path)
}

pub fn random_id() -> StorageResult<String> {
    let mut id = [0; 16];
    getrandom::fill(&mut id).map_err(|_| StorageError::Unavailable)?;
    Ok(id.iter().map(|b| format!("{b:02x}")).collect())
}

pub fn chunk(path: &Path, offset: u64) -> StorageResult<Vec<u8>> {
    use std::io::{Seek, SeekFrom};
    if let Some(reply) = external_chunk(path, "read_chunk", None, offset) {
        return reply.map(|r| r.bytes);
    }
    if !io(fs::symlink_metadata(path))?.file_type().is_file() {
        return Err(StorageError::InvalidDatabase);
    }
    let mut file = io(fs::File::open(path))?;
    io(file.seek(SeekFrom::Start(offset)))?;
    let mut bytes = Vec::new();
    io(file.take(65536).read_to_end(&mut bytes))?;
    Ok(bytes)
}
pub fn append(path: &Path, offset: u64, bytes: &[u8]) -> StorageResult<()> {
    if let Some(reply) = external_chunk(path, "append", Some(bytes), offset) {
        return reply.map(|_| ());
    }
    if !io(fs::symlink_metadata(path))?.file_type().is_file() {
        return Err(StorageError::InvalidDatabase);
    }
    let mut file = io(fs::OpenOptions::new().append(true).open(path))?;
    if io(file.metadata())?.len() != offset {
        return Err(StorageError::InvalidDatabase);
    }
    io(file.write_all(bytes))?;
    io(file.sync_all())
}
fn external_chunk(
    path: &Path,
    operation: &str,
    bytes: Option<&[u8]>,
    offset: u64,
) -> Option<StorageResult<TreeReply>> {
    let text = path.to_str()?;
    let (root, rest) = text.split_once('/')?;
    let root = root.strip_prefix("saf-")?;
    Some((|| {
        let parts: Vec<String> = rest.split('/').map(str::to_owned).collect();
        if parts.iter().any(|s| !valid_name(s)) {
            return Err(StorageError::InvalidDatabase);
        }
        let result = backend()?.execute(TreeRequest {
            root: root.into(),
            path: parts,
            operation: operation.into(),
            bytes: bytes.map(Vec::from),
            limit: 65536,
            offset,
        })?;
        if result.bytes.len() > 65536 {
            return Err(StorageError::InvalidDatabase);
        }
        Ok(result)
    })())
}
