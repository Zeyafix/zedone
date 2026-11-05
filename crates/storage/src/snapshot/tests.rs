use automerge::{ActorId, ROOT, ReadDoc, sync::State, transaction::Transactable};
use tempfile::TempDir;

use super::*;

const LIMITS: SnapshotLimits = SnapshotLimits {
    max_bytes: 1024 * 1024,
    max_changes: 1000,
};
const SPACE: &str = "пространство/ё🙂";

fn commit(doc: &mut Automerge, field: &str, value: &str) -> Change {
    let mut tx = doc.transaction();
    tx.put(ROOT, field, value).unwrap();
    let (hash, _) = tx.commit();
    doc.get_change_by_hash(&hash.unwrap()).unwrap()
}

fn graph(snapshot: &SpaceSnapshot) -> Automerge {
    let mut doc = Automerge::new();
    doc.apply_changes(snapshot.changes().iter().cloned())
        .unwrap();
    doc
}

fn fixture() -> SpaceSnapshot {
    let mut doc = Automerge::new().with_actor(ActorId::from(vec![1]));
    let change = commit(&mut doc, "title", "Эталон ё🙂");
    SpaceSnapshot {
        metadata: SnapshotMetadata {
            space: SPACE.into(),
            created_at_unix_ms: 42,
        },
        changes: vec![change],
    }
}

fn envelope(metadata: &[u8], records: &[&[u8]]) -> Vec<u8> {
    let mut bytes = MAGIC.to_vec();
    bytes.extend_from_slice(&VERSION.to_le_bytes());
    bytes.extend_from_slice(&(metadata.len() as u64).to_le_bytes());
    bytes.extend_from_slice(&(records.len() as u64).to_le_bytes());
    bytes.extend_from_slice(metadata);
    for record in records {
        bytes.extend_from_slice(&(record.len() as u64).to_le_bytes());
        bytes.extend_from_slice(record);
    }
    bytes.extend_from_slice(&[0; DIGEST]);
    reseal(&mut bytes);
    bytes
}

fn reseal(bytes: &mut [u8]) {
    let body = bytes.len() - DIGEST;
    let hash = Sha256::digest(&bytes[..body]);
    bytes[body..].copy_from_slice(&hash);
}

