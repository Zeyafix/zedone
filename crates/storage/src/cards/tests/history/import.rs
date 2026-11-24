use super::*;
use crate::{
    cards,
    snapshot::{SnapshotLimits, SpaceSnapshot},
};
use automerge::{ROOT, ReadDoc};
use std::collections::{BTreeMap, BTreeSet};
use zedone_core::{
    spaces::{SpaceSnapshot as Space, SpaceStore},
    tags::TagStore,
};

const LIMITS: SnapshotLimits = SnapshotLimits {
    max_bytes: 1024 * 1024,
    max_changes: 1000,
};

fn load(bytes: &[u8]) -> Automerge {
    let snapshot = SpaceSnapshot::decode(bytes, LIMITS).unwrap();
    let mut graph = Automerge::new();
    graph
        .apply_changes(snapshot.changes().iter().cloned())
        .unwrap();
    graph
}

async fn fixture() -> (TempDir, Storage, Arc<PendingAccess>, Vec<u8>) {
    let (dir, storage, admin) = setup().await;
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
            let tag = ctx.create_tag(&zedone_core::tags::CreateTag {
                space: sid(1),
                tag: sid(4),
                name: "Work".into(),
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
            Ok(())
        })
        .await
        .unwrap();
    let card = fresh(&storage, &admin, 4, 9).await;
    run!(storage, admin, begin_editing, begin(5, card.version)).unwrap();
    at!(
        storage,
        admin,
        1000,
        autosave,
        save(
            6,
            5,
            &sid(1),
            &format!("{{\"column\":\"{}\"}}", sid(2)),
            card.version
        )
    )
    .unwrap();
    let bytes = storage
        .export_snapshot(sid(1), 42, LIMITS)
        .await
        .unwrap()
        .encode(LIMITS)
        .unwrap();
    (dir, storage, admin, bytes)
}

async fn raw(storage: &Storage, space: String) -> Vec<u8> {
    storage
        .snapshot(space, 42, LIMITS)
        .await
        .unwrap()
        .encode(LIMITS)
        .unwrap()
}

async fn import(storage: &Storage, bytes: &[u8]) -> StorageResult<Space> {
    let bytes = bytes.to_vec();
    storage
        .transact(move |ctx| ctx.import_snapshot(&bytes, LIMITS))
        .await
}

async fn counts(storage: &Storage) -> Vec<i64> {
    storage
        .read(|ctx| {
            [
                "documents",
                "access_principals",
                "access_keys",
                "access_spaces",
                "peers",
                "sync_spaces",
                "sync_selections",
                "command_results",
                "card_editing",
            ]
            .iter()
            .map(|table| {
                ctx.connection()
                    .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                    .map_err(StorageError::from)
            })
            .collect()
        })
        .await
        .unwrap()
}

fn mappings(
    original: &Automerge,
    copied: &Automerge,
    old_space: &str,
    new_space: &str,
) -> BTreeMap<String, String> {
    let mut result = BTreeMap::from([(new_space.into(), old_space.into())]);
    let keys = |graph: &Automerge| {
        graph
            .keys(ROOT)
            .filter_map(|key| {
                key.split_once(':')
                    .map(|(kind, id)| (kind.to_owned(), id.to_owned()))
            })
            .collect::<Vec<_>>()
    };
    let old = keys(original);
    let new = keys(copied);
    assert_eq!(old.len(), new.len());
    for ((old_kind, old_id), (new_kind, new_id)) in old.into_iter().zip(new) {
        assert_eq!(old_kind, new_kind);
        assert_ne!(old_id, new_id);
        result.insert(new_id, old_id);
    }
    result
}

fn normalized(mut value: serde_json::Value, ids: &BTreeMap<String, String>) -> serde_json::Value {
    fn visit(value: &mut serde_json::Value, key: &str, ids: &BTreeMap<String, String>) {
        match value {
            serde_json::Value::Object(fields) => {
                fields.remove("version");
                for (field, value) in fields {
                    visit(value, field, ids);
                }
            }
            serde_json::Value::Array(values) => {
                for value in values {
                    visit(value, key, ids);
                }
            }
            serde_json::Value::String(text) => {
                if matches!(
                    key,
                    "space" | "card" | "column" | "tag" | "columns" | "tags" | "name_group" | "id"
                ) && !text.is_empty()
                {
                    *text = ids
                        .get(text)
                        .unwrap_or_else(|| panic!("Unmapped structural ID {text}"))
                        .clone();
                } else if key == "deletion_groups" && text.starts_with("trash:") {
                    *text = text.split(':').take(2).collect::<Vec<_>>().join(":");
                }
            }
            _ => {}
        }
    }
    visit(&mut value, "", ids);
    value
}

