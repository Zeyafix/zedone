use super::*;
use crate::{Stage, restore_backup::tests::fixture};
use automerge::{ObjType, ROOT, ReadDoc, transaction::Transactable};
use std::fs;
use zedone_core::{recovery::RecoveryStore, spaces::SpaceStore};
fn id(value: u8) -> String {
    format!("{value:032x}")
}

#[tokio::test]
async fn inferred_card_column_and_space_deletion_conflicts_survive_recovery_and_normal_resolution()
{
    use zedone_core::cards::CardStore;
    use zedone_core::columns::ColumnStore;
    for kind in ["card", "column", "space"] {
        let (_root, storage, _) = fixture().await;
        storage
            .transact(move |ctx| {
                ctx.finish_pending_text(&id(1), &id(4))?;
                let base = ctx.read().document(&id(1))?.clone();
                let object = match kind {
                    "card" => crate::cards::object(&base, &id(4))?,
                    "column" => crate::columns::object(&base, &id(3))?,
                    _ => ROOT,
                };
                let mut deleted = base.fork();
                let mut changed = base.fork();
                let mut tx = deleted.transaction();
                tx.put(&object, "lifecycle", format!("trash:{}:{}", id(20), id(21)))?;
                tx.commit();
                let card = crate::cards::object(&changed, &id(4))?;
                let mut tx = changed.transaction();
                tx.put(&card, "title", "Независимая правка")?;
                tx.commit();
                let mut changes = deleted.get_changes(&base.get_heads());
                changes.extend(changed.get_changes(&base.get_heads()));
                ctx.apply_changes(&id(1), changes)?;
                Ok(())
            })
            .await
            .unwrap();
        let saved = archive(&storage).await;
        storage
            .transact(move |ctx| {
                match kind {
                    "card" => ctx.set_card_lifecycle(&id(1), &id(4), "active")?,
                    "column" => ctx.set_column_lifecycle(&id(1), &id(3), "active")?,
                    _ => ctx.set_space_lifecycle(&id(1), "active")?,
                };
                Ok(())
            })
            .await
            .unwrap();
        restore(&storage, saved).await.unwrap();
        storage
            .transact(move |ctx| {
                match kind {
                    "card" => {
                        assert!(ctx.card_snapshot(&id(1), &id(4))?.deletion_conflict);
                        ctx.set_card_lifecycle(&id(1), &id(4), "active")?;
                        assert!(!ctx.card_snapshot(&id(1), &id(4))?.deletion_conflict);
                    }
                    "column" => {
                        assert_eq!(
                            ctx.column_snapshot(&id(1), &id(3))?.state,
                            zedone_core::spaces::SpaceState::Conflicted
                        );
                        ctx.set_column_lifecycle(&id(1), &id(3), "active")?;
                        assert_eq!(
                            ctx.column_snapshot(&id(1), &id(3))?.state,
                            zedone_core::spaces::SpaceState::Active
                        );
                    }
                    _ => {
                        assert_eq!(
                            ctx.space_snapshot(&id(1))?.state,
                            zedone_core::spaces::SpaceState::Conflicted
                        );
                        ctx.set_space_lifecycle(&id(1), "active")?;
                        assert_eq!(
                            ctx.space_snapshot(&id(1))?.state,
                            zedone_core::spaces::SpaceState::Active
                        );
                    }
                }
                Ok(())
            })
            .await
            .unwrap();
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn later_concurrent_tag_alias_is_retired_without_hiding_the_restored_tag_or_its_assignment() {
    let (_root, storage, _) = fixture().await;
    let saved = archive(&storage).await;
    storage
        .transact(|ctx| {
            let graph = ctx.read().document(&id(1))?.clone();
            let tag = graph.get(ROOT, format!("tag:{}", id(7)))?.unwrap().1;
            let hash = graph.hash_for_opid(&tag).unwrap();
            let deps = graph.get_change_by_hash(&hash).unwrap().deps().to_vec();
            let mut peer = graph.fork_at(&deps)?;
            let mut tx = peer.transaction();
            let object = tx.put_object(ROOT, format!("tag:{}", id(6)), ObjType::Map)?;
            tx.put(&object, "format", "zedone.tag.v1")?;
            tx.put(&object, "name", "Работа")?;
            tx.put(&object, "deleted", false)?;
            tx.commit();
            ctx.apply_changes(&id(1), peer.get_changes(&[]))?;
            assert_eq!(
                ctx.space_snapshot(&id(1))?
                    .contents
                    .unwrap()
                    .cards
                    .iter()
                    .find(|c| c.card == id(4))
                    .unwrap()
                    .tags,
                [id(6)]
            );
            Ok(())
        })
        .await
        .unwrap();
    restore(&storage, saved).await.unwrap();
    storage
        .transact(|ctx| {
            let state = ctx.space_snapshot(&id(1))?.contents.unwrap();
            assert_eq!(
                state.cards.iter().find(|c| c.card == id(4)).unwrap().tags,
                [id(7)]
            );
            assert!(
                state
                    .tags
                    .iter()
                    .any(|t| t.tag == id(7) && t.state == zedone_core::tags::TagState::Active)
            );
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn purge_against_independent_edit_stays_visible_after_restore() {
    use zedone_core::cards::CardStore;
    let (_root, storage, _) = fixture().await;
    storage
        .transact(|ctx| {
            ctx.finish_pending_text(&id(1), &id(4))?;
            let base = ctx.read().document(&id(1))?.clone();
            let object = crate::cards::object(&base, &id(4))?;
            let mut deleted = base.fork();
            let mut edited = base.fork();
            let mut tx = deleted.transaction();
            tx.put(&object, "lifecycle", "purged")?;
            tx.commit();
            let mut tx = edited.transaction();
            tx.put(&object, "title", "Правка против очистки")?;
            tx.commit();
            let mut changes = deleted.get_changes(&base.get_heads());
            changes.extend(edited.get_changes(&base.get_heads()));
            ctx.apply_changes(&id(1), changes)?;
            assert_eq!(
                ctx.card_snapshot(&id(1), &id(4))?.state,
                zedone_core::spaces::SpaceState::Conflicted
            );
            Ok(())
        })
        .await
        .unwrap();
    let saved = archive(&storage).await;
    storage
        .transact(|ctx| Ok(ctx.set_card_lifecycle(&id(1), &id(4), "active")?))
        .await
        .unwrap();
    restore(&storage, saved).await.unwrap();
    storage
        .transact(|ctx| {
            assert_eq!(
                ctx.card_snapshot(&id(1), &id(4))?.state,
                zedone_core::spaces::SpaceState::Conflicted
            );
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn tag_deletion_assignment_conflict_is_restored_and_can_be_resolved_normally() {
    use zedone_core::tags::TagStore;
    let (_root, storage, _) = fixture().await;
    storage
        .transact(|ctx| {
            ctx.finish_pending_text(&id(1), &id(4))?;
            let graph = ctx.read().document(&id(1))?.clone();
            let tag = graph.get(ROOT, format!("tag:{}", id(7)))?.unwrap().1;
            let card = crate::cards::object(&graph, &id(4))?;
            let tags = graph.get(&card, "tags")?.unwrap().1;
            let mut deleted = graph.fork();
            let mut assigned = graph.fork();
            let mut tx = deleted.transaction();
            tx.put(&tag, "deleted", true)?;
            tx.delete(&tags, id(7))?;
            tx.put(&tags, id(7), false)?;
            tx.commit();
            let mut tx = assigned.transaction();
            tx.delete(&tags, id(7))?;
            tx.put(&tags, id(7), true)?;
            tx.commit();
            let mut changes = deleted.get_changes(&graph.get_heads());
            changes.extend(assigned.get_changes(&graph.get_heads()));
            ctx.apply_changes(&id(1), changes)?;
            assert!(ctx.tag_snapshot(&id(1), &id(7))?.deletion_conflict);
            Ok(())
        })
        .await
        .unwrap();
    let saved = archive(&storage).await;
    storage
        .transact(|ctx| {
            ctx.resolve_tag_deletion(&id(1), &id(7), false)?;
            Ok(())
        })
        .await
        .unwrap();
    restore(&storage, saved).await.unwrap();
    storage
        .transact(|ctx| {
            assert!(ctx.tag_snapshot(&id(1), &id(7))?.deletion_conflict);
            assert!(
                ctx.space_snapshot(&id(1))?
                    .contents
                    .unwrap()
                    .cards
                    .iter()
                    .find(|c| c.card == id(4))
                    .unwrap()
                    .tags
                    .contains(&id(7))
            );
            ctx.resolve_tag_deletion(&id(1), &id(7), true)?;
            assert!(!ctx.tag_snapshot(&id(1), &id(7))?.deletion_conflict);
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}
async fn archive(storage: &Storage) -> String {
    storage
        .export_snapshot(id(1), 42, LIMITS)
        .await
        .unwrap()
        .encode(LIMITS)
        .unwrap()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
async fn restore(storage: &Storage, archive: String) -> StorageResult<RestoredSnapshot> {
    storage
        .transact(move |ctx| {
            let expected = ctx.recovery_state(&id(1))?.version;
            Ok(ctx.restore_snapshot(
                &RestoreSnapshot {
                    space: id(1),
                    archive,
                    expected,
                },
                50,
            )?)
        })
        .await
}
async fn edit(storage: &Storage) {
    storage
        .transact(|ctx| {
            ctx.finish_pending_text(&id(1), &id(4))?;
            ctx.rename_space(&id(1), "Позже")?;
            let object = crate::cards::object(ctx.read().document(&id(1))?, &id(4))?;
            ctx.edit(&id(1), |tx| {
                tx.put(&object, "description", "Поздний текст")?;
                tx.put(&object, "completed", true)?;
                Ok(())
            })?;
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn restore_is_new_causal_action_with_pause_full_safety_reverse_and_receipts_preserved() {
    let (root, storage, _) = fixture().await;
    let original = archive(&storage).await;
    edit(&storage).await;
    let prior = storage
        .read(|ctx| Ok(ctx.document(&id(1))?.get_heads()))
        .await
        .unwrap();
    let receipt = storage
        .read(|ctx| ctx.recorded(b"private-key"))
        .await
        .unwrap();
    let first = restore(&storage, original).await.unwrap();
    let result = storage
        .transact(|ctx| {
            let state = ctx.recovery_state(&id(1))?;
            assert!(state.paused);
            let space = ctx.space_snapshot(&id(1))?;
            assert_eq!(space.names, ["ПК", "Телефон"]);
            let card = space
                .contents
                .unwrap()
                .cards
                .into_iter()
                .find(|c| c.card == id(4))
                .unwrap();
            assert_eq!(card.descriptions, ["Незавершённая история"]);
            assert_eq!(card.completed, [false]);
            assert_eq!(card.tags, [id(7)]);
            Ok(ctx
                .read_safety_backup(&id(1), &state.backup.unwrap().id)?
                .archive)
        })
        .await
        .unwrap();
    assert_eq!(
        storage
            .read(|ctx| ctx.recorded(b"private-key"))
            .await
            .unwrap(),
        receipt
    );
    storage
        .read(move |ctx| {
            let graph = ctx.document(&id(1))?;
            for hash in prior {
                assert!(graph.get_change_by_hash(&hash).is_some());
            }
            Ok(())
        })
        .await
        .unwrap();
    let second = restore(&storage, result).await.unwrap();
    assert_ne!(first.safety, second.safety);
    storage.cleanup_restore_backup(id(1)).await.unwrap();
    assert!(
        !root
            .path()
            .join("source.db.backups")
            .join(first.safety)
            .exists()
    );
    storage.shutdown().await.unwrap();
    let storage = Storage::open(root.path().join("source.db")).await.unwrap();
    storage
        .transact(|ctx| {
            assert!(ctx.recovery_state(&id(1))?.paused);
            assert_eq!(ctx.space_snapshot(&id(1))?.names, ["Позже"]);
            assert_eq!(ctx.space_snapshot(&id(2))?.names, ["OTHER-SPACE-SECRET"]);
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}
#[tokio::test]
async fn every_commit_boundary_keeps_one_graph_pause_and_backup_witness_after_restart() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (root, storage, _) = fixture().await;
        let saved = archive(&storage).await;
        edit(&storage).await;
        let result = storage
            .transact_checked(
                move |ctx| {
                    let expected = ctx.recovery_state(&id(1))?.version;
                    Ok(ctx.restore_snapshot(
                        &RestoreSnapshot {
                            space: id(1),
                            archive: saved,
                            expected,
                        },
                        50,
                    )?)
                },
                move |at| {
                    if at == stage {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await;
        assert_eq!(
            result.unwrap_err(),
            if stage == Stage::AfterCommit {
                StorageError::OutcomeUnknown
            } else {
                StorageError::Unavailable
            }
        );
        storage.shutdown().await.unwrap();
        let storage = Storage::open(root.path().join("source.db")).await.unwrap();
        storage
            .transact(move |ctx| {
                let state = ctx.recovery_state(&id(1))?;
                assert_eq!(state.backup.is_some(), stage == Stage::AfterCommit);
                let names = ctx.space_snapshot(&id(1))?.names;
                assert_eq!(names == ["ПК", "Телефон"], stage == Stage::AfterCommit);
                Ok(())
            })
            .await
            .unwrap();
        storage.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn missing_folder_corruption_and_failed_witness_cannot_replace_previous_successful_safety() {
    let (root, storage, _) = fixture().await;
    let saved = archive(&storage).await;
    edit(&storage).await;
    let first = restore(&storage, saved.clone()).await.unwrap();
    let folder = root.path().join("source.db.backups");
    let path = folder.join(&first.safety).join(ARCHIVE);
    let original = fs::read(&path).unwrap();
    let unavailable = root.path().join("temporarily-unavailable-backups");
    fs::rename(&folder, &unavailable).unwrap();
    assert!(restore(&storage, saved.clone()).await.is_err());
    assert!(!folder.exists());
    fs::rename(&unavailable, &folder).unwrap();
    fs::write(&path, b"damaged").unwrap();
    assert!(
        storage
            .transact(move |ctx| Ok(ctx.read_safety_backup(&id(1), &first.safety)?))
            .await
            .is_err()
    );
    fs::write(&path, &original).unwrap();
    storage.run(|connection| { connection.execute_batch("CREATE TRIGGER fail_restore_witness BEFORE UPDATE ON command_results WHEN NEW.request = CAST('space-restore-backup.v1' AS BLOB) BEGIN SELECT RAISE(ABORT, 'blocked'); END")?; Ok(()) }).await.unwrap();
    assert!(restore(&storage, saved).await.is_err());
    assert_eq!(fs::read(path).unwrap(), original);
    storage
        .run(|connection| {
            connection.execute_batch("DROP TRIGGER fail_restore_witness")?;
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}
#[tokio::test]
async fn independent_peer_changes_merge_after_explicit_resume_without_losing_history_or_conflicts()
{
    let (_root, storage, _) = fixture().await;
    let saved = archive(&storage).await;
    let mut peer = storage
        .read(|ctx| Ok(ctx.document(&id(1))?.fork()))
        .await
        .unwrap();
    let peer_base = peer.get_heads();
    let object = crate::cards::object(&peer, &id(4)).unwrap();
    let mut tx = peer.transaction();
    tx.put(&object, "title", "Независимый телефон").unwrap();
    tx.put(&object, "completed", true).unwrap();
    tx.commit();
    edit(&storage).await;
    restore(&storage, saved).await.unwrap();
    let remote = peer.get_changes(&peer_base);
    storage
        .transact(move |ctx| {
            let policy = ctx.read().space_sync_control(&id(1))?;
            ctx.set_space_sync_paused(&id(1), false, policy.version)?;
            ctx.apply_changes(&id(1), remote)?;
            let graph = ctx.read().document(&id(1))?;
            let object = crate::cards::object(graph, &id(4))?;
            assert!(
                graph
                    .get_all(&object, "title")?
                    .iter()
                    .any(|(v, _)| v.as_str() == Some("Независимый телефон"))
            );
            assert!(
                graph
                    .get_all(&object, "completed")?
                    .iter()
                    .any(|(v, _)| v.as_bool() == Some(true))
            );
            assert!(crate::cards::history(graph, &object)?.len() >= 2);
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}
