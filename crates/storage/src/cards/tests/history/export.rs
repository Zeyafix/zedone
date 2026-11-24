use super::*;
use crate::cards;
use crate::snapshot::{SnapshotLimits, SpaceSnapshot};
use automerge::ReadDoc;

const LIMITS: SnapshotLimits = SnapshotLimits {
    max_bytes: 1024 * 1024,
    max_changes: 1000,
};

fn graph(snapshot: &SpaceSnapshot) -> Automerge {
    let mut graph = Automerge::new();
    graph
        .apply_changes(snapshot.changes().iter().cloned())
        .unwrap();
    graph
}

async fn source(storage: &Storage) -> (Vec<u8>, Vec<String>, i64) {
    let bytes = storage
        .snapshot(sid(1), 42, LIMITS)
        .await
        .unwrap()
        .encode(LIMITS)
        .unwrap();
    let (sessions, receipts) = storage.read(|ctx| {
        let sessions = ctx.connection().prepare(
            "SELECT quote(card) || quote(subject) || quote(session) || quote(pending_id) || quote(title) || quote(description) || quote(changed_ms) FROM card_editing ORDER BY space, card",
        )?.query_map([], |r| r.get(0))?.collect::<Result<Vec<String>, _>>()?;
        let receipts = ctx.connection().query_row("SELECT count(*) FROM command_results", [], |r| r.get(0))?;
        Ok((sessions, receipts))
    }).await.unwrap();
    (bytes, sessions, receipts)
}

async fn pending(storage: &Storage, admin: &PendingAccess) -> zedone_core::cards::CardSnapshot {
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
    at!(
        storage,
        admin,
        1000,
        autosave,
        save(3, 2, "Сохранено ё🙂", "**последняя пара**", first.version)
    )
    .unwrap()
    .result
}