fn equivalent(bytes: &[u8], copied: &Automerge, copy: &Space) {
    let source = SpaceSnapshot::decode(bytes, LIMITS).unwrap();
    let graph = load(bytes);
    let old = crate::spaces::snapshot(&graph, &source.metadata().space, true).unwrap();
    let ids = mappings(&graph, copied, &old.space, &copy.space);
    let identity = ids.values().map(|id| (id.clone(), id.clone())).collect();
    assert_eq!(
        normalized(serde_json::to_value(copy).unwrap(), &ids),
        normalized(serde_json::to_value(&old).unwrap(), &identity)
    );
    for (new, old) in ids
        .iter()
        .filter(|(_, old)| graph.get(ROOT, format!("card:{old}")).unwrap().is_some())
    {
        let history = cards::history(&graph, &cards::object(&graph, old).unwrap()).unwrap();
        let imported = cards::history(copied, &cards::object(copied, new).unwrap()).unwrap();
        assert_eq!(history.len(), imported.len());
        for (before, after) in history.iter().zip(imported) {
            assert_eq!(
                (
                    &before.subject,
                    before.sequence,
                    &before.title,
                    &before.description
                ),
                (
                    &after.subject,
                    after.sequence,
                    &after.title,
                    &after.description
                )
            );
            assert_ne!(before.id, after.id);
        }
    }
    let actors = |graph: &Automerge| {
        graph
            .get_changes(&[])
            .iter()
            .map(|c| c.actor_id().clone())
            .collect::<BTreeSet<_>>()
    };
    assert!(actors(&graph).is_disjoint(&actors(copied)));
    assert_eq!(graph.get_changes(&[]).len(), copied.get_changes(&[]).len());
    assert!(copied.get_missing_deps(&[]).is_empty());
    assert!(!actors(copied).contains(copied.get_actor()));
}

