use zedone_core::access_commands::AccessStore;

use super::tests::{NOW, TestClock, check, denied, setup};
use super::*;
use crate::Storage;
use automerge::{ROOT, ReadDoc, transaction::Transactable};
use std::sync::Arc;

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
