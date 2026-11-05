use std::{collections::BTreeMap, fs, path::PathBuf, process::Command};

use automerge::{ReadDoc, transaction::Transactable};
use sha2::{Digest, Sha256};
use tempfile::TempDir;

use super::*;
use crate::Storage;

const V1: &[u8] = include_bytes!("../../tests/fixtures/schema-v1.sqlite");
const SPACE: &str = "Историческое ../ 空間";

fn fixture() -> (TempDir, PathBuf) {
    let root = TempDir::new().unwrap();
    let path = root.path().join("device.sqlite");
    fs::write(&path, V1).unwrap();
    (root, path)
}

// SHM contains SQLite's transient reader locks, not authoritative DB/WAL data.
fn durable_files(path: &Path) -> BTreeMap<String, Vec<u8>> {
    ["", "-wal", "-journal"]
        .into_iter()
        .filter_map(|suffix| {
            let file = path.with_file_name(format!("device.sqlite{suffix}"));
            match fs::read(file) {
                Ok(bytes) => Some((suffix.to_owned(), bytes)),
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => panic!("{error}"),
            }
        })
        .collect()
}

#[tokio::test]
async fn historical_v1_opens_keeps_receipts_and_supports_reverse_after_restart() {
    assert_eq!(
        Sha256::digest(V1)
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
        "08683246876eb7e32592efe22be11737406fccfa15e787742b3505c9f1e6a044"
    );
    let (_root, path) = fixture();
    let storage = Storage::open(&path).await.unwrap();
    storage
        .write(b"edit".to_vec(), vec![], |ctx| {
            assert_eq!(ctx.read().document(SPACE)?.get_actor().to_bytes(), &[0x31]);
            let record = ctx.read().recorded(b"old-command")?.unwrap();
            assert_eq!(record.request, b"original request");
            assert_eq!(record.result, b"\"original result\"");
            ctx.edit(SPACE, |tx| {
                tx.put(automerge::ROOT, "title", "Новая правка")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(&path).await.unwrap();
    storage
        .write(b"reverse".to_vec(), vec![], |ctx| {
            assert_eq!(ctx.read().document(SPACE)?.get_actor().to_bytes(), &[0x31]);
            ctx.edit(SPACE, |tx| {
                tx.delete(automerge::ROOT, "title")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    storage
        .read(|ctx| {
            assert!(
                ctx.document(SPACE)?
                    .get(automerge::ROOT, "title")?
                    .is_none()
            );
            assert_eq!(ctx.document(SPACE)?.get_changes(&[]).len(), 2);
            assert!(ctx.recorded(b"old-command")?.is_some());
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn future_schema_rejected_before_writer_admission_in_delete_and_live_wal() {
    for mode in ["DELETE", "WAL"] {
        let (_root, path) = fixture();
        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "journal_mode", mode)
            .unwrap();
        connection.pragma_update(None, "user_version", 99).unwrap();
        connection
            .execute_batch(
                "BEGIN IMMEDIATE; INSERT INTO command_results VALUES (x'01', x'02', x'03')",
            )
            .unwrap();
        let before = durable_files(&path);
        assert_eq!(
            Storage::open(&path).await.err(),
            Some(StorageError::UnsupportedSchema)
        );
        assert_eq!(durable_files(&path), before);
        assert!(
            !connection.is_autocommit(),
            "other writer remains untouched"
        );
        connection.execute_batch("ROLLBACK").unwrap();
    }
}

#[tokio::test]
async fn foreign_keys_are_checked_before_enabling_wal_and_retry_can_succeed() {
    let (_root, path) = fixture();
    let connection = Connection::open(&path).unwrap();
    connection
        .pragma_update(None, "foreign_keys", false)
        .unwrap();
    connection
        .execute(
            "INSERT INTO changes VALUES ('missing', zeroblob(32), x'01')",
            [],
        )
        .unwrap();
    connection.close().unwrap();
    let before = durable_files(&path);
    assert_eq!(
        Storage::open(&path).await.err(),
        Some(StorageError::InvalidDatabase)
    );
    assert_eq!(durable_files(&path), before);
    let connection = Connection::open(&path).unwrap();
    connection
        .execute("DELETE FROM changes WHERE space = 'missing'", [])
        .unwrap();
    connection.close().unwrap();
    Storage::open(&path)
        .await
        .unwrap()
        .shutdown()
        .await
        .unwrap();
}

#[tokio::test]
async fn invalid_layout_and_false_empty_identity_do_not_enable_wal_or_reset_data() {
    for sql in [
        "PRAGMA application_id = 7",
        "PRAGMA application_id = 0; PRAGMA user_version = 0",
        "PRAGMA user_version = 0",
        "DROP TABLE peers",
        "CREATE INDEX unexpected ON documents(actor)",
    ] {
        let (_root, path) = fixture();
        let connection = Connection::open(&path).unwrap();
        connection.execute_batch(sql).unwrap();
        connection.close().unwrap();
        let before = durable_files(&path);
        assert!(Storage::open(&path).await.is_err());
        assert_eq!(durable_files(&path), before);
    }
}

#[tokio::test]
async fn damaged_sqlite_files_are_preserved_and_never_initialized() {
    for bytes in [b"not sqlite".as_slice(), &V1[..V1.len() - 4096]] {
        let (_root, path) = fixture();
        fs::write(&path, bytes).unwrap();
        let before = durable_files(&path);
        assert!(Storage::open(&path).await.is_err());
        assert_eq!(durable_files(&path), before);
    }
}

#[test]
fn admission_is_rechecked_after_preflight_before_any_persistent_pragma() {
    for sql in [
        "PRAGMA user_version = 99",
        "CREATE TABLE surprise(value BLOB)",
        "INSERT INTO changes VALUES ('missing', zeroblob(32), x'01')",
    ] {
        let (_root, path) = fixture();
        preflight(&path).unwrap();
        let mut connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection.execute_batch(sql).unwrap();
        let before = durable_files(&path);
        assert!(initialize(&mut connection).is_err());
        assert_eq!(durable_files(&path), before);
        assert_eq!(
            connection
                .query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "delete"
        );
    }
}

#[tokio::test]
async fn preflight_does_not_create_missing_or_initialize_empty_files() {
    let root = TempDir::new().unwrap();
    let path = root.path().join("device.sqlite");
    preflight(&path).unwrap();
    assert!(!path.exists());
    fs::write(&path, []).unwrap();
    preflight(&path).unwrap();
    assert!(fs::read(&path).unwrap().is_empty());
    Storage::open(&path)
        .await
        .unwrap()
        .shutdown()
        .await
        .unwrap();
    assert!(fs::metadata(path).unwrap().len() > 0);
}

#[test]
fn hot_journal_child() {
    let Some(path) = std::env::var_os("ZEDONE_SCHEMA_HOT_JOURNAL_CHILD") else {
        return;
    };
    let connection = Connection::open(PathBuf::from(path)).unwrap();
    // Force dirty pages to disk so a real hot journal is needed on reopen.
    connection
        .execute_batch(
            "PRAGMA cache_size = 1; PRAGMA synchronous = FULL; BEGIN IMMEDIATE;
        UPDATE command_results SET result = zeroblob(1048576)",
        )
        .unwrap();
    std::process::exit(71);
}

#[tokio::test]
async fn readonly_preflight_does_not_roll_back_a_future_database_hot_journal() {
    let (_root, path) = fixture();
    let connection = Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", 99).unwrap();
    connection.close().unwrap();
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "schema::tests::hot_journal_child", "--nocapture"])
        .env("ZEDONE_SCHEMA_HOT_JOURNAL_CHILD", &path)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(71));
    let before = durable_files(&path);
    assert!(
        before
            .get("-journal")
            .is_some_and(|bytes| !bytes.is_empty())
    );
    assert!(Storage::open(&path).await.is_err());
    assert_eq!(durable_files(&path), before);
}