#[tokio::test]
async fn export_keeps_pending_history_without_finishing_or_changing_the_source_series() {
    let (dir, storage, admin) = setup().await;
    let current = pending(&storage, &admin).await;
    let before = source(&storage).await;
    let snapshot = storage.export_snapshot(sid(1), 42, LIMITS).await.unwrap();
    let bytes = snapshot.encode(LIMITS).unwrap();
    assert_eq!(source(&storage).await, before);
    let decoded = SpaceSnapshot::decode(&bytes, LIMITS).unwrap();
    let archived = graph(&decoded);
    let history = cards::history(&archived, &cards::object(&archived, &sid(9)).unwrap()).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[1].title, "Сохранено ё🙂");
    assert_eq!(history[1].description, "**последняя пара**");
    assert_eq!(
        decoded.changes().len(),
        graph(&SpaceSnapshot::decode(&before.0, LIMITS).unwrap())
            .get_changes(&[])
            .len()
            + 1
    );
    let next = at!(
        storage,
        admin,
        2000,
        autosave,
        save(4, 2, "Следующий текст", "другая пара", current.version)
    )
    .unwrap()
    .result;
    at!(
        storage,
        admin,
        2001,
        finish_editing,
        finish(5, 2, next.version)
    )
    .unwrap();
    let completed = versions(&storage, 9).await;
    assert_eq!(completed.len(), 2);
    assert_eq!(completed[1].id, history[1].id);
    assert_eq!(completed[1].title, "Следующий текст");
    assert_eq!(decoded.encode(LIMITS).unwrap(), bytes);
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert_eq!(versions(&storage, 9).await, completed);
    assert_eq!(decoded.encode(LIMITS).unwrap(), bytes);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn export_preserves_product_trash_tags_conflicts_and_every_orphan_change() {
    let (_dir, storage, admin) = setup().await;
    run!(storage, admin, create, creation(1, 9)).unwrap();
    let card = run!(storage, admin, create, creation(2, 8)).unwrap().result;
    run!(
        storage,
        admin,
        trash,
        req(
            3,
            TrashCard {
                space: sid(1),
                card: sid(8),
                expected: card.version
            }
        )
    )
    .unwrap();
    storage
        .transact(|ctx| {
            use zedone_core::tags::TagStore;
            let tag = ctx.create_tag(&zedone_core::tags::CreateTag {
                space: sid(1),
                tag: sid(4),
                name: "Работа".into(),
            })?;
            let card = ctx.card_snapshot(&sid(1), &sid(9))?;
            ctx.set_card_tag(&zedone_core::tags::SetCardTag {
                space: sid(1),
                card: sid(9),
                tag: sid(4),
                present: true,
                expected: card.version,
                expected_tag: tag.version,
            })?;
            let base = ctx.read().document(&sid(1))?.clone();
            let mut left = base.fork();
            let mut right = base.fork();
            for (graph, name) in [(&mut left, "ПК"), (&mut right, "Телефон")] {
                let mut tx = graph.transaction();
                tx.put(automerge::ROOT, "name", name)?;
                tx.commit();
            }
            let mut remote = base.fork();
            let object = cards::object(&remote, &sid(9))?;
            let mut tx = remote.transaction();
            tx.put(&object, "title", "пропущенная зависимость")?;
            tx.commit();
            let mut tx = remote.transaction();
            tx.put(&object, "title", "ожидающее изменение")?;
            let (hash, _) = tx.commit();
            let orphan = remote.get_change_by_hash(&hash.unwrap()).unwrap();
            let mut changes = left.get_changes(&base.get_heads());
            changes.extend(right.get_changes(&base.get_heads()));
            changes.push(orphan);
            ctx.apply_changes(&sid(1), changes)?;
            ctx.create_document("other")?;
            ctx.edit("other", |tx| {
                tx.put(automerge::ROOT, "private", "OTHER-SPACE-SECRET")?;
                Ok(())
            })?;
            Ok(())
        })
        .await
        .unwrap();
    let before = source(&storage).await;
    let snapshot = storage.export_snapshot(sid(1), 42, LIMITS).await.unwrap();
    let bytes = snapshot.encode(LIMITS).unwrap();
    assert_eq!(bytes, before.0);
    assert_eq!(source(&storage).await, before);
    let restored = graph(&SpaceSnapshot::decode(&bytes, LIMITS).unwrap());
    let space = crate::spaces::snapshot(&restored, &sid(1), true).unwrap();
    assert_eq!(space.names.len(), 2);
    let contents = space.contents.unwrap();
    assert_eq!(contents.columns.len(), 2);
    assert_eq!(contents.cards.len(), 2);
    assert!(
        contents
            .cards
            .iter()
            .any(|c| c.card == sid(8) && !c.deletion_groups.is_empty())
    );
    assert!(
        contents
            .cards
            .iter()
            .any(|c| c.card == sid(9) && !c.tags.is_empty())
    );
    assert_eq!(contents.tags.len(), 1);
    assert_eq!(restored.get_missing_deps(&[]).len(), 1);
    assert!(
        !bytes
            .windows(b"OTHER-SPACE-SECRET".len())
            .any(|s| s == b"OTHER-SPACE-SECRET")
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn export_budget_failures_include_overlay_and_leave_the_source_unchanged() {
    let (_dir, storage, admin) = setup().await;
    pending(&storage, &admin).await;
    let before = source(&storage).await;
    let raw = SpaceSnapshot::decode(&before.0, LIMITS).unwrap();
    for limits in [
        SnapshotLimits {
            max_bytes: before.0.len() - 1,
            ..LIMITS
        },
        SnapshotLimits {
            max_changes: raw.changes().len(),
            ..LIMITS
        },
        SnapshotLimits {
            max_bytes: before.0.len(),
            ..LIMITS
        },
    ] {
        assert_eq!(
            storage
                .export_snapshot(sid(1), 42, limits)
                .await
                .unwrap_err(),
            StorageError::Unavailable
        );
        assert_eq!(source(&storage).await, before);
    }
    assert!(storage.export_snapshot(sid(1), 42, LIMITS).await.is_ok());
    assert_eq!(source(&storage).await, before);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn export_refuses_invalid_product_or_pending_history_without_repairing_the_source() {
    let (_dir, storage, admin) = setup().await;
    pending(&storage, &admin).await;
    let existing = versions(&storage, 9).await[0].id.clone();
    storage
        .transact(move |ctx| {
            ctx.connection()
                .execute("UPDATE card_editing SET pending_id = ?1", [existing])?;
            Ok(())
        })
        .await
        .unwrap();
    let before = source(&storage).await;
    assert_eq!(
        storage
            .export_snapshot(sid(1), 42, LIMITS)
            .await
            .unwrap_err(),
        StorageError::InvalidDatabase
    );
    assert_eq!(source(&storage).await, before);
    storage
        .transact(|ctx| {
            ctx.connection()
                .execute("UPDATE card_editing SET pending_id = 'invalid'", [])?;
            Ok(())
        })
        .await
        .unwrap();
    let before = source(&storage).await;
    assert_eq!(
        storage
            .export_snapshot(sid(1), 42, LIMITS)
            .await
            .unwrap_err(),
        StorageError::InvalidDatabase
    );
    assert_eq!(source(&storage).await, before);
    storage
        .transact(|ctx| {
            ctx.edit(&sid(1), |tx| {
                tx.put(automerge::ROOT, "format", "future-format")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let before = source(&storage).await;
    assert_eq!(
        storage
            .export_snapshot(sid(1), 42, LIMITS)
            .await
            .unwrap_err(),
        StorageError::InvalidDatabase
    );
    assert_eq!(source(&storage).await, before);
    rejected(
        storage.export_snapshot(sid(99), 42, LIMITS).await,
        ErrorCode::NotFound,
    );
    storage.shutdown().await.unwrap();
}
