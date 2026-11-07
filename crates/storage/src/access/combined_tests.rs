use zedone_core::access_commands::AccessStore;

use super::tests::{NOW, TestClock, check, denied, setup};
use super::*;
use crate::{
    Storage,
    migration_backup::tests::LIMITS,
    startup::{MigrationOptions, Startup},
};
use automerge::{ROOT, ReadDoc, transaction::Transactable};
use std::{fs, sync::Arc};
use tempfile::TempDir;

fn combined() -> GrantScope {
    GrantScope::AdministrationAndSpaces(BTreeMap::from([("a".into(), SpacePermission::ReadWrite)]))
}

async fn active(storage: &Storage, secret: Vec<u8>, now: u64) -> StorageResult<String> {
    storage
        .transact(move |ctx| {
            Ok(with_authorized(
                ctx,
                &Credential(&secret),
                &[RequiredAccess::ActiveGrant],
                &TestClock(now),
                |_, subject| Ok(subject.into()),
            )?)
        })
        .await
}

#[tokio::test]
async fn combined_grant_survives_restart_and_can_remove_data_without_losing_final_admin() {
    let (dir, storage, admin, member) = setup().await;
    let replacement = Arc::new(AccessKey::generate().unwrap());
    let (a, r) = (admin.clone(), replacement.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(&a.credential(), &NOW, a.subject(), &r, &combined(), None)?;
            assert!(matches!(
                ctx.resolve_access(&r.credential())?.unwrap().grant.scope,
                GrantScope::AdministrationAndSpaces(_)
            ));
            let snapshot = ctx.access_snapshot(a.subject())?.unwrap();
            assert_eq!(
                serde_json::to_value(snapshot.scope).unwrap(),
                serde_json::json!({"kind":"administration_and_spaces", "spaces":{"a":"read_write"}})
            );
            Ok(())
        })
        .await
        .unwrap();
    denied(active(&storage, admin.credential().0.to_vec(), 100).await);
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let secret = || replacement.credential().0.to_vec();
    assert_eq!(
        check(&storage, secret(), "a", SpacePermission::ReadWrite, 100)
            .await
            .unwrap(),
        admin.subject()
    );
    denied(check(&storage, secret(), "b", SpacePermission::Read, 100).await);
    active(&storage, member.credential().0.to_vec(), 100)
        .await
        .unwrap();
    let (a, r) = (admin.clone(), replacement.clone());
    assert!(
        storage
            .transact(move |ctx| ctx.replace_access(
                &r.credential(),
                &NOW,
                a.subject(),
                &AccessKey::generate()?,
                &GrantScope::Spaces(BTreeMap::from([("a".into(), SpacePermission::ReadWrite)])),
                None
            ))
            .await
            .is_err()
    );
    let pure = Arc::new(AccessKey::generate().unwrap());
    let (a, r, p) = (admin.clone(), replacement.clone(), pure.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(
                &r.credential(),
                &NOW,
                a.subject(),
                &p,
                &GrantScope::Administration,
                None,
            )
        })
        .await
        .unwrap();
    denied(active(&storage, secret(), 100).await);
    denied(
        check(
            &storage,
            pure.credential().0.to_vec(),
            "a",
            SpacePermission::Read,
            100,
        )
        .await,
    );
    let (a, p) = (admin.clone(), pure.clone());
    storage
        .transact(move |ctx| {
            assert!(
                ctx.inspect_access(&p.credential(), &NOW, a.subject())?
                    .unwrap()
                    .scope
                    .administration()
            );
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    Storage::open(dir.path().join("device.db"))
        .await
        .unwrap()
        .shutdown()
        .await
        .unwrap();
}

#[tokio::test]
async fn combined_expiry_revocation_and_regrant_use_the_existing_subject_and_tombstones() {
    let (_dir, storage, admin, member) = setup().await;
    let replacement = Arc::new(AccessKey::generate().unwrap());
    let (a, m, r) = (admin.clone(), member.clone(), replacement.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(
                &a.credential(),
                &NOW,
                m.subject(),
                &r,
                &combined(),
                Some(UNIX_EPOCH + Duration::from_millis(200)),
            )
        })
        .await
        .unwrap();
    let secret = || replacement.credential().0.to_vec();
    active(&storage, secret(), 199).await.unwrap();
    denied(active(&storage, secret(), 200).await);
    denied(check(&storage, secret(), "a", SpacePermission::ReadWrite, 200).await);
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| ctx.revoke_access(&a.credential(), &NOW, m.subject()))
        .await
        .unwrap();
    denied(active(&storage, secret(), 100).await);
    let (a, m, r) = (admin.clone(), member.clone(), replacement.clone());
    assert!(
        storage
            .transact(move |ctx| ctx.replace_access(
                &a.credential(),
                &NOW,
                m.subject(),
                &r,
                &combined(),
                None
            ))
            .await
            .is_err(),
        "old keys cannot revive"
    );
    let fresh = Arc::new(AccessKey::generate().unwrap());
    let (a, m, f) = (admin.clone(), member.clone(), fresh.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(&a.credential(), &NOW, m.subject(), &f, &combined(), None)
        })
        .await
        .unwrap();
    assert_eq!(
        active(&storage, fresh.credential().0.to_vec(), 100)
            .await
            .unwrap(),
        member.subject()
    );
    denied(active(&storage, member.credential().0.to_vec(), 100).await);
    denied(active(&storage, secret(), 100).await);
}

