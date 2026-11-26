use super::*;
use crate::{restore_backup, snapshot::SpaceSnapshot};
use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::Path,
};
const COMMIT: &str = "commit.json";
fn io<T>(value: std::io::Result<T>) -> StorageResult<T> {
    value.map_err(|_| unavailable())
}
pub(super) fn folder(c: &Connection) -> StorageResult<PathBuf> {
    let path = io(fs::canonicalize(c.path().ok_or_else(unavailable)?))?;
    let name = path.file_name().ok_or_else(unavailable)?.to_string_lossy();
    Ok(path.with_file_name(format!("{name}.backups")))
}
fn directory(path: &Path) -> StorageResult<()> {
    if io(fs::symlink_metadata(path))?.file_type().is_dir() {
        Ok(())
    } else {
        Err(invalid())
    }
}
fn namespace(g: &Global) -> String {
    format!("zedone-daily-{}-", g.installation)
}
fn prefix(g: &Global, space: &str) -> String {
    format!("{}{space}-", namespace(g))
}
fn path(c: &Connection, r: &Reference) -> StorageResult<PathBuf> {
    let g = global(c)?;
    if !r
        .id
        .strip_prefix(&prefix(&g, &r.space))
        .is_some_and(|tail| {
            !tail.is_empty() && tail.len() <= 64 && tail.bytes().all(|b| b.is_ascii_alphanumeric())
        })
    {
        return Err(invalid());
    }
    let root = folder(c)?;
    directory(&root)?;
    let path = root.join(&r.id);
    directory(&path)?;
    Ok(path)
}
pub(super) fn capture(
    c: &Connection,
    g: &mut Global,
    space: &str,
    date: u64,
) -> StorageResult<Reference> {
    let root = folder(c)?;
    if !g.folder_initialized && !root.exists() {
        io(fs::create_dir(&root))?;
    }
    directory(&root)?;
    g.folder_initialized = true;
    let candidate = restore_backup::capture_named(
        c,
        &root,
        space.into(),
        date,
        LIMITS,
        &prefix(g, space),
        |_, _| Ok(()),
    )?;
    Ok(Reference {
        id: candidate
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(unavailable)?
            .into(),
        space: space.into(),
        created_at_unix_ms: date,
        bytes: candidate.bytes,
        digest: candidate.digest,
    })
}
pub(super) fn read(c: &Connection, r: &Reference) -> StorageResult<Vec<u8>> {
    let file = path(c, r)?.join(restore_backup::ARCHIVE);
    if !io(fs::symlink_metadata(&file))?.file_type().is_file() || r.bytes > MAX_ARCHIVE_BYTES {
        return Err(invalid());
    }
    let mut bytes = Vec::new();
    io(io(File::open(file))?
        .take(r.bytes as u64 + 1)
        .read_to_end(&mut bytes))?;
    if bytes.len() != r.bytes || Sha256::digest(&bytes).as_slice() != r.digest {
        return Err(invalid());
    }
    let snapshot = SpaceSnapshot::decode(&bytes, LIMITS).map_err(|_| invalid())?;
    if snapshot.metadata().space != r.space
        || snapshot.metadata().created_at_unix_ms != r.created_at_unix_ms
    {
        return Err(invalid());
    }
    Ok(bytes)
}
fn commit(c: &Connection, r: &Reference) -> StorageResult<()> {
    let directory = path(c, r)?;
    let marker = directory.join(COMMIT);
    let bytes = serde_json::to_vec(r).map_err(|_| invalid())?;
    if marker.try_exists().map_err(|_| unavailable())? {
        if !io(fs::symlink_metadata(&marker))?.file_type().is_file() {
            return Err(invalid());
        }
        let mut stored = Vec::new();
        io(io(File::open(&marker))?.take(4097).read_to_end(&mut stored))?;
        if stored == bytes {
            return Ok(());
        }
    }
    read(c, r)?;
    let mut file = io(OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(marker))?;
    io(file.write_all(&bytes))?;
    io(file.sync_all())?;
    restore_backup::sync_directory(&directory)
}
fn remove(directory: &Path) -> StorageResult<()> {
    match fs::symlink_metadata(directory) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Ok(m) if m.file_type().is_dir() => {}
        _ => return Err(unavailable()),
    }
    let entries = io(fs::read_dir(directory))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| unavailable())?;
    for e in &entries {
        if (e.file_name() != restore_backup::ARCHIVE && e.file_name() != COMMIT)
            || !io(e.file_type())?.is_file()
        {
            return Err(invalid());
        }
    }
    for e in entries {
        io(fs::remove_file(e.path()))?;
    }
    io(fs::remove_dir(directory))?;
    restore_backup::sync_directory(directory.parent().ok_or_else(unavailable)?)
}
pub(super) fn cleanup(c: &Connection) -> StorageResult<()> {
    let g = global(c)?;
    let root = folder(c)?;
    if !g.folder_initialized && !root.try_exists().map_err(|_| unavailable())? {
        return Ok(());
    }
    directory(&root)?;
    let mut upper = PREFIX.as_bytes().to_vec();
    upper.push(255);
    let mut query =
        c.prepare("SELECT key FROM command_results WHERE key>=?1 AND key<?2 ORDER BY key")?;
    let keys = query
        .query_map((PREFIX.as_bytes(), upper), |r| r.get::<_, Vec<u8>>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut retained = BTreeSet::new();
    for key in keys {
        let space = std::str::from_utf8(&key[PREFIX.len()..]).map_err(|_| invalid())?;
        if !crate::columns::valid_id(space) {
            return Err(invalid());
        }
        let mut s = state(c, space)?;
        for r in &s.files {
            commit(c, r)?;
            retained.insert(r.id.clone());
        }
        if !s.obsolete.is_empty()
            && let Some(last) = s.files.last()
        {
            read(c, last)?;
        }
        for r in &s.obsolete {
            let path = root.join(&r.id);
            if path.try_exists().map_err(|_| unavailable())? {
                path_for_delete(c, r)?;
                remove(&path)?;
            }
        }
        if !s.obsolete.is_empty() {
            s.obsolete.clear();
            save(c, &state_key(space), &s)?;
        }
    }
    for entry in io(fs::read_dir(&root))? {
        let entry = io(entry)?;
        let name = entry.file_name().to_string_lossy().into_owned();
        if name
            .strip_prefix(&namespace(&g))
            .and_then(|s| s.split_once('-'))
            .is_some_and(|(space, tail)| {
                crate::columns::valid_id(space)
                    && !tail.is_empty()
                    && tail.len() <= 64
                    && tail.bytes().all(|b| b.is_ascii_alphanumeric())
            })
            && !retained.contains(&name)
            && io(entry.file_type())?.is_dir()
            && !entry
                .path()
                .join(COMMIT)
                .try_exists()
                .map_err(|_| unavailable())?
        {
            remove(&entry.path())?;
        }
    }
    Ok(())
}
fn path_for_delete(c: &Connection, r: &Reference) -> StorageResult<()> {
    path(c, r).map(|_| ())
}