#[tokio::test]
async fn complete_scoped_log_roundtrip_keeps_conflicts_orphans_history_and_reverse() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("device.db");
    let storage = Storage::open(&path).await.unwrap();
    let mut base = Automerge::new().with_actor(ActorId::from(vec![1]));
    commit(&mut base, "title", "раньше");
    commit(&mut base, "history", "вся история");
    commit(&mut base, "trash", "удалённая карточка");
    let mut left = base.fork().with_actor(ActorId::from(vec![2]));
    let mut right = base.fork().with_actor(ActorId::from(vec![3]));
    commit(&mut left, "title", "ПК");
    commit(&mut right, "title", "телефон");
    left.merge(&mut right).unwrap();
    let mut remote = base.fork().with_actor(ActorId::from(vec![4]));
    let missing = commit(&mut remote, "late", "dependency");
    let orphan = commit(&mut remote, "late", "ожидает зависимость");
    let orphan_hash = orphan.hash();
    let mut changes = left.get_changes(&[]);
    changes.push(orphan);
    let expected_count = changes.len();
    storage
        .write(
            b"private-receipt".to_vec(),
            b"private-request".to_vec(),
            move |ctx| {
                ctx.create_document(SPACE)?;
                ctx.apply_changes(SPACE, changes)?;
                ctx.set_peer_state(SPACE, b"private-peer", &State::new())?;
                ctx.create_document("other")?;
                ctx.edit("other", |tx| {
                    tx.put(ROOT, "secret", "OTHER-SPACE-SECRET")?;
                    Ok(())
                })?;
                Ok("PRIVATE-RECEIPT-RESULT")
            },
        )
        .await
        .unwrap();
    let before = storage.snapshot(SPACE.into(), 42, LIMITS).await.unwrap();
    let bytes = before.encode(LIMITS).unwrap();
    let decoded = SpaceSnapshot::decode(&bytes, LIMITS).unwrap();
    assert_eq!(decoded.metadata(), before.metadata());
    assert_eq!(decoded.encode(LIMITS).unwrap(), bytes);
    assert_eq!(decoded.changes().len(), expected_count);
    assert!(decoded.changes().iter().any(|c| c.hash() == orphan_hash));
    let mut doc = graph(&decoded);
    assert_eq!(doc.get_all(ROOT, "title").unwrap().len(), 2);
    assert!(doc.get(ROOT, "history").unwrap().is_some());
    assert!(doc.get(ROOT, "trash").unwrap().is_some());
    assert_eq!(doc.get_missing_deps(&[]), vec![missing.hash()]);
    doc.apply_changes([missing]).unwrap();
    assert_eq!(
        doc.get(ROOT, "late").unwrap().unwrap().0.as_str(),
        Some("ожидает зависимость")
    );
    for secret in [
        "OTHER-SPACE-SECRET",
        "private-peer",
        "private-request",
        "PRIVATE-RECEIPT-RESULT",
    ] {
        assert!(!bytes.windows(secret.len()).any(|w| w == secret.as_bytes()));
    }
    storage
        .write(b"change".to_vec(), vec![], |ctx| {
            ctx.edit(SPACE, |tx| {
                tx.put(ROOT, "done", true)?;
                Ok(())
            })
        })
        .await
        .unwrap();
    storage
        .write(b"reverse".to_vec(), vec![], |ctx| {
            ctx.edit(SPACE, |tx| {
                tx.put(ROOT, "done", false)?;
                Ok(())
            })
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(&path).await.unwrap();
    let after = storage.snapshot(SPACE.into(), 43, LIMITS).await.unwrap();
    assert_eq!(after.changes().len(), expected_count + 2);
    assert_eq!(
        graph(&after)
            .get(ROOT, "done")
            .unwrap()
            .unwrap()
            .0
            .as_bool(),
        Some(false)
    );
    assert!(graph(&decoded).get(ROOT, "done").unwrap().is_none());
    storage
        .read(|ctx| {
            assert!(ctx.recorded(b"private-receipt")?.is_some());
            assert!(ctx.document("other")?.get(ROOT, "secret")?.is_some());
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[test]
fn every_truncation_and_single_byte_corruption_is_rejected_without_touching_source() {
    let original = fixture().encode(LIMITS).unwrap();
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("snapshot.bin");
    std::fs::write(&path, &original).unwrap();
    let file_bytes = std::fs::read(&path).unwrap();
    SpaceSnapshot::decode(&file_bytes, LIMITS).unwrap();
    for len in 0..original.len() {
        assert!(
            SpaceSnapshot::decode(&file_bytes[..len], LIMITS).is_err(),
            "length {len}"
        );
    }
    for offset in 0..original.len() {
        let mut bad = original.clone();
        bad[offset] ^= 1;
        assert!(
            SpaceSnapshot::decode(&bad, LIMITS).is_err(),
            "offset {offset}"
        );
    }
    let mut appended = original.clone();
    appended.push(0);
    assert!(SpaceSnapshot::decode(&appended, LIMITS).is_err());
    assert_eq!(std::fs::read(path).unwrap(), original);
}

#[test]
fn resealed_malformed_lengths_metadata_records_and_order_are_rejected() {
    let snapshot = fixture();
    let good = snapshot.encode(LIMITS).unwrap();
    let metadata = metadata_bytes(snapshot.metadata()).unwrap();
    for offset in [12, 20, HEADER + metadata.len()] {
        let mut bad = good.clone();
        bad[offset..offset + 8].copy_from_slice(&u64::MAX.to_le_bytes());
        reseal(&mut bad);
        assert!(SpaceSnapshot::decode(&bad, LIMITS).is_err());
    }
    for metadata in [
        br#"{}"#.as_slice(),
        br#"{"space":"","created_at_unix_ms":1}"#,
        br#"{"space":"a","space":"b","created_at_unix_ms":1}"#,
        br#"{"space":"a","created_at_unix_ms":-1}"#,
        br#"{"space":"a","created_at_unix_ms":1,"rights":true}"#,
        br#"{"space":"a","created_at_unix_ms":1} {}"#,
    ] {
        assert_eq!(
            SpaceSnapshot::decode(&envelope(metadata, &[]), LIMITS).unwrap_err(),
            SnapshotError::Invalid
        );
    }
    let change = snapshot.changes()[0].raw_bytes();
    for record in [
        &[][..],
        b"not automerge".as_slice(),
        &change[..change.len() - 1],
    ] {
        assert_eq!(
            SpaceSnapshot::decode(&envelope(&metadata, &[record]), LIMITS).unwrap_err(),
            SnapshotError::Invalid
        );
    }
    assert!(SpaceSnapshot::decode(&envelope(&metadata, &[change, change]), LIMITS).is_err());
    let mut other = Automerge::new().with_actor(ActorId::from(vec![2]));
    let second = commit(&mut other, "other", "value");
    let mut records = [snapshot.changes()[0].clone(), second];
    records.sort_by_key(|c| std::cmp::Reverse(c.hash()));
    assert!(
        SpaceSnapshot::decode(
            &envelope(&metadata, &[records[0].raw_bytes(), records[1].raw_bytes()]),
            LIMITS
        )
        .is_err()
    );
    let mut trailing = good.clone();
    trailing.insert(trailing.len() - DIGEST, 0);
    reseal(&mut trailing);
    assert!(SpaceSnapshot::decode(&trailing, LIMITS).is_err());
    let mut compressed = change.to_vec();
    compressed[8] = 2;
    assert!(SpaceSnapshot::decode(&envelope(&metadata, &[&compressed]), LIMITS).is_err());
    let mut wrong_checksum = change.to_vec();
    wrong_checksum[4] ^= 1;
    assert!(SpaceSnapshot::decode(&envelope(&metadata, &[&wrong_checksum]), LIMITS).is_err());
    let mut collision = Automerge::new().with_actor(ActorId::from(vec![1]));
    let same_sequence = commit(&mut collision, "title", "incompatible actor sequence");
    let mut records = [snapshot.changes()[0].clone(), same_sequence];
    records.sort_by_key(Change::hash);
    assert!(
        SpaceSnapshot::decode(
            &envelope(&metadata, &[records[0].raw_bytes(), records[1].raw_bytes()]),
            LIMITS
        )
        .is_err()
    );
    let mut future = good;
    future[8..12].copy_from_slice(&2_u32.to_le_bytes());
    reseal(&mut future);
    assert_eq!(
        SpaceSnapshot::decode(&future, LIMITS).unwrap_err(),
        SnapshotError::UnsupportedVersion
    );
}

#[test]
fn explicit_budgets_have_exact_boundaries_and_empty_spaces_are_valid() {
    let snapshot = fixture();
    let bytes = snapshot.encode(LIMITS).unwrap();
    let exact = SnapshotLimits {
        max_bytes: bytes.len(),
        max_changes: 1,
    };
    assert_eq!(snapshot.encode(exact).unwrap(), bytes);
    SpaceSnapshot::decode(&bytes, exact).unwrap();
    for limits in [
        SnapshotLimits {
            max_bytes: bytes.len() - 1,
            ..exact
        },
        SnapshotLimits {
            max_changes: 0,
            ..exact
        },
    ] {
        assert_eq!(
            snapshot.encode(limits).unwrap_err(),
            SnapshotError::LimitExceeded
        );
        assert_eq!(
            SpaceSnapshot::decode(&bytes, limits).unwrap_err(),
            SnapshotError::LimitExceeded
        );
    }
    let empty = SpaceSnapshot {
        metadata: snapshot.metadata,
        changes: vec![],
    };
    assert!(
        SpaceSnapshot::decode(&empty.encode(LIMITS).unwrap(), LIMITS)
            .unwrap()
            .changes()
            .is_empty()
    );
}

#[test]
fn initial_v1_fixture_remains_readable_and_byte_identical() {
    let bytes = include_bytes!("../../tests/fixtures/space-v1.bin");
    let snapshot = SpaceSnapshot::decode(bytes, LIMITS).unwrap();
    assert_eq!(snapshot.metadata().space, SPACE);
    assert_eq!(snapshot.metadata().created_at_unix_ms, 42);
    assert_eq!(snapshot.changes().len(), 1);
    assert_eq!(
        graph(&snapshot)
            .get(ROOT, "title")
            .unwrap()
            .unwrap()
            .0
            .as_str(),
        Some("Эталон ё🙂")
    );
    assert_eq!(snapshot.encode(LIMITS).unwrap(), bytes);
    assert_eq!(fixture().encode(LIMITS).unwrap(), bytes);
}

#[tokio::test]
async fn capture_uses_one_sqlite_snapshot_during_another_workers_commit() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("device.db");
    let storage = Storage::open(&path).await.unwrap();
    let initial = fixture();
    storage
        .write(vec![1], vec![], move |ctx| {
            ctx.create_document(SPACE)?;
            ctx.apply_changes(SPACE, initial.changes)?;
            Ok(())
        })
        .await
        .unwrap();
    let mut reader =
        Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let tx = reader.transaction().unwrap();
    let count: i64 = tx
        .query_row("SELECT count(*) FROM changes", [], |r| r.get(0))
        .unwrap();
    assert_eq!(count, 1);
    let other = Storage::open(&path).await.unwrap();
    other
        .write(vec![2], vec![], |ctx| {
            ctx.edit(SPACE, |tx| {
                tx.put(ROOT, "title", "after")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let snapshot = capture(
        &tx,
        SnapshotMetadata {
            space: SPACE.into(),
            created_at_unix_ms: 42,
        },
        LIMITS,
    )
    .unwrap();
    assert_eq!(snapshot.changes().len(), 1);
    assert_eq!(
        graph(&snapshot)
            .get(ROOT, "title")
            .unwrap()
            .unwrap()
            .0
            .as_str(),
        Some("Эталон ё🙂")
    );
    tx.commit().unwrap();
    assert_eq!(
        storage
            .snapshot(SPACE.into(), 43, LIMITS)
            .await
            .unwrap()
            .changes()
            .len(),
        2
    );
    other.shutdown().await.unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn capture_rejects_missing_space_limits_and_corrupt_storage_without_writes() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("device.db");
    let storage = Storage::open(&path).await.unwrap();
    assert!(matches!(
        storage.snapshot("missing".into(), 0, LIMITS).await,
        Err(StorageError::Rejected(zedone_core::error::CommandError {
            code: zedone_core::error::ErrorCode::NotFound
        }))
    ));
    let snapshot = fixture();
    storage
        .write(vec![1], vec![], move |ctx| {
            ctx.create_document(SPACE)?;
            ctx.apply_changes(SPACE, snapshot.changes)?;
            Ok(())
        })
        .await
        .unwrap();
    let limits = SnapshotLimits {
        max_bytes: 1,
        ..LIMITS
    };
    assert!(matches!(
        storage.snapshot(SPACE.into(), 0, limits).await,
        Err(StorageError::Unavailable)
    ));
    storage
        .run(|c| {
            c.execute("UPDATE changes SET hash = zeroblob(32)", [])?;
            Ok(())
        })
        .await
        .unwrap();
    assert!(matches!(
        storage.snapshot(SPACE.into(), 0, LIMITS).await,
        Err(StorageError::InvalidDatabase)
    ));
    storage.shutdown().await.unwrap();
    let original = std::fs::read(&path).unwrap();
    let storage = Storage::open(&path).await.unwrap();
    assert!(matches!(
        storage.snapshot(SPACE.into(), 0, LIMITS).await,
        Err(StorageError::InvalidDatabase)
    ));
    storage.shutdown().await.unwrap();
    assert_eq!(std::fs::read(path).unwrap(), original);
}
