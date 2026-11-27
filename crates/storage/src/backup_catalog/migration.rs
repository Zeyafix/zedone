use super::*;
pub(super) const INDEX: &str = "migration-current.json";
pub(super) fn name(id: &str) -> bool {
    id.strip_prefix("zedone-migration-candidate-")
        .is_some_and(|tail| {
            !tail.is_empty() && tail.len() <= 64 && tail.bytes().all(|b| b.is_ascii_alphanumeric())
        })
}
pub(super) fn snapshot_name(name: &str) -> bool {
    name.strip_prefix("space-")
        .and_then(|s| s.strip_suffix(".bin"))
        .is_some_and(|n| !n.is_empty() && n.len() <= 8 && n.bytes().all(|b| b.is_ascii_digit()))
}
pub(super) fn measure(path: &Path) -> StorageResult<FileRecord> {
    let mut digest = Sha256::new();
    let mut size = 0usize;
    loop {
        let bytes = io::chunk(path, size as u64)?;
        if bytes.is_empty() {
            break;
        }
        size = size.checked_add(bytes.len()).ok_or_else(unavailable)?;
        digest.update(bytes);
    }
    Ok(FileRecord {
        name: path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(unavailable)?
            .into(),
        size,
        digest: digest.finalize().into(),
    })
}
pub(super) fn records(path: &Path) -> StorageResult<Vec<FileRecord>> {
    if path.file_name().is_some_and(|n| n == INDEX) {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Digest {
            bytes: usize,
            sha256: [u8; 32],
        }
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Index {
            name: String,
            manifest: Digest,
        }
        let index: Index = serde_json::from_slice(&io::read(path, 4096)?)
            .map_err(|_| StorageError::InvalidDatabase)?;
        if !name(&index.name) {
            return Err(StorageError::InvalidDatabase);
        }
        let package = path
            .parent()
            .ok_or(StorageError::InvalidDatabase)?
            .join(index.name);
        io::directory(&package)?;
        let manifest = io::read(&package.join("manifest.json"), MAX_ARCHIVE_BYTES)?;
        if manifest.len() != index.manifest.bytes
            || Sha256::digest(&manifest).as_slice() != index.manifest.sha256
        {
            return Err(StorageError::InvalidDatabase);
        }
        return Ok(vec![measure(path)?]);
    }
    let manifest: serde_json::Value =
        serde_json::from_slice(&io::read(&path.join("manifest.json"), MAX_ARCHIVE_BYTES)?)
            .map_err(|_| StorageError::InvalidDatabase)?;
    let spaces = manifest["spaces"]
        .as_array()
        .ok_or(StorageError::InvalidDatabase)?;
    if manifest["version"] != 1 || spaces.len() > 10000 {
        return Err(StorageError::InvalidDatabase);
    }
    let entries = io::list(path)?;
    if entries.len() != spaces.len() + 2 {
        return Err(StorageError::InvalidDatabase);
    }
    let mut records = Vec::new();
    for entry in entries {
        if entry.directory
            || !(entry.name == "database.sqlite"
                || entry.name == "manifest.json"
                || snapshot_name(&entry.name))
        {
            return Err(StorageError::InvalidDatabase);
        }
        let record = measure(&path.join(&entry.name))?;
        if entry.name != "manifest.json" {
            let expected = if entry.name == "database.sqlite" {
                &manifest["database"]
            } else {
                let i = entry.name[6..entry.name.len() - 4]
                    .parse::<usize>()
                    .map_err(|_| StorageError::InvalidDatabase)?;
                spaces.get(i).ok_or(StorageError::InvalidDatabase)?
            };
            if expected["bytes"] != record.size
                || expected["sha256"] != serde_json::json!(record.digest)
            {
                return Err(StorageError::InvalidDatabase);
            }
        }
        records.push(record);
    }
    records.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(records)
}
pub(super) fn copies(root: &Path, id: &str) -> StorageResult<(Vec<CatalogCopy>, CopyRecord)> {
    let path = root.join(id);
    let files = records(&path)?;
    let mut copies = Vec::new();
    let mut used = 0usize;
    for file in &files {
        if !snapshot_name(&file.name) {
            continue;
        }
        let bytes = io::read(&path.join(&file.name), MAX_ARCHIVE_BYTES)?;
        let snapshot =
            SpaceSnapshot::decode(&bytes, LIMITS).map_err(|_| StorageError::InvalidDatabase)?;
        if snapshot.metadata().created_at_unix_ms
            > zedone_core::access_commands::MAX_EXPIRY_UNIX_MS as u64
        {
            return Err(StorageError::InvalidDatabase);
        }
        let mut graph = Automerge::new();
        graph.apply_changes(snapshot.changes().iter().cloned())?;
        let names = graph
            .get_all(ROOT, "name")?
            .into_iter()
            .filter_map(|(v, _)| v.as_str().map(str::to_owned))
            .collect();
        let copy = CatalogCopy {
            id: format!("{id}@{}", file.name),
            space: snapshot.metadata().space.clone(),
            names,
            kind: "migration".into(),
            created_at_unix_ms: snapshot.metadata().created_at_unix_ms,
            digest: StateVersion(Sha256::digest(&bytes).into()),
        };
        used = used
            .checked_add(serde_json::to_vec(&copy).map_err(|_| unavailable())?.len())
            .ok_or_else(unavailable)?;
        if used > MAX_ARCHIVE_BYTES {
            return Err(unavailable());
        }
        copies.push(copy);
    }
    Ok((
        copies,
        CopyRecord {
            id: id.into(),
            files,
        },
    ))
}
pub(super) fn file_path(root: &Path, id: &str, file: &str) -> PathBuf {
    if id == INDEX {
        root.join(file)
    } else {
        root.join(id).join(file)
    }
}
pub(super) fn copy_file(source: &Path, target: &Path, expected: &FileRecord) -> StorageResult<()> {
    let source_record = measure(source)?;
    if source_record.size != expected.size || source_record.digest != expected.digest {
        return Err(StorageError::InvalidDatabase);
    }
    if !io::exists(target)? {
        io::write(target, &[], true)?;
    }
    let mut offset = 0u64;
    loop {
        let existing = io::chunk(target, offset)?;
        if existing.is_empty() {
            break;
        }
        let original = io::chunk(source, offset)?;
        if !original.starts_with(&existing) {
            return Err(StorageError::InvalidDatabase);
        }
        offset += existing.len() as u64;
    }
    loop {
        let bytes = io::chunk(source, offset)?;
        if bytes.is_empty() {
            break;
        }
        io::append(target, offset, &bytes)?;
        offset += bytes.len() as u64;
    }
    let actual = measure(target)?;
    if actual.size != expected.size || actual.digest != expected.digest {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}
pub(super) fn transfer_record(
    source: &Path,
    target: &Path,
    record: &CopyRecord,
) -> StorageResult<()> {
    let destination = target.join(&record.id);
    if io::exists(&destination)? && check_record(target, record).is_ok() {
        return Ok(());
    }
    let marker = if record.id == INDEX {
        target.join("zedone-transfer-migration-current.json")
    } else {
        destination.join("transfer.json")
    };
    let bytes = serde_json::to_vec(record).map_err(|_| unavailable())?;
    if record.id != INDEX && !io::exists(&destination)? {
        io::mkdir(&destination)?;
    }
    if io::exists(&marker)? {
        if io::read(&marker, MAX_ARCHIVE_BYTES)? != bytes {
            return Err(StorageError::InvalidDatabase);
        }
    } else {
        if record.id == INDEX {
            if io::exists(&destination)? {
                return Err(StorageError::InvalidDatabase);
            }
        } else if !io::list(&destination)?.is_empty() {
            return Err(StorageError::InvalidDatabase);
        }
        io::write(&marker, &bytes, true)?;
    }
    if record.id != INDEX
        && io::list(&destination)?.iter().any(|e| {
            e.directory
                || (e.name != "transfer.json" && !record.files.iter().any(|f| f.name == e.name))
        })
    {
        return Err(StorageError::InvalidDatabase);
    }
    for file in &record.files {
        copy_file(
            &file_path(source, &record.id, &file.name),
            &file_path(target, &record.id, &file.name),
            file,
        )?;
    }
    for file in &record.files {
        let actual = measure(&file_path(target, &record.id, &file.name))?;
        if actual.size != file.size || actual.digest != file.digest {
            return Err(StorageError::InvalidDatabase);
        }
    }
    io::remove_file(&marker)?;
    check_record(target, record)
}
