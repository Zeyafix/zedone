use std::{
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{StorageError, StorageResult};

const SUFFIXES: [&str; 4] = ["", "-wal", "-shm", "-journal"];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Entry {
    pub(crate) bytes: u64,
    pub(crate) sha256: [u8; 32],
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    kind: String,
    version: u32,
    files: [Option<Entry>; 4],
    #[serde(default, skip_serializing_if = "is_false")]
    missing_database: bool,
}
fn is_false(value: &bool) -> bool {
    !value
}

#[derive(Debug)]
pub struct ProblemState {
    path: PathBuf,
    manifest: Manifest,
}

impl ProblemState {
    pub fn open(path: PathBuf, max_total_bytes: u64) -> StorageResult<Self> {
        let mut bytes = Vec::new();
        regular(&path.join("manifest.json"))?
            .ok_or(StorageError::Unavailable)?
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(io)?;
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|_| StorageError::InvalidDatabase)?;
        if manifest.kind != "zedone.problem-state"
            || manifest.version != 1
            || manifest.files[0].is_none() != manifest.missing_database
        {
            return Err(StorageError::InvalidDatabase);
        }
        let total = total_bytes(&manifest)?;
        if total > max_total_bytes {
            return Err(StorageError::Unavailable);
        }
        let state = Self { path, manifest };
        state.verify()?;
        Ok(state)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn verify(&self) -> StorageResult<()> {
        if !fs::symlink_metadata(&self.path)
            .map_err(io)?
            .file_type()
            .is_dir()
        {
            return Err(StorageError::Unavailable);
        }
        let mut bytes = Vec::new();
        regular(&self.path.join("manifest.json"))?
            .ok_or(StorageError::Unavailable)?
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(io)?;
        let expected = serde_json::to_vec(&self.manifest).map_err(|_| StorageError::Unavailable)?;
        if bytes != expected {
            return Err(StorageError::InvalidDatabase);
        }
        let budget = total_bytes(&self.manifest)?;
        if inventory(&self.path.join("database.sqlite"), budget)? != self.manifest.files {
            return Err(StorageError::InvalidDatabase);
        }
        Ok(())
    }

    pub fn discard(self) -> StorageResult<()> {
        self.verify()?;
        let expected_count = self.manifest.files.iter().flatten().count() + 1;
        if fs::read_dir(&self.path)
            .map_err(io)?
            .take(expected_count + 1)
            .try_fold(0_usize, |count, entry| {
                entry.map_err(io)?;
                Ok::<_, StorageError>(count + 1)
            })?
            != expected_count
        {
            return Err(StorageError::Unavailable);
        }
        for (suffix, entry) in SUFFIXES.iter().zip(&self.manifest.files) {
            if entry.is_some() {
                fs::remove_file(suffixed(&self.path.join("database.sqlite"), suffix))
                    .map_err(io)?;
            }
        }
        fs::remove_file(self.path.join("manifest.json")).map_err(io)?;
        fs::remove_dir(&self.path).map_err(io)
    }
    pub(crate) fn deletion_record(&self) -> StorageResult<Vec<u8>> {
        self.verify()?;
        serde_json::to_vec(&self.manifest).map_err(|_| StorageError::Unavailable)
    }
}

pub(crate) fn discard_remaining(
    path: &Path,
    record: &[u8],
    mut check: impl FnMut() -> StorageResult<()>,
) -> StorageResult<()> {
    if !path.try_exists().map_err(io)? {
        return Ok(());
    }
    if !fs::symlink_metadata(path).map_err(io)?.file_type().is_dir() {
        return Err(StorageError::InvalidDatabase);
    }
    let manifest: Manifest =
        serde_json::from_slice(record).map_err(|_| StorageError::InvalidDatabase)?;
    if manifest.kind != "zedone.problem-state"
        || manifest.version != 1
        || manifest.files[0].is_none() != manifest.missing_database
    {
        return Err(StorageError::InvalidDatabase);
    }
    let mut names = vec!["manifest.json".to_owned()];
    for (i, suffix) in SUFFIXES.iter().enumerate() {
        if manifest.files[i].is_some() {
            names.push(format!("database.sqlite{suffix}"));
        }
    }
    for entry in fs::read_dir(path).map_err(io)?.take(6) {
        let entry = entry.map_err(io)?;
        if !entry.file_type().map_err(io)?.is_file()
            || !names.iter().any(|n| entry.file_name() == n.as_str())
        {
            return Err(StorageError::InvalidDatabase);
        }
    }
    let present = inventory(&path.join("database.sqlite"), total_bytes(&manifest)?)?;
    for (actual, expected) in present.iter().zip(&manifest.files) {
        if actual.is_some() && actual != expected {
            return Err(StorageError::InvalidDatabase);
        }
    }
    let marker = path.join("manifest.json");
    if marker.exists() {
        if crate::backup_io::read(&marker, 4096)? != record {
            return Err(StorageError::InvalidDatabase);
        }
    } else if present.iter().any(Option::is_some) {
        return Err(StorageError::InvalidDatabase);
    }
    for (i, suffix) in SUFFIXES.iter().enumerate() {
        if present[i].is_some() {
            check()?;
            fs::remove_file(suffixed(&path.join("database.sqlite"), suffix)).map_err(io)?;
            #[cfg(test)]
            if std::env::var("ZEDONE_EMERGENCY_CRASH").as_deref() == Ok("problem_delete") {
                std::process::exit(74);
            }
        }
    }
    if marker.exists() {
        check()?;
        fs::remove_file(marker).map_err(io)?;
    }
    fs::remove_dir(path).map_err(io)
}

fn total_bytes(manifest: &Manifest) -> StorageResult<u64> {
    manifest
        .files
        .iter()
        .flatten()
        .try_fold(0_u64, |total, entry| {
            total
                .checked_add(entry.bytes)
                .ok_or(StorageError::Unavailable)
        })
}

fn sync_directory(path: &Path) -> StorageResult<()> {
    #[cfg(unix)]
    File::open(path).map_err(io)?.sync_all().map_err(io)?;
    #[cfg(not(unix))]
    let _ = path;
    Ok(())
}

fn io(_: std::io::Error) -> StorageError {
    StorageError::Unavailable
}

fn suffixed(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

fn regular(path: &Path) -> StorageResult<Option<File>> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(io(error)),
        Ok(metadata) if metadata.file_type().is_file() => {
            let file = File::open(path).map_err(io)?;
            if !file.metadata().map_err(io)?.is_file() {
                return Err(StorageError::Unavailable);
            }
            Ok(Some(file))
        }
        Ok(_) => Err(StorageError::Unavailable),
    }
}

