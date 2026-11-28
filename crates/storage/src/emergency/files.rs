use super::*;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

pub(super) const STATE_LIMIT: usize = 64 * 1024 * 1024;
pub(super) const RAW_LIMIT: u64 = 64 * 1024 * 1024 * 1024;
pub(super) const LIMITS: migration_backup::BackupLimits = migration_backup::BackupLimits {
    max_database_bytes: 16 * 1024 * 1024 * 1024,
    max_spaces: 10000,
    max_total_snapshot_bytes: 32 * 1024 * 1024 * 1024,
    snapshot: crate::snapshot::SnapshotLimits {
        max_bytes: zedone_core::snapshots::MAX_ARCHIVE_BYTES,
        max_changes: zedone_core::snapshots::MAX_ARCHIVE_CHANGES,
    },
};

pub(super) fn read<T: serde::de::DeserializeOwned>(path: &Path) -> StorageResult<T> {
    serde_json::from_slice(&backup_io::read(path, STATE_LIMIT)?)
        .map_err(|_| StorageError::InvalidDatabase)
}
pub(super) fn save(path: &Path, value: &impl Serialize) -> StorageResult<()> {
    let bytes = serde_json::to_vec(value).map_err(|_| StorageError::Unavailable)?;
    if bytes.len() > STATE_LIMIT {
        return Err(StorageError::Unavailable);
    }
    let parent = path.parent().ok_or(StorageError::Unavailable)?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(io)?;
    file.write_all(&bytes).map_err(io)?;
    file.as_file().sync_all().map_err(io)?;
    file.persist(path)
        .map_err(|_| StorageError::OutcomeUnknown)?;
    backup_io::sync(parent)
}
pub(super) fn io(_: std::io::Error) -> StorageError {
    StorageError::Unavailable
}
pub(super) fn hash(path: &Path) -> StorageResult<crate::problem_state::Entry> {
    let mut file = std::fs::File::open(path).map_err(io)?;
    let mut hasher = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(io)?;
        if n == 0 {
            break;
        }
        bytes = bytes
            .checked_add(n as u64)
            .ok_or(StorageError::Unavailable)?;
        if bytes > RAW_LIMIT {
            return Err(StorageError::Unavailable);
        }
        hasher.update(&buffer[..n]);
    }
    Ok(crate::problem_state::Entry {
        bytes,
        sha256: hasher.finalize().into(),
    })
}
pub(super) fn copy(source: &Path, destination: &Path, limit: u64) -> StorageResult<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(destination)
        .map_err(io)?;
    let mut offset = 0_u64;
    loop {
        let bytes = backup_io::chunk(source, offset)?;
        if bytes.is_empty() {
            break;
        }
        offset = offset
            .checked_add(bytes.len() as u64)
            .ok_or(StorageError::Unavailable)?;
        if offset > limit {
            return Err(StorageError::Unavailable);
        }
        file.write_all(&bytes).map_err(io)?;
    }
    file.sync_all().map_err(io)
}
pub(super) fn replace(source: &Path, target: &Path) -> StorageResult<()> {
    replace_checked(source, target, || Ok(()))
}
pub(super) fn replace_checked(
    source: &Path,
    target: &Path,
    check: impl FnOnce() -> StorageResult<()>,
) -> StorageResult<()> {
    let parent = target.parent().ok_or(StorageError::Unavailable)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(io)?;
    let mut input = std::fs::File::open(source).map_err(io)?;
    std::io::copy(&mut input, &mut temporary).map_err(io)?;
    temporary.as_file().sync_all().map_err(io)?;
    check()?;
    temporary
        .persist(target)
        .map_err(|_| StorageError::OutcomeUnknown)?;
    backup_io::sync(parent)
}
pub(super) fn copy_bundle(
    root: &Path,
    destination: &Path,
) -> StorageResult<zedone_core::maintenance::FullCopy> {
    backup_io::directory(root)?;
    let manifest_bytes = backup_io::read(&root.join("manifest.json"), 3 * 1024 * 1024)?;
    let manifest: serde_json::Value =
        serde_json::from_slice(&manifest_bytes).map_err(|_| StorageError::InvalidDatabase)?;
    let spaces = manifest
        .get("spaces")
        .and_then(|v| v.as_array())
        .ok_or(StorageError::InvalidDatabase)?
        .len();
    if spaces > LIMITS.max_spaces {
        return Err(StorageError::Unavailable);
    }
    let mut names = vec!["manifest.json".into(), "database.sqlite".into()];
    names.extend((0..spaces).map(|i| format!("space-{i}.bin")));
    let entries = backup_io::list(root)?;
    if entries.len() != names.len()
        || entries
            .iter()
            .any(|e| e.directory || !names.contains(&e.name))
    {
        return Err(StorageError::InvalidDatabase);
    }
    std::fs::create_dir(destination).map_err(io)?;
    for name in names {
        let limit = if name == "database.sqlite" {
            LIMITS.max_database_bytes
        } else if name == "manifest.json" {
            3 * 1024 * 1024
        } else {
            LIMITS.snapshot.max_bytes as u64
        };
        copy(&root.join(name.clone()), &destination.join(name), limit)?;
    }
    migration_backup::verify_with(destination, LIMITS, schema::REGISTRY)?;
    if backup_io::read(&root.join("manifest.json"), 3 * 1024 * 1024)? != manifest_bytes {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(zedone_core::maintenance::FullCopy {
        id: root
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or(StorageError::Unavailable)?
            .into(),
        created_at_unix_ms: manifest
            .get("created_at_unix_ms")
            .and_then(|v| v.as_u64())
            .ok_or(StorageError::InvalidDatabase)?,
        spaces,
    })
}
pub(super) fn catalog(root: &Path) -> StorageResult<Vec<zedone_core::maintenance::FullCopy>> {
    let mut result = Vec::new();
    for entry in backup_io::list(root)? {
        if !entry.directory
            || !entry.name.starts_with("zedone-migration-candidate-")
            || !backup_io::valid_name(&entry.name)
        {
            continue;
        }
        let Ok(bytes) = backup_io::read(
            &root.join(&entry.name).join("manifest.json"),
            3 * 1024 * 1024,
        ) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            continue;
        };
        if value.get("version").and_then(|v| v.as_u64()) != Some(1) {
            continue;
        }
        if let (Some(date), Some(spaces)) = (
            value.get("created_at_unix_ms").and_then(|v| v.as_u64()),
            value.get("spaces").and_then(|v| v.as_array()),
        ) && spaces.len() <= LIMITS.max_spaces
        {
            result.push(zedone_core::maintenance::FullCopy {
                id: entry.name,
                created_at_unix_ms: date,
                spaces: spaces.len(),
            });
        }
    }
    result.sort_by(|a, b| {
        b.created_at_unix_ms
            .cmp(&a.created_at_unix_ms)
            .then(a.id.cmp(&b.id))
    });
    Ok(result)
}
