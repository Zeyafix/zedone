use super::*;

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("problem 空間.sqlite");
    fs::write(&source, b"invalid SQLite\0\xff").unwrap();
    let folder = root.path().join("copies");
    fs::create_dir(&folder).unwrap();
    (root, source, folder)
}

#[test]
fn corrupt_bytes_and_all_sidecars_survive_drop_reopen_and_explicit_discard() {
    let (_root, source, folder) = fixture();
    for suffix in &SUFFIXES[1..] {
        fs::write(suffixed(&source, suffix), suffix.as_bytes()).unwrap();
    }
    let before = inventory(&source, 1024).unwrap();
    let state = preserve(&source, &folder, 1024).unwrap();
    let path = state.path().to_owned();
    assert_eq!(
        inventory(&path.join("database.sqlite"), 1024).unwrap(),
        before
    );
    assert_eq!(inventory(&source, 1024).unwrap(), before);
    let manifest = fs::read_to_string(path.join("manifest.json")).unwrap();
    assert!(manifest.contains("zedone.problem-state"));
    assert!(!manifest.contains("空間"));
    drop(state);
    let state = ProblemState::open(path.clone(), 1024).unwrap();
    state.discard().unwrap();
    assert!(!path.exists());
    assert_eq!(inventory(&source, 1024).unwrap(), before);
}

#[test]
fn exact_total_budget_includes_every_sidecar_and_failure_keeps_previous_copies() {
    let (_root, source, folder) = fixture();
    fs::write(suffixed(&source, "-wal"), b"wal bytes").unwrap();
    let entries = inventory(&source, 1024).unwrap();
    let total: u64 = entries.iter().flatten().map(|e| e.bytes).sum();
    let previous = preserve(&source, &folder, total).unwrap();
    assert!(preserve(&source, &folder, total - 1).is_err());
    assert!(ProblemState::open(previous.path().to_owned(), total - 1).is_err());
    previous.verify().unwrap();
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    assert_eq!(inventory(&source, 1024).unwrap(), entries);
}

#[test]
fn source_mutation_addition_and_removal_refuse_publication() {
    for change in 0..3 {
        let (_root, source, folder) = fixture();
        fs::write(suffixed(&source, "-wal"), b"original").unwrap();
        let result = preserve_with(&source, &folder, 1024, || {
            match change {
                0 => fs::write(&source, b"modified"),
                1 => fs::write(suffixed(&source, "-journal"), b"new journal"),
                _ => fs::remove_file(suffixed(&source, "-wal")),
            }
            .unwrap();
            Ok(())
        });
        assert!(result.is_err());
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 0);
    }
}

#[test]
fn missing_source_bad_file_kind_and_unavailable_folder_never_create_a_database() {
    let (_root, source, folder) = fixture();
    let before = fs::read(&source).unwrap();
    assert!(preserve(&source, &folder.join("missing"), 1024).is_err());
    fs::create_dir(suffixed(&source, "-shm")).unwrap();
    assert!(preserve(&source, &folder, 1024).is_err());
    assert_eq!(fs::read(&source).unwrap(), before);
    fs::remove_file(&source).unwrap();
    assert!(preserve(&source, &folder, 1024).is_err());
    assert!(!source.exists());
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 0);
}

#[test]
fn changed_capture_or_unknown_content_cannot_be_discarded() {
    for change in 0..4 {
        let (_root, source, folder) = fixture();
        let state = preserve(&source, &folder, 1024).unwrap();
        let path = state.path().to_owned();
        match change {
            0 => fs::write(path.join("database.sqlite"), b"tampered").unwrap(),
            1 => fs::write(path.join("manifest.json"), b"{}").unwrap(),
            2 => fs::write(path.join("unknown-file"), b"preserve me").unwrap(),
            _ => fs::write(path.join("database.sqlite-wal"), b"unexpected").unwrap(),
        }
        assert!(state.discard().is_err());
        assert!(path.join("database.sqlite").exists());
        assert!(path.join("manifest.json").exists());
    }
}

#[test]
fn multiple_captures_are_independent_and_never_replace_each_other() {
    let (_root, source, folder) = fixture();
    let first = preserve(&source, &folder, 1024).unwrap();
    fs::write(&source, b"new failure state").unwrap();
    let second = preserve(&source, &folder, 1024).unwrap();
    assert_ne!(first.path(), second.path());
    first.verify().unwrap();
    second.discard().unwrap();
    first.verify().unwrap();
}

#[test]
fn sqlite_crash_child() {
    let Some(path) = std::env::var_os("ZEDONE_PROBLEM_CRASH_PATH") else {
        return;
    };
    let connection = rusqlite::Connection::open(PathBuf::from(path)).unwrap();
    connection
        .execute_batch(
            "PRAGMA cache_size=1; BEGIN IMMEDIATE; INSERT INTO probe VALUES (zeroblob(131072));",
        )
        .unwrap();
    std::process::exit(73);
}

#[tokio::test]
async fn real_crash_wal_and_hot_journal_are_copied_without_sqlite_recovery() {
    for mode in ["WAL", "DELETE"] {
        let (_root, source, folder) = fixture();
        fs::remove_file(&source).unwrap();
        let connection = rusqlite::Connection::open(&source).unwrap();
        connection
            .pragma_update(None, "journal_mode", mode)
            .unwrap();
        connection
            .execute_batch("CREATE TABLE probe (payload BLOB);")
            .unwrap();
        drop(connection);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "problem_state::tests::sqlite_crash_child",
                "--nocapture",
            ])
            .env("ZEDONE_PROBLEM_CRASH_PATH", &source)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        let before = inventory(&source, 1024 * 1024).unwrap();
        assert!(
            before[if mode == "WAL" { 1 } else { 3 }]
                .as_ref()
                .unwrap()
                .bytes
                > 0
        );
        let mut startup = crate::startup::Startup::new(&source);
        assert!(startup.automatic(None).await.is_err());
        let after_failure = inventory(&source, 1024 * 1024).unwrap();
        for index in [0, 1, 3] {
            assert_eq!(after_failure[index], before[index]);
        }
        let state = startup
            .preserve_problem_state(folder, 1024 * 1024)
            .await
            .unwrap();
        assert_eq!(inventory(&source, 1024 * 1024).unwrap(), after_failure);
        assert_eq!(
            inventory(&state.path().join("database.sqlite"), 1024 * 1024).unwrap(),
            after_failure
        );
        state.discard().unwrap();
        assert_eq!(inventory(&source, 1024 * 1024).unwrap(), after_failure);
    }
}

#[cfg(unix)]
#[test]
fn symbolic_source_and_capture_files_are_rejected() {
    let (root, source, folder) = fixture();
    let link = root.path().join("link.sqlite");
    std::os::unix::fs::symlink(&source, &link).unwrap();
    assert!(preserve(&link, &folder, 1024).is_err());
    let state = preserve(&source, &folder, 1024).unwrap();
    let copy = state.path().join("database.sqlite");
    fs::remove_file(&copy).unwrap();
    std::os::unix::fs::symlink(&source, &copy).unwrap();
    assert!(state.discard().is_err());
    assert!(source.exists());
}