#[tokio::test]
async fn invalid_or_failed_combined_issuance_cannot_commit_partial_data_rights_or_receipts() {
    let (_dir, storage, admin, _member) = setup().await;
    for spaces in [
        BTreeMap::new(),
        BTreeMap::from([("".into(), SpacePermission::ReadWrite)]),
        BTreeMap::from([("missing".into(), SpacePermission::ReadWrite)]),
    ] {
        let a = admin.clone();
        let pending = Arc::new(PendingAccess::generate().unwrap());
        let p = pending.clone();
        assert!(
            storage
                .transact(move |ctx| {
                    // A swallowed failed mutator still poisons the whole transaction.
                    let _ = ctx.issue_access(
                        &a.credential(),
                        &NOW,
                        &p,
                        &GrantScope::AdministrationAndSpaces(spaces),
                        None,
                    );
                    ctx.edit("a", |doc| {
                        doc.put(ROOT, "partial", true)?;
                        Ok(())
                    })?;
                    ctx.record(
                        b"must-not-commit",
                        &RecordedCommand {
                            request: vec![1],
                            result: vec![2],
                        },
                    )
                })
                .await
                .is_err()
        );
        let p = pending.clone();
        storage
            .transact(move |ctx| {
                assert!(ctx.resolve_access(&p.credential())?.is_none());
                assert!(ctx.read().document("a")?.get(ROOT, "partial")?.is_none());
                assert!(ctx.read().recorded(b"must-not-commit")?.is_none());
                Ok(())
            })
            .await
            .unwrap();
    }
    let (a, pending) = (admin.clone(), Arc::new(PendingAccess::generate().unwrap()));
    let p = pending.clone();
    assert!(
        storage
            .transact(move |ctx| -> StorageResult<()> {
                ctx.issue_access(&a.credential(), &NOW, &p, &combined(), None)?;
                ctx.edit("a", |doc| {
                    doc.put(ROOT, "partial", true)?;
                    Ok(())
                })?;
                ctx.record(
                    b"must-not-commit",
                    &RecordedCommand {
                        request: vec![1],
                        result: vec![2],
                    },
                )?;
                Err(StorageError::Unavailable)
            })
            .await
            .is_err()
    );
    storage
        .transact(move |ctx| {
            assert!(ctx.resolve_access(&pending.credential())?.is_none());
            assert!(ctx.read().document("a")?.get(ROOT, "partial")?.is_none());
            assert!(ctx.read().recorded(b"must-not-commit")?.is_none());
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn v3_to_v4_is_backed_up_preserves_state_and_does_not_expand_any_grant() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("device.db");
    let mut startup = Startup::new(&path);
    startup.registry = crate::schema::Registry(&crate::schema::REGISTRY.0[..3]);
    let storage = startup.automatic(None).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    let before = storage
        .timed_transact(move |ctx, clock| {
            ctx.create_document("a")?;
            ctx.bootstrap_access(&a)?;
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Spaces(BTreeMap::from([("a".into(), SpacePermission::Read)])),
                None,
            )?;
            ctx.edit("a", |doc| {
                doc.put(ROOT, "title", "История 空間")?;
                Ok(())
            })?;
            ctx.set_peer_state("a", b"peer", &automerge::sync::State::new())?;
            ctx.record(
                b"old-receipt",
                &RecordedCommand {
                    request: vec![1],
                    result: vec![2],
                },
            )?;
            Ok((
                ctx.access_snapshot(a.subject())?.unwrap(),
                ctx.access_snapshot(m.subject())?.unwrap(),
                ctx.read().document("a")?.get_actor().to_bytes().to_vec(),
            ))
        })
        .await
        .unwrap();
    let a = admin.clone();
    assert!(
        storage
            .transact(move |ctx| ctx.issue_access(
                &a.credential(),
                &NOW,
                &PendingAccess::generate()?,
                &combined(),
                None
            ))
            .await
            .is_err(),
        "legacy runtime cannot persist the new semantics"
    );
    storage.shutdown().await.unwrap();
    let original = fs::read(&path).unwrap();
    assert_eq!(
        Storage::open(&path).await.err(),
        Some(StorageError::Unavailable)
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    let options = MigrationOptions {
        folder: dir.path().to_owned(),
        created_at_unix_ms: 10,
        limits: LIMITS,
    };
    let storage = Startup::new(&path).automatic(Some(options)).await.unwrap();
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            assert_eq!(ctx.access_snapshot(a.subject())?.unwrap(), before.0);
            assert_eq!(ctx.access_snapshot(m.subject())?.unwrap(), before.1);
            assert_eq!(ctx.read().document("a")?.get_actor().to_bytes(), before.2);
            assert_eq!(
                ctx.read()
                    .document("a")?
                    .get(ROOT, "title")?
                    .unwrap()
                    .0
                    .as_str(),
                Some("История 空間")
            );
            assert_eq!(
                ctx.read().recorded(b"old-receipt")?.unwrap().result,
                vec![2]
            );
            assert!(ctx.read().peer_state("a", b"peer")?.shared_heads.is_empty());
            assert!(
                ctx.resolve_access(&m.credential())?
                    .unwrap()
                    .grant
                    .scope
                    .spaces()
                    .unwrap()
                    .len()
                    == 1
            );
            Ok(())
        })
        .await
        .unwrap();
    denied(
        check(
            &storage,
            admin.credential().0.to_vec(),
            "a",
            SpacePermission::Read,
            100,
        )
        .await,
    );
    denied(
        check(
            &storage,
            member.credential().0.to_vec(),
            "a",
            SpacePermission::ReadWrite,
            100,
        )
        .await,
    );
    storage.shutdown().await.unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        crate::schema::REGISTRY.current().version
    );
    let candidates: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.join("manifest.json").is_file())
        .collect();
    assert_eq!(candidates.len(), 1);
    let backup = rusqlite::Connection::open(candidates[0].join("database.sqlite")).unwrap();
    assert_eq!(
        backup
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        3
    );
    assert_eq!(
        backup
            .query_row("SELECT count(*) FROM access_spaces", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn v4_does_not_reinterpret_corrupt_v3_as_legitimate_combined_access() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("device.db");
    let mut startup = Startup::new(&path);
    startup.registry = crate::schema::Registry(&crate::schema::REGISTRY.0[..3]);
    let storage = startup.automatic(None).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    storage
        .transact(move |ctx| {
            ctx.create_document("a")?;
            ctx.bootstrap_access(&a)
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.execute(
        "INSERT INTO access_spaces VALUES (?1, 'a', 1)",
        [admin.subject()],
    )
    .unwrap();
    drop(conn);
    let before = fs::read(&path).unwrap();
    assert_eq!(
        Startup::new(&path)
            .automatic(Some(MigrationOptions {
                folder: dir.path().to_owned(),
                created_at_unix_ms: 10,
                limits: LIMITS
            }))
            .await
            .err(),
        Some(StorageError::InvalidDatabase)
    );
    assert_eq!(fs::read(&path).unwrap(), before);
}
