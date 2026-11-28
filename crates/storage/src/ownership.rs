use crate::{StorageError, StorageResult};
use std::{
    fs::{self, File, OpenOptions},
    path::{Path, PathBuf},
};

pub(crate) struct Ownership(File);

pub(crate) fn database_path(path: &Path) -> StorageResult<PathBuf> {
    if path.exists() {
        return fs::canonicalize(path).map_err(|_| StorageError::Unavailable);
    }
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    Ok(fs::canonicalize(parent)
        .map_err(|_| StorageError::Unavailable)?
        .join(path.file_name().ok_or(StorageError::Unavailable)?))
}
pub(crate) fn sibling(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
impl Ownership {
    fn acquire(path: &Path, exclusive: bool) -> StorageResult<Self> {
        let path = sibling(&database_path(path)?, ".owner.lock");
        if path.exists()
            && !fs::symlink_metadata(&path)
                .map_err(|_| StorageError::Unavailable)?
                .file_type()
                .is_file()
        {
            return Err(StorageError::Unavailable);
        }
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|_| StorageError::Unavailable)?;
        if exclusive {
            file.try_lock()
        } else {
            file.try_lock_shared()
        }
        .map_err(|_| StorageError::Unavailable)?;
        Ok(Self(file))
    }
    pub(crate) fn shared(path: &Path) -> StorageResult<Self> {
        Self::acquire(path, false)
    }
    pub(crate) fn exclusive(path: &Path) -> StorageResult<Self> {
        Self::acquire(path, true)
    }
}
impl Drop for Ownership {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
