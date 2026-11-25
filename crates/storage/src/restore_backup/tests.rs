use super::*;
use crate::{access::PendingAccess, cards, migration_backup::tests::files};
use automerge::{Automerge, ROOT, ReadDoc, sync::State, transaction::Transactable};
use rusqlite::types::Value;
use std::{process::Command, time::Duration};
use zedone_core::{
    card_history::AutosaveCard,
    cards::{CardStore, CreateCard},
    columns::ColumnStore,
    spaces::{SpaceState, SpaceStore},
    tags::{CreateTag, SetCardTag, TagStore},
};

const LIMITS: SnapshotLimits = SnapshotLimits {
    max_bytes: 1024 * 1024,
    max_changes: 1000,
};

fn id(value: u8) -> String {
    format!("{value:032x}")
}

pub(crate) async fn fixture() -> (TempDir, Storage, PathBuf) {
    let root = TempDir::new().unwrap();
    let folder = root.path().join("копии ё🙂");
    fs::create_dir(&folder).unwrap();
    fs::write(folder.join("previous.bin"), b"previous successful backup").unwrap();
    let storage = Storage::open(root.path().join("source.db")).await.unwrap();
    storage
        .write(
            b"private-key".to_vec(),
            b"private-request".to_vec(),
            |ctx| {
                let admin = PendingAccess::generate()?;
                ctx.bootstrap_access(&admin)?;
                ctx.create_space(&id(1), "Снимок ё🙂", admin.subject())?;
                ctx.create_space(&id(2), "OTHER-SPACE-SECRET", admin.subject())?;
                ctx.create_column(&id(1), &id(3), "Колонка")?;
                for card in [4, 5] {
                    ctx.create_card(
                        &CreateCard {
                            space: id(1),
                            column: id(3),
                            card: id(card),
                            title: "Исходный заголовок".into(),
                            description: "<script>буквальный HTML</script>".into(),
                        },
                        admin.subject(),
                        &id(card),
                    )?;
                }
                ctx.set_card_lifecycle(
                    &id(1),
                    &id(5),
                    &format!("trash:{}:{}", admin.subject(), id(6)),
                )?;
                let tag = ctx.create_tag(&CreateTag {
                    space: id(1),
                    tag: id(7),
                    name: "Работа".into(),
                })?;
                let card = ctx.card_snapshot(&id(1), &id(4))?;
                ctx.set_card_tag(&SetCardTag {
                    space: id(1),
                    card: id(4),
                    tag: id(7),
                    present: true,
                    expected: card.version,
                    expected_tag: tag.version,
                })?;
                ctx.start_text_session(&id(1), &id(4), admin.subject(), &id(8))?;
                ctx.save_text_series(
                    &AutosaveCard {
                        space: id(1),
                        card: id(4),
                        session: id(8),
                        title: "Последняя пара ё🙂".into(),
                        description: "Незавершённая история".into(),
                        expected: card.version,
                    },
                    admin.subject(),
                    &id(9),
                    100,
                )?;
                let base = ctx.read().document(&id(1))?.clone();
                let mut changes = Vec::new();
                for name in ["ПК", "Телефон"] {
                    let mut fork = base.fork();
                    let mut tx = fork.transaction();
                    tx.put(ROOT, "name", name)?;
                    tx.commit();
                    changes.extend(fork.get_changes(&base.get_heads()));
                }
                let mut remote = base.fork();
                let mut tx = remote.transaction();
                tx.put(ROOT, "name", "пропущенная зависимость")?;
                tx.commit();
                let mut tx = remote.transaction();
                tx.put(ROOT, "name", "ожидает зависимость")?;
                let (hash, _) = tx.commit();
                changes.push(remote.get_change_by_hash(&hash.unwrap()).unwrap());
                ctx.apply_changes(&id(1), changes)?;
                ctx.set_peer_state(&id(1), b"private-peer", &State::new())?;
                let pause = ctx.read().space_sync_control(&id(1))?;
                ctx.set_space_sync_paused(&id(1), true, pause.version)?;
                let selected = ctx.read().sync_selection(&id(1), &id(10))?;
                ctx.set_sync_selected(&id(1), &id(10), true, selected.version)?;
                Ok("PRIVATE-RECEIPT-RESULT")
            },
        )
        .await
        .unwrap();
    (root, storage, folder)
}