fn digest(file: &mut File, budget: u64, mut copy: Option<&mut File>) -> StorageResult<Entry> {
    let mut hash = Sha256::new();
    let mut bytes = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).map_err(io)?;
        if count == 0 {
            break;
        }
        bytes = bytes
            .checked_add(count as u64)
            .ok_or(StorageError::Unavailable)?;
        if bytes > budget {
            return Err(StorageError::Unavailable);
        }
        hash.update(&buffer[..count]);
        if let Some(destination) = copy.as_deref_mut() {
            destination.write_all(&buffer[..count]).map_err(io)?;
        }
    }
    Ok(Entry {
        bytes,
        sha256: hash.finalize().into(),
    })
}

pub(crate) fn inventory(path: &Path, mut budget: u64) -> StorageResult<[Option<Entry>; 4]> {
    let mut entries = [const { None }; 4];
    for (index, suffix) in SUFFIXES.iter().enumerate() {
        if let Some(mut file) = regular(&suffixed(path, suffix))? {
            let entry = digest(&mut file, budget, None)?;
            budget -= entry.bytes;
            entries[index] = Some(entry);
        }
    }
    Ok(entries)
}

pub(crate) fn preserve(
    path: &Path,
    folder: &Path,
    max_total_bytes: u64,
) -> StorageResult<ProblemState> {
    preserve_with(path, folder, max_total_bytes, || Ok(()))
}

fn preserve_with(
    path: &Path,
    folder: &Path,
    max_total_bytes: u64,
    checkpoint: impl FnOnce() -> StorageResult<()>,
) -> StorageResult<ProblemState> {
    capture(path, folder, max_total_bytes, false, checkpoint)
}
pub(crate) fn preserve_absent(
    path: &Path,
    folder: &Path,
    max_total_bytes: u64,
) -> StorageResult<ProblemState> {
    capture(path, folder, max_total_bytes, true, || Ok(()))
}
fn capture(
    path: &Path,
    folder: &Path,
    max_total_bytes: u64,
    missing_database: bool,
    checkpoint: impl FnOnce() -> StorageResult<()>,
) -> StorageResult<ProblemState> {
    let directory = tempfile::Builder::new()
        .prefix("problem-state-")
        .tempdir_in(folder)
        .map_err(io)?;
    let mut files = [const { None }; 4];
    let mut remaining = max_total_bytes;
    for (index, suffix) in SUFFIXES.iter().enumerate() {
        if let Some(mut source) = regular(&suffixed(path, suffix))? {
            let destination = suffixed(&directory.path().join("database.sqlite"), suffix);
            let mut destination = OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(destination)
                .map_err(io)?;
            let entry = digest(&mut source, remaining, Some(&mut destination))?;
            destination.sync_all().map_err(io)?;
            remaining -= entry.bytes;
            files[index] = Some(entry);
        }
    }
    if files[0].is_none() != missing_database {
        return Err(StorageError::Unavailable);
    }
    checkpoint()?;
    if inventory(path, max_total_bytes)? != files {
        return Err(StorageError::Unavailable);
    }
    let manifest = Manifest {
        kind: "zedone.problem-state".into(),
        version: 1,
        files,
        missing_database,
    };
    let bytes = serde_json::to_vec(&manifest).map_err(|_| StorageError::Unavailable)?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(directory.path().join("manifest.json"))
        .map_err(io)?;
    output.write_all(&bytes).map_err(io)?;
    output.sync_all().map_err(io)?;
    drop(output);
    let state = ProblemState {
        path: directory.path().to_owned(),
        manifest,
    };
    state.verify()?;
    sync_directory(directory.path())?;
    sync_directory(folder)?;
    let _ = directory.keep();
    Ok(state)
}

#[cfg(test)]
mod tests;
