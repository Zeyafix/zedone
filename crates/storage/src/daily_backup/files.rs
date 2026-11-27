use super::*;
use crate::backup_io as io;
use crate::{restore_backup, snapshot::SpaceSnapshot};
use std::path::Path;
const COMMIT: &str = "commit.json";
pub(super) fn folder(c: &Connection) -> StorageResult<PathBuf> {
    crate::backup_catalog::folder(c)
}
fn directory(path: &Path) -> StorageResult<()> {
    io::directory(path)
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
    if !g.folder_initialized && !io::exists(&root)? {
        io::mkdir(&root)?;
    }
    directory(&root)?;
    g.folder_initialized = true;
    crate::backup_catalog::writable(c)?;
    let snapshot = crate::snapshot::export::capture_export(c, space.into(), date, LIMITS)?;
    let bytes = snapshot.encode(LIMITS).map_err(|_| invalid())?;
    let id = format!("{}{}", prefix(g, space), &io::random_id()?[..16]);
    let path = root.join(&id);
    io::mkdir(&path)?;
    io::write(&path.join(restore_backup::ARCHIVE), &bytes, true)?;
    let reference = Reference {
        id,
        space: space.into(),
        created_at_unix_ms: date,
        bytes: bytes.len(),
        digest: Sha256::digest(&bytes).into(),
    };
    read(c, &reference)?;
    Ok(reference)
}

pub(super) fn read(c: &Connection, r: &Reference) -> StorageResult<Vec<u8>> {
    let file = path(c, r)?.join(restore_backup::ARCHIVE);
    if r.bytes > MAX_ARCHIVE_BYTES {
        return Err(invalid());
    }
    let bytes = io::read(&file, r.bytes)?;
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
    if io::exists(&marker)? && io::read(&marker, 4096)? == bytes {
        return Ok(());
    }
    read(c, r)?;
    io::write(&marker, &bytes, false)
}

fn remove(directory: &Path) -> StorageResult<()> {
    if !io::exists(directory)? {
        return Ok(());
    }
    let entries = io::list(directory)?;
    if entries
        .iter()
        .any(|e| e.directory || !matches!(e.name.as_str(), "space.bin" | "commit.json"))
    {
        return Err(invalid());
    }
    for entry in entries {
        io::remove_file(&directory.join(entry.name))?;
    }
    io::remove_dir(directory)
}
pub(super) fn cleanup(c: &Connection) -> StorageResult<()> {
    crate::backup_catalog::writable(c)?;
    let g = global(c)?;
    let root = folder(c)?;
    if !g.folder_initialized && !io::exists(&root)? {
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
            if io::exists(&path)? {
                path_for_delete(c, r)?;
                remove(&path)?;
            }
        }
        if !s.obsolete.is_empty() {
            s.obsolete.clear();
            save(c, &state_key(space), &s)?;
        }
    }
    for entry in io::list(&root)? {
        let name = entry.name;
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
            && entry.directory
            && !io::exists(&root.join(&name).join(COMMIT))?
        {
            remove(&root.join(name))?;
        }
    }
    Ok(())
}
fn path_for_delete(c: &Connection, r: &Reference) -> StorageResult<()> {
    path(c, r).map(|_| ())
}