async fn source(storage: &Storage) -> Vec<Vec<Vec<Value>>> {
    storage
        .read(|ctx| {
            let tables = ctx
                .connection()
                .prepare("SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name")?
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            tables
                .into_iter()
                .map(|table| {
                    let mut statement = ctx
                        .connection()
                        .prepare(&format!("SELECT * FROM {table} ORDER BY 1"))?;
                    let count = statement.column_count();
                    Ok(statement
                        .query_map([], |row| (0..count).map(|i| row.get(i)).collect())?
                        .collect::<Result<Vec<Vec<Value>>, _>>()?)
                })
                .collect()
        })
        .await
        .unwrap()
}

fn load(candidate: &PreparedRestoreBackup) -> (SpaceSnapshot, Automerge) {
    let snapshot =
        SpaceSnapshot::decode(&fs::read(candidate.path().join(ARCHIVE)).unwrap(), LIMITS).unwrap();
    let mut graph = Automerge::new();
    graph
        .apply_changes(snapshot.changes().iter().cloned())
        .unwrap();
    (snapshot, graph)
}

#[tokio::test]
async fn complete_product_backup_keeps_pending_history_and_every_source_row_unchanged() {
    let (root, storage, folder) = fixture().await;
    let before = source(&storage).await;
    let physical =
        ["source.db", "source.db-wal"].map(|file| fs::read(root.path().join(file)).unwrap());
    let candidate = storage
        .prepare_restore_backup(&folder, id(1), 42, LIMITS)
        .await
        .unwrap();
    assert_eq!(candidate.path().parent(), Some(folder.as_path()));
    assert_eq!(
        candidate.metadata(),
        &SnapshotMetadata {
            space: id(1),
            created_at_unix_ms: 42
        }
    );
    candidate.verify(LIMITS).unwrap();
    let (snapshot, graph) = load(&candidate);
    assert_eq!(graph.get_missing_deps(&[]).len(), 1);
    assert_eq!(graph.get_all(ROOT, "name").unwrap().len(), 2);
    let product = crate::spaces::snapshot(&graph, &id(1), true).unwrap();
    assert_eq!(product.names, ["ПК", "Телефон"]);
    let contents = product.contents.unwrap();
    assert_eq!(contents.columns.len(), 1);
    assert_eq!(contents.tags.len(), 1);
    assert_eq!(
        contents
            .cards
            .iter()
            .find(|card| card.card == id(5))
            .unwrap()
            .state,
        SpaceState::Trashed
    );
    assert_eq!(
        contents
            .cards
            .iter()
            .find(|card| card.card == id(4))
            .unwrap()
            .tags,
        [id(7)]
    );
    let versions = cards::history(&graph, &cards::object(&graph, &id(4)).unwrap()).unwrap();
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[1].title, "Последняя пара ё🙂");
    assert_eq!(versions[1].description, "Незавершённая история");
    let raw = storage.snapshot(id(1), 42, LIMITS).await.unwrap();
    assert_eq!(snapshot.changes().len(), raw.changes().len() + 1);
    for original in raw.changes() {
        assert!(
            snapshot
                .changes()
                .iter()
                .any(|change| change.raw_bytes() == original.raw_bytes())
        );
    }
    let bytes = fs::read(candidate.path().join(ARCHIVE)).unwrap();
    for secret in [
        "OTHER-SPACE-SECRET",
        "private-peer",
        "private-key",
        "private-request",
        "PRIVATE-RECEIPT-RESULT",
    ] {
        assert!(
            !bytes
                .windows(secret.len())
                .any(|part| part == secret.as_bytes())
        );
    }
    assert_eq!(source(&storage).await, before);
    assert_eq!(
        ["source.db", "source.db-wal"].map(|file| fs::read(root.path().join(file)).unwrap()),
        physical
    );
    assert_eq!(
        fs::read(folder.join("previous.bin")).unwrap(),
        b"previous successful backup"
    );
    candidate.discard().unwrap();
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn independent_candidates_survive_drop_restart_and_later_changes_until_explicit_discard() {
    let (root, storage, folder) = fixture().await;
    let first = storage
        .prepare_restore_backup(&folder, id(1), 42, LIMITS)
        .await
        .unwrap();
    let old = files(first.path());
    for name in ["Изменено", "ПК"] {
        storage
            .transact(move |ctx| Ok(ctx.rename_space(&id(1), name)?))
            .await
            .unwrap();
    }
    let second = storage
        .prepare_restore_backup(&folder, id(1), 43, LIMITS)
        .await
        .unwrap();
    assert_ne!(first.path(), second.path());
    assert_eq!(files(first.path()), old);
    let second_path = second.path().to_path_buf();
    drop(second);
    storage.shutdown().await.unwrap();
    let storage = Storage::open(root.path().join("source.db")).await.unwrap();
    first.verify(LIMITS).unwrap();
    assert!(second_path.join(ARCHIVE).is_file());
    first.discard().unwrap();
    assert!(second_path.join(ARCHIVE).is_file());
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 2);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn every_capture_failure_preserves_old_files_and_source_and_removes_only_its_candidate() {
    let (_root, storage, folder) = fixture().await;
    let before = source(&storage).await;
    let old = files(&folder);
    for point in [
        Checkpoint::BeforeWrite,
        Checkpoint::AfterWrite,
        Checkpoint::BeforeVerify,
        Checkpoint::AfterVerify,
    ] {
        let target = folder.clone();
        assert_eq!(
            storage
                .run(
                    move |connection| prepare(connection, &target, id(1), 42, LIMITS, |at, _| {
                        if at == point {
                            Err(StorageError::Unavailable)
                        } else {
                            Ok(())
                        }
                    })
                )
                .await
                .unwrap_err(),
            StorageError::Unavailable
        );
        assert_eq!(files(&folder), old);
        assert_eq!(source(&storage).await, before);
    }
    let target = folder.clone();
    assert!(
        storage
            .run(
                move |connection| prepare(connection, &target, id(1), 42, LIMITS, |at, path| {
                    if at == Checkpoint::BeforeVerify {
                        io(fs::write(path.join(ARCHIVE), b"damaged"))?;
                    }
                    Ok(())
                })
            )
            .await
            .is_err()
    );
    assert_eq!(files(&folder), old);
    assert_eq!(source(&storage).await, before);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn unavailable_folders_missing_space_and_budgets_never_fall_back_or_publish_partial_files() {
    let (_root, storage, folder) = fixture().await;
    let before = source(&storage).await;
    let old = files(&folder);
    for target in [folder.join("missing"), folder.join("previous.bin")] {
        assert!(
            storage
                .prepare_restore_backup(&target, id(1), 42, LIMITS)
                .await
                .is_err()
        );
    }
    for limits in [
        SnapshotLimits {
            max_bytes: 1,
            ..LIMITS
        },
        SnapshotLimits {
            max_changes: 1,
            ..LIMITS
        },
    ] {
        assert!(
            storage
                .prepare_restore_backup(&folder, id(1), 42, limits)
                .await
                .is_err()
        );
    }
    assert!(
        storage
            .prepare_restore_backup(&folder, id(99), 42, LIMITS)
            .await
            .is_err()
    );
    assert_eq!(files(&folder), old);
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
    assert_eq!(source(&storage).await, before);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn verification_rejects_resealed_substitution_truncation_growth_extra_files_and_stricter_limits()
 {
    let (_root, storage, folder) = fixture().await;
    let candidate = storage
        .prepare_restore_backup(&folder, id(1), 42, LIMITS)
        .await
        .unwrap();
    let path = candidate.path().join(ARCHIVE);
    let bytes = fs::read(&path).unwrap();
    let mut resealed = bytes.clone();
    let offset = resealed.windows(2).position(|part| part == b"42").unwrap();
    resealed[offset] = b'5';
    let body = resealed.len() - 32;
    let hash = Sha256::digest(&resealed[..body]);
    resealed[body..].copy_from_slice(&hash);
    SpaceSnapshot::decode(&resealed, LIMITS).unwrap();
    let mut oversized = bytes.clone();
    oversized.resize(LIMITS.max_bytes + 1, 0);
    for invalid in [bytes[..bytes.len() - 1].to_vec(), oversized, resealed] {
        fs::write(&path, invalid).unwrap();
        assert!(candidate.verify(LIMITS).is_err());
    }
    fs::write(&path, &bytes).unwrap();
    for limits in [
        SnapshotLimits {
            max_bytes: bytes.len() - 1,
            ..LIMITS
        },
        SnapshotLimits {
            max_changes: 1,
            ..LIMITS
        },
    ] {
        assert!(candidate.verify(limits).is_err());
    }
    fs::write(candidate.path().join("extra"), b"unexpected").unwrap();
    assert!(candidate.verify(LIMITS).is_err());
    fs::remove_file(candidate.path().join("extra")).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(candidate.verify(LIMITS).is_err());
    fs::create_dir(&path).unwrap();
    assert!(candidate.verify(LIMITS).is_err());
    fs::remove_dir(&path).unwrap();
    fs::write(&path, bytes).unwrap();
    candidate.verify(LIMITS).unwrap();
    candidate.discard().unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn capture_holds_writer_lock_through_file_verification_and_releases_it_on_return() {
    let (root, storage, folder) = fixture().await;
    let path = root.path().join("source.db");
    let other = Connection::open(&path).unwrap();
    other.busy_timeout(Duration::ZERO).unwrap();
    let candidate = storage
        .run(move |connection| {
            prepare(connection, &folder, id(1), 42, LIMITS, |_, _| {
                let other = Connection::open(&path)?;
                other.busy_timeout(Duration::ZERO)?;
                let error = other.execute_batch("BEGIN IMMEDIATE").unwrap_err();
                assert_eq!(
                    error.sqlite_error_code(),
                    Some(rusqlite::ErrorCode::DatabaseBusy)
                );
                Ok(())
            })
        })
        .await
        .unwrap();
    other.execute_batch("BEGIN IMMEDIATE; ROLLBACK").unwrap();
    candidate.verify(LIMITS).unwrap();
    candidate.discard().unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_source_model_history_and_schema_preserve_files_and_data() {
    for fault in ["model", "history", "schema"] {
        let (_root, storage, folder) = fixture().await;
        storage
            .transact(move |ctx| {
                match fault {
                    "model" => ctx.edit(&id(1), |tx| {
                        tx.put(ROOT, "unknown", "invalid")?;
                        Ok(())
                    })?,
                    "history" => {
                        let graph = ctx.read().document(&id(1))?;
                        let existing = cards::history(graph, &cards::object(graph, &id(4))?)?[0]
                            .id
                            .clone();
                        ctx.connection()
                            .execute("UPDATE card_editing SET pending_id = ?1", [existing])?;
                    }
                    "schema" => {
                        ctx.connection()
                            .execute_batch("CREATE TABLE unexpected (value TEXT)")?;
                    }
                    _ => unreachable!(),
                }
                Ok(())
            })
            .await
            .unwrap();
        let before = source(&storage).await;
        let old = files(&folder);
        assert!(
            storage
                .prepare_restore_backup(&folder, id(1), 42, LIMITS)
                .await
                .is_err()
        );
        assert_eq!(source(&storage).await, before);
        assert_eq!(files(&folder), old);
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 1);
        storage.shutdown().await.unwrap();
    }
}

#[cfg(unix)]
#[tokio::test]
async fn verification_rejects_symbolic_links_and_capture_does_not_follow_folder_links() {
    let (root, storage, folder) = fixture().await;
    let candidate = storage
        .prepare_restore_backup(&folder, id(1), 42, LIMITS)
        .await
        .unwrap();
    let archive = candidate.path().join(ARCHIVE);
    let real = root.path().join("original.bin");
    fs::rename(&archive, &real).unwrap();
    std::os::unix::fs::symlink(&real, &archive).unwrap();
    assert!(candidate.verify(LIMITS).is_err());
    let alias = root.path().join("alias");
    std::os::unix::fs::symlink(&folder, &alias).unwrap();
    assert!(
        storage
            .prepare_restore_backup(&alias, id(1), 42, LIMITS)
            .await
            .is_err()
    );
    candidate.discard().unwrap();
    assert!(real.is_file());
    storage.shutdown().await.unwrap();
}

#[test]
#[ignore]
fn crash_probe() {
    let (Ok(database), Ok(folder), Ok(point)) = (
        std::env::var("ZEDONE_RESTORE_TEST_DB"),
        std::env::var("ZEDONE_RESTORE_TEST_FOLDER"),
        std::env::var("ZEDONE_RESTORE_TEST_POINT"),
    ) else {
        return;
    };
    let mut connection = Connection::open(database).unwrap();
    prepare(
        &mut connection,
        Path::new(&folder),
        id(1),
        42,
        LIMITS,
        |at, _| {
            if format!("{at:?}") == point {
                std::process::exit(73);
            }
            Ok(())
        },
    )
    .unwrap();
    panic!("crash checkpoint was not reached");
}

#[tokio::test]
async fn process_crashes_preserve_source_and_old_copy_without_promoting_candidate_files() {
    let (root, storage, folder) = fixture().await;
    let before = source(&storage).await;
    for point in ["BeforeWrite", "AfterWrite", "BeforeVerify", "AfterVerify"] {
        let status = Command::new(std::env::current_exe().unwrap())
            .args(["--ignored", "--exact", "restore_backup::tests::crash_probe"])
            .env("ZEDONE_RESTORE_TEST_DB", root.path().join("source.db"))
            .env("ZEDONE_RESTORE_TEST_FOLDER", &folder)
            .env("ZEDONE_RESTORE_TEST_POINT", point)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(73));
        assert_eq!(source(&storage).await, before);
        assert_eq!(
            fs::read(folder.join("previous.bin")).unwrap(),
            b"previous successful backup"
        );
        let other = Connection::open(root.path().join("source.db")).unwrap();
        other.execute_batch("BEGIN IMMEDIATE; ROLLBACK").unwrap();
    }
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 5);
    storage.shutdown().await.unwrap();
}