#[tokio::test]
async fn independent_import_keeps_every_field_history_and_trash_without_transferring_access() {
    let (dir, storage, admin, bytes) = fixture().await;
    let before = raw(&storage, sid(1)).await;
    let service = counts(&storage).await;
    let one = import(&storage, &bytes).await.unwrap();
    let two = import(&storage, &bytes).await.unwrap();
    assert_ne!(one.space, two.space);
    assert_ne!(one.space, sid(1));
    let mut expected = service;
    expected[0] += 2;
    assert_eq!(counts(&storage).await, expected);
    assert_eq!(raw(&storage, sid(1)).await, before);
    let two_bytes = raw(&storage, two.space.clone()).await;
    let space = one.space.clone();
    let graph = storage
        .read(move |ctx| Ok(ctx.document(&space)?.clone()))
        .await
        .unwrap();
    equivalent(&bytes, &graph, &one);
    let second = load(&two_bytes);
    let one_keys: BTreeSet<_> = graph.keys(ROOT).filter(|key| key.contains(':')).collect();
    assert!(
        second
            .keys(ROOT)
            .filter(|key| key.contains(':'))
            .all(|key| !one_keys.contains(&key))
    );
    let new_space = one.space.clone();
    let key = admin.credential().0.to_vec();
    rejected(
        storage
            .transact(move |ctx| {
                Ok(spaces::read(
                    ctx,
                    &Credential(&key),
                    &Now,
                    &req(40, zedone_core::spaces::ReadSpace { space: new_space }),
                )?)
            })
            .await,
        ErrorCode::AccessDenied,
    );
    let subject = admin.subject().to_owned();
    let space = one.space.clone();
    storage
        .transact(move |ctx| {
            ctx.connection().execute(
                "INSERT INTO access_spaces VALUES (?1, ?2, 1)",
                (&subject, &space),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let card = one
        .contents
        .as_ref()
        .unwrap()
        .cards
        .iter()
        .find(|c| c.titles == [sid(1)])
        .unwrap()
        .clone();
    let key = admin.credential().0.to_vec();
    let trash = storage
        .transact(move |ctx| {
            Ok(core::trash(
                ctx,
                &Credential(&key),
                &Now,
                &req(
                    41,
                    TrashCard {
                        space: card.space,
                        card: card.card,
                        expected: card.version,
                    },
                ),
            )?)
        })
        .await
        .unwrap()
        .result;
    let key = admin.credential().0.to_vec();
    let restored = storage
        .transact(move |ctx| {
            Ok(core::restore(
                ctx,
                &Credential(&key),
                &Now,
                &req(
                    42,
                    RestoreCard {
                        space: trash.space,
                        card: trash.card,
                        column: None,
                        expected_column: None,
                        expected: trash.version,
                    },
                ),
            )?)
        })
        .await
        .unwrap()
        .result;
    assert_eq!(restored.titles, [sid(1)]);
    assert_eq!(raw(&storage, sid(1)).await, before);
    assert_eq!(raw(&storage, two.space).await, two_bytes);
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert_eq!(raw(&storage, sid(1)).await, before);
    let persisted = one.space;
    let copied = storage
        .transact(move |ctx| Ok(ctx.space_snapshot(&persisted)?))
        .await
        .unwrap();
    assert_eq!(
        copied
            .contents
            .unwrap()
            .cards
            .iter()
            .find(|c| c.card == restored.card)
            .unwrap()
            .titles,
        restored.titles
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn import_preserves_concurrent_fields_positions_tag_groups_and_causal_deletions() {
    let (_dir, storage, admin, _) = fixture().await;
    let current = fresh(&storage, &admin, 50, 9).await;
    at!(
        storage,
        admin,
        1001,
        finish_editing,
        finish(51, 5, current.version)
    )
    .unwrap();
    storage
        .transact(move |ctx| {
            let first = crate::columns::object(ctx.read().document(&sid(1))?, &sid(2))?;
            let second = crate::columns::object(ctx.read().document(&sid(1))?, &sid(3))?;
            ctx.edit(&sid(1), |tx| {
                for (column, token) in [(&first, 8), (&second, 9)] {
                    crate::ordering::put_rank(
                        tx,
                        column,
                        &zedone_core::ordering::Rank::Path(vec![zedone_core::ordering::RankPart {
                            digit: 7,
                            id: sid(token),
                        }]),
                    )?;
                }
                Ok(())
            })?;
            let base = ctx.read().document(&sid(1))?.clone();
            let mut changes = Vec::new();
            for (index, label) in [(0, "ПК"), (1, "Телефон")] {
                let mut peer = base.fork();
                let card = cards::object(&peer, &sid(9))?;
                let column = crate::columns::object(&peer, &sid(2))?;
                let tag = peer.get(ROOT, format!("tag:{}", sid(4)))?.unwrap().1;
                let mut tx = peer.transaction();
                tx.put(ROOT, "name", label)?;
                tx.put(&card, "title", label)?;
                tx.put(&card, "description", format!("{label} {}", sid(1)))?;
                tx.delete(&card, "completed")?;
                tx.put(&card, "completed", index == 0)?;
                tx.put(&column, "name", label)?;
                let rank =
                    zedone_core::ordering::Rank::Path(vec![zedone_core::ordering::RankPart {
                        digit: index,
                        id: sid(9),
                    }]);
                tx.put(
                    &card,
                    "placement",
                    serde_json::to_string(&zedone_core::ordering::CardPlacement {
                        column: sid(2 + index as u8),
                        rank,
                    })
                    .unwrap(),
                )?;
                tx.delete(&card, "lifecycle")?;
                tx.put(&card, "lifecycle", "active")?;
                if index == 0 {
                    tx.put(ROOT, "purge_boundary", true)?;
                }
                tx.put(&tag, "name", label)?;
                let alias = tx.put_object(
                    ROOT,
                    format!("tag:{}", sid(6 + index as u8)),
                    automerge::ObjType::Map,
                )?;
                tx.put(&alias, "format", "zedone.tag.v1")?;
                tx.put(&alias, "name", if index == 0 { "Shared" } else { "SHARED" })?;
                tx.put(&alias, "deleted", false)?;
                let assignments = cards::map(&base, &card, "tags")?;
                tx.put(&assignments, sid(6 + index as u8), true)?;
                tx.commit();
                changes.extend(peer.get_changes(&base.get_heads()));
            }
            ctx.apply_changes(&sid(1), changes)?;
            Ok(())
        })
        .await
        .unwrap();
    let bytes = storage
        .export_snapshot(sid(1), 42, LIMITS)
        .await
        .unwrap()
        .encode(LIMITS)
        .unwrap();
    let imported = import(&storage, &bytes).await.unwrap();
    let copied = load(&raw(&storage, imported.space.clone()).await);
    equivalent(&bytes, &copied, &imported);
    assert_eq!(imported.names.len(), 2);
    let card = imported
        .contents
        .unwrap()
        .cards
        .into_iter()
        .find(|c| c.titles.len() == 2)
        .unwrap();
    assert_eq!(card.placements.len(), 2);
    assert_eq!(card.completed, [false, true]);
    let original = load(&bytes);
    let winner = original.get(ROOT, "name").unwrap().unwrap().0.into_owned();
    assert_eq!(
        copied.get(ROOT, "name").unwrap().unwrap().0.into_owned(),
        winner
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_incomplete_and_historically_unsupported_archives_never_create_partial_spaces() {
    for scenario in 0..7 {
        let (_dir, storage, _admin, bytes) = fixture().await;
        storage
            .transact(move |ctx| {
                match scenario {
                    0 => {
                        ctx.edit(&sid(1), |tx| {
                            tx.put(ROOT, "future-field", "hidden payload")?;
                            Ok(())
                        })?;
                        ctx.edit(&sid(1), |tx| {
                            tx.delete(ROOT, "future-field")?;
                            Ok(())
                        })?;
                    }
                    1 => {
                        let card = cards::object(ctx.read().document(&sid(1))?, &sid(9))?;
                        let tags = cards::map(ctx.read().document(&sid(1))?, &card, "tags")?;
                        ctx.edit(&sid(1), |tx| {
                            tx.put(&tags, sid(99), false)?;
                            Ok(())
                        })?;
                        ctx.edit(&sid(1), |tx| {
                            tx.delete(&tags, sid(99))?;
                            Ok(())
                        })?;
                    }
                    2 => {
                        let base = ctx.read().document(&sid(1))?.clone();
                        let mut peer = base.fork();
                        let mut tx = peer.transaction();
                        tx.put(ROOT, "name", "missing")?;
                        tx.commit();
                        let mut tx = peer.transaction();
                        tx.put(ROOT, "name", "orphan")?;
                        let (hash, _) = tx.commit();
                        ctx.apply_changes(
                            &sid(1),
                            vec![peer.get_change_by_hash(&hash.unwrap()).unwrap()],
                        )?;
                    }
                    3 => {
                        let card = cards::object(ctx.read().document(&sid(1))?, &sid(9))?;
                        ctx.edit(&sid(1), |tx| {
                            tx.put(&card, "completed", "invalid old type")?;
                            Ok(())
                        })?;
                        ctx.edit(&sid(1), |tx| {
                            tx.put(&card, "completed", false)?;
                            Ok(())
                        })?;
                    }
                    5 => {
                        let card = cards::object(ctx.read().document(&sid(1))?, &sid(9))?;
                        ctx.edit(&sid(1), |tx| {
                            tx.put(&card, "position", -1_i64)?;
                            Ok(())
                        })?;
                        ctx.edit(&sid(1), |tx| {
                            tx.put(&card, "position", 0_u64)?;
                            Ok(())
                        })?;
                    }
                    6 => {
                        let graph = ctx.read().document(&sid(1))?;
                        let card = cards::object(graph, &sid(9))?;
                        let history = cards::map(graph, &card, "history")?;
                        let version = graph
                            .get(&history, graph.keys(&history).next().unwrap())?
                            .unwrap()
                            .1;
                        ctx.edit(&sid(1), |tx| {
                            tx.put(&version, "sequence", 0.5_f64)?;
                            Ok(())
                        })?;
                        ctx.edit(&sid(1), |tx| {
                            tx.put(&version, "sequence", 0_u64)?;
                            Ok(())
                        })?;
                    }
                    _ => {}
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut invalid_bytes = raw(&storage, sid(1)).await;
        if scenario == 4 {
            invalid_bytes[0] ^= 1;
        }
        let original = invalid_bytes.clone();
        let before = counts(&storage).await;
        let source = raw(&storage, sid(1)).await;
        rejected(
            import(&storage, &invalid_bytes).await,
            ErrorCode::InvalidRequest,
        );
        assert_eq!(invalid_bytes, original);
        assert_eq!(counts(&storage).await, before);
        assert_eq!(raw(&storage, sid(1)).await, source);
        if scenario == 4 {
            let request = bytes.clone();
            rejected(
                storage
                    .transact(move |ctx| {
                        ctx.import_snapshot(
                            &request,
                            SnapshotLimits {
                                max_bytes: 1,
                                ..LIMITS
                            },
                        )
                    })
                    .await,
                ErrorCode::InvalidRequest,
            );
            let request = bytes;
            rejected(
                storage
                    .transact(move |ctx| {
                        ctx.import_snapshot(
                            &request,
                            SnapshotLimits {
                                max_changes: 0,
                                ..LIMITS
                            },
                        )
                    })
                    .await,
                ErrorCode::InvalidRequest,
            );
            assert_eq!(counts(&storage).await, before);
        }
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn import_commits_all_or_nothing_and_caught_failures_poison_the_transaction() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (dir, storage, _admin, bytes) = fixture().await;
        let before = raw(&storage, sid(1)).await;
        let initial = counts(&storage).await;
        let result = storage
            .transact_checked(
                move |ctx| ctx.import_snapshot(&bytes, LIMITS),
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
        let storage = Storage::open(dir.path().join("cards.db")).await.unwrap();
        let mut expected = initial;
        if stage == Stage::AfterCommit {
            expected[0] += 1;
        }
        assert_eq!(counts(&storage).await, expected);
        assert_eq!(raw(&storage, sid(1)).await, before);
        storage.shutdown().await.unwrap();
    }
    let (_dir, storage, _admin, _) = fixture().await;
    let before = counts(&storage).await;
    assert_eq!(
        storage
            .transact(|ctx| {
                assert!(ctx.import_snapshot(&[], LIMITS).is_err());
                ctx.create_document(&sid(77))?;
                Ok(())
            })
            .await
            .unwrap_err(),
        StorageError::Unavailable
    );
    assert_eq!(counts(&storage).await, before);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_real_sql_failure_rolls_back_the_document_and_never_copies_deleted_tag_access() {
    let (_dir, storage, _admin, bytes) = fixture().await;
    let before = counts(&storage).await;
    let source = raw(&storage, sid(1)).await;
    storage.run(|connection| {
        connection.execute_batch("CREATE TEMP TRIGGER reject_import BEFORE INSERT ON changes WHEN NEW.space != '00000000000000000000000000000001' BEGIN SELECT RAISE(FAIL, 'import failure'); END;")?;
        Ok(())
    }).await.unwrap();
    assert_eq!(
        import(&storage, &bytes).await.unwrap_err(),
        StorageError::Unavailable
    );
    assert_eq!(counts(&storage).await, before);
    assert_eq!(raw(&storage, sid(1)).await, source);
    storage
        .run(|connection| {
            connection.execute_batch("DROP TRIGGER reject_import")?;
            Ok(())
        })
        .await
        .unwrap();
    storage
        .transact(|ctx| {
            ctx.delete_tag(&sid(1), &sid(4))?;
            Ok(())
        })
        .await
        .unwrap();
    let after_delete = raw(&storage, sid(1)).await;
    let restored = import(&storage, &bytes).await.unwrap();
    assert!(
        restored
            .contents
            .as_ref()
            .unwrap()
            .cards
            .iter()
            .any(|card| !card.tags.is_empty())
    );
    assert_eq!(raw(&storage, sid(1)).await, after_delete);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn a_snapshot_that_outgrows_the_budget_during_reencoding_is_not_published() {
    let (_dir, storage, _admin, _) = fixture().await;
    storage
        .transact(|ctx| {
            let mut peer = ctx
                .read()
                .document(&sid(1))?
                .fork()
                .with_actor(ActorId::from(vec![1]));
            let mut tx = peer.transaction();
            tx.put(ROOT, "name", "short actor")?;
            let (hash, _) = tx.commit();
            ctx.apply_changes(
                &sid(1),
                vec![peer.get_change_by_hash(&hash.unwrap()).unwrap()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let bytes = storage
        .export_snapshot(sid(1), 42, LIMITS)
        .await
        .unwrap()
        .encode(LIMITS)
        .unwrap();
    let before = counts(&storage).await;
    let source = raw(&storage, sid(1)).await;
    let request = bytes.clone();
    assert_eq!(
        storage
            .transact(move |ctx| ctx.import_snapshot(
                &request,
                SnapshotLimits {
                    max_bytes: request.len(),
                    ..LIMITS
                }
            ))
            .await
            .unwrap_err(),
        StorageError::Unavailable
    );
    assert_eq!(counts(&storage).await, before);
    assert_eq!(raw(&storage, sid(1)).await, source);
    assert!(import(&storage, &bytes).await.is_ok());
    storage.shutdown().await.unwrap();
}
