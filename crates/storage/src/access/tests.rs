use std::{fs, sync::Arc};

use automerge::{ROOT, ReadDoc, transaction::Transactable};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use utoipa::ToSchema;
use zedone_core::{
    command::Command,
    replay::{self, Execution},
};

use super::*;
use crate::{
    Storage,
    migration_backup::tests::LIMITS,
    startup::{MigrationOptions, Startup},
};

pub(super) struct TestClock(pub(super) u64);
impl Clock for TestClock {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_millis(self.0))
    }
}
pub(super) const NOW: TestClock = TestClock(100);

fn scope(permission: SpacePermission) -> GrantScope {
    GrantScope::Spaces(BTreeMap::from([("a".into(), permission)]))
}

pub(super) fn denied<T>(result: StorageResult<T>) {
    assert!(matches!(
        result,
        Err(StorageError::Rejected(CommandError {
            code: ErrorCode::AccessDenied
        }))
    ));
}

pub(super) async fn setup() -> (TempDir, Storage, Arc<PendingAccess>, Arc<PendingAccess>) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            for space in ["a", "b"] {
                ctx.create_document(space)?;
            }
            ctx.bootstrap_access(&a)?;
            ctx.issue_access(
                &a.credential(),
                &NOW,
                &m,
                &scope(SpacePermission::Read),
                None,
            )
        })
        .await
        .unwrap();
    (dir, storage, admin, member)
}

pub(super) async fn check(
    storage: &Storage,
    secret: Vec<u8>,
    space: &'static str,
    permission: SpacePermission,
    now: u64,
) -> StorageResult<String> {
    storage
        .transact(move |ctx| {
            Ok(with_authorized(
                ctx,
                &Credential(&secret),
                &[RequiredAccess::Space {
                    id: space,
                    permission,
                }],
                &TestClock(now),
                |ctx, subject| {
                    ctx.read().document(space)?;
                    Ok(subject.into())
                },
            )?)
        })
        .await
}

#[tokio::test]
async fn bootstrap_is_once_and_admin_transfer_keeps_revocation_reversible_without_owner_bypass() {
    let (dir, storage, admin, member) = setup().await;
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
    let m = member.clone();
    denied(
        storage
            .transact(move |ctx| ctx.revoke_access(&m.credential(), &NOW, "unknown"))
            .await,
    );
    let a = admin.clone();
    assert!(
        storage
            .transact(move |ctx| ctx.revoke_access(&a.credential(), &NOW, a.subject()))
            .await
            .is_err(),
        "last administrator cannot be removed before transfer"
    );
    let next = Arc::new(PendingAccess::generate().unwrap());
    let (a, n) = (admin.clone(), next.clone());
    storage
        .transact(move |ctx| {
            ctx.issue_access(&a.credential(), &NOW, &n, &GrantScope::Administration, None)?;
            ctx.revoke_access(&a.credential(), &NOW, a.subject())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let fresh = PendingAccess::generate().unwrap();
    denied(
        storage
            .transact(move |ctx| ctx.bootstrap_access(&fresh))
            .await,
    );
    let a = admin.clone();
    denied(
        storage
            .transact(move |ctx| ctx.revoke_access(&a.credential(), &NOW, "unknown"))
            .await,
    );
    let (n, m) = (next.clone(), member.clone());
    storage
        .transact(move |ctx| ctx.revoke_access(&n.credential(), &NOW, m.subject()))
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn real_credentials_enforce_read_mode_selected_spaces_expiry_and_invalid_material() {
    let (_dir, storage, admin, member) = setup().await;
    assert_eq!(
        check(
            &storage,
            member.credential().0.to_vec(),
            "a",
            SpacePermission::Read,
            100
        )
        .await
        .unwrap(),
        member.subject()
    );
    for (space, permission) in [
        ("a", SpacePermission::ReadWrite),
        ("b", SpacePermission::Read),
        ("missing", SpacePermission::Read),
    ] {
        denied(
            check(
                &storage,
                member.credential().0.to_vec(),
                space,
                permission,
                100,
            )
            .await,
        );
    }
    for secret in [vec![], vec![1; 31], vec![1; 32], vec![1; 33]] {
        denied(check(&storage, secret, "a", SpacePermission::Read, 100).await);
    }
    let a = admin.clone();
    let timed = Arc::new(PendingAccess::generate().unwrap());
    let t = timed.clone();
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &NOW,
                &t,
                &scope(SpacePermission::ReadWrite),
                Some(UNIX_EPOCH + Duration::from_millis(200)),
            )
        })
        .await
        .unwrap();
    check(
        &storage,
        timed.credential().0.to_vec(),
        "a",
        SpacePermission::ReadWrite,
        199,
    )
    .await
    .unwrap();
    for now in [200, 201] {
        denied(
            check(
                &storage,
                timed.credential().0.to_vec(),
                "a",
                SpacePermission::Read,
                now,
            )
            .await,
        );
    }
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn replacement_keeps_subject_changes_both_ways_and_never_reactivates_retired_keys() {
    let (dir, storage, admin, member) = setup().await;
    let writable = Arc::new(AccessKey::generate().unwrap());
    let (a, m, key) = (admin.clone(), member.clone(), writable.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(
                &a.credential(),
                &NOW,
                m.subject(),
                &key,
                &scope(SpacePermission::ReadWrite),
                None,
            )
        })
        .await
        .unwrap();
    denied(
        check(
            &storage,
            member.credential().0.to_vec(),
            "a",
            SpacePermission::Read,
            100,
        )
        .await,
    );
    assert_eq!(
        check(
            &storage,
            writable.credential().0.to_vec(),
            "a",
            SpacePermission::ReadWrite,
            100
        )
        .await
        .unwrap(),
        member.subject()
    );
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| ctx.revoke_access(&a.credential(), &NOW, m.subject()))
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    denied(
        check(
            &storage,
            writable.credential().0.to_vec(),
            "a",
            SpacePermission::Read,
            100,
        )
        .await,
    );
    let (a, m, key) = (admin.clone(), member.clone(), writable.clone());
    assert!(
        storage
            .transact(move |ctx| ctx.replace_access(
                &a.credential(),
                &NOW,
                m.subject(),
                &key,
                &scope(SpacePermission::ReadWrite),
                None
            ))
            .await
            .is_err()
    );
    let readonly = Arc::new(AccessKey::generate().unwrap());
    let (a, m, key) = (admin.clone(), member.clone(), readonly.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(
                &a.credential(),
                &NOW,
                m.subject(),
                &key,
                &scope(SpacePermission::Read),
                None,
            )
        })
        .await
        .unwrap();
    assert_eq!(
        check(
            &storage,
            readonly.credential().0.to_vec(),
            "a",
            SpacePermission::Read,
            100
        )
        .await
        .unwrap(),
        member.subject()
    );
    denied(
        check(
            &storage,
            readonly.credential().0.to_vec(),
            "a",
            SpacePermission::ReadWrite,
            100,
        )
        .await,
    );
    let (a, m) = (admin.clone(), member.clone());
    assert!(
        storage
            .transact(move |ctx| ctx.replace_access(
                &a.credential(),
                &NOW,
                m.subject(),
                &m.key,
                &scope(SpacePermission::ReadWrite),
                None
            ))
            .await
            .is_err(),
        "even an older key stays retired"
    );
    let (a, m) = (admin.clone(), member.clone());
    assert!(
        storage
            .transact(move |ctx| ctx.issue_access(
                &a.credential(),
                &NOW,
                &m,
                &GrantScope::Administration,
                None
            ))
            .await
            .is_err(),
        "issuance cannot overwrite a subject"
    );
    storage.shutdown().await.unwrap();
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
struct SetTitle {
    title: String,
}
impl Command for SetTitle {
    const NAME: &'static str = "test.auth.set_title";
    type Output = String;
}

async fn command(storage: &Storage, key: Vec<u8>) -> StorageResult<Execution<String>> {
    storage
        .transact(move |ctx| {
            Ok(replay::execute(
                ctx,
                &Credential(&key),
                &NOW,
                &[RequiredAccess::Space {
                    id: "a",
                    permission: SpacePermission::ReadWrite,
                }],
                [42; 16],
                &SetTitle {
                    title: "saved".into(),
                },
                |ctx| {
                    ctx.edit("a", |doc| {
                        doc.put(ROOT, "title", "saved")?;
                        Ok(())
                    })?;
                    Ok("saved".into())
                },
            )?)
        })
        .await
}

#[tokio::test]
async fn persisted_auth_precedes_receipts_and_regrant_replays_same_subject_without_old_key() {
    let (dir, storage, admin, member) = setup().await;
    let rw = Arc::new(AccessKey::generate().unwrap());
    let (a, m, key) = (admin.clone(), member.clone(), rw.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(
                &a.credential(),
                &NOW,
                m.subject(),
                &key,
                &scope(SpacePermission::ReadWrite),
                None,
            )
        })
        .await
        .unwrap();
    assert!(
        !command(&storage, rw.credential().0.to_vec())
            .await
            .unwrap()
            .replayed
    );
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            ctx.revoke_access(&a.credential(), &NOW, m.subject())?;
            ctx.edit("a", |doc| {
                doc.put(ROOT, "title", "later")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    denied(command(&storage, rw.credential().0.to_vec()).await);
    let fresh = Arc::new(AccessKey::generate().unwrap());
    let (a, m, key) = (admin.clone(), member.clone(), fresh.clone());
    storage
        .transact(move |ctx| {
            ctx.replace_access(
                &a.credential(),
                &NOW,
                m.subject(),
                &key,
                &scope(SpacePermission::ReadWrite),
                None,
            )
        })
        .await
        .unwrap();
    assert!(
        command(&storage, fresh.credential().0.to_vec())
            .await
            .unwrap()
            .replayed
    );
    storage
        .read(|ctx| {
            assert_eq!(
                ctx.document("a")?.get(ROOT, "title")?.unwrap().0.as_str(),
                Some("later")
            );
            assert_eq!(ctx.document("a")?.get_changes(&[]).len(), 2);
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    // Raw credential bytes never appear in SQLite, graph, WAL or receipts.
    for entry in fs::read_dir(dir.path()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let bytes = fs::read(path).unwrap();
            for secret in [
                admin.credential(),
                member.credential(),
                rw.credential(),
                fresh.credential(),
            ] {
                assert!(!bytes.windows(32).any(|part| part == secret.0));
            }
        }
    }
}

#[tokio::test]
async fn failed_management_poisoning_rolls_back_partial_grants_and_domain_work() {
    let (_dir, storage, admin, member) = setup().await;
    let pending = Arc::new(PendingAccess::generate().unwrap());
    let (a, p) = (admin.clone(), pending.clone());
    assert!(
        storage
            .transact(move |ctx| {
                let invalid = GrantScope::Spaces(BTreeMap::from([
                    ("a".into(), SpacePermission::Read),
                    ("z-missing".into(), SpacePermission::ReadWrite),
                ]));
                assert!(
                    ctx.issue_access(&a.credential(), &NOW, &p, &invalid, None)
                        .is_err()
                );
                ctx.edit("a", |doc| {
                    doc.put(ROOT, "partial", true)?;
                    Ok(())
                })
            })
            .await
            .is_err()
    );
    let (a, p) = (admin.clone(), pending.clone());
    storage
        .transact(move |ctx| {
            assert!(
                ctx.inspect_access(&a.credential(), &NOW, p.subject())?
                    .is_none()
            );
            assert!(ctx.read().document("a")?.get(ROOT, "partial")?.is_none());
            ctx.issue_access(
                &a.credential(),
                &NOW,
                &p,
                &scope(SpacePermission::Read),
                None,
            )
        })
        .await
        .unwrap();
    let (a, m) = (admin.clone(), member.clone());
    assert!(
        storage
            .transact_checked(
                move |ctx| ctx.revoke_access(&a.credential(), &NOW, m.subject()),
                |stage| if stage == crate::Stage::BeforeCommit {
                    Err(StorageError::Unavailable)
                } else {
                    Ok(())
                }
            )
            .await
            .is_err()
    );
    check(
        &storage,
        member.credential().0.to_vec(),
        "a",
        SpacePermission::Read,
        100,
    )
    .await
    .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn malformed_or_unavailable_authority_and_unavailable_clock_never_fall_back() {
    let (dir, storage, admin, _member) = setup().await;
    struct NoClock;
    impl Clock for NoClock {
        fn now(&self) -> CommandResult<SystemTime> {
            Err(CommandError {
                code: ErrorCode::Unavailable,
            })
        }
    }
    let a = admin.clone();
    assert!(
        storage
            .transact(move |ctx| ctx.revoke_access(&a.credential(), &NoClock, "anything"))
            .await
            .is_err()
    );
    storage.shutdown().await.unwrap();
    let path = dir.path().join("device.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection
        .execute("DELETE FROM access_bootstrap", [])
        .unwrap();
    drop(connection);
    let before = fs::read(&path).unwrap();
    assert_eq!(
        Storage::open(&path).await.err(),
        Some(StorageError::InvalidDatabase)
    );
    assert_eq!(fs::read(path).unwrap(), before);
}

#[tokio::test]
async fn product_v1_migration_preserves_history_and_archives_without_inventing_access() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("device.db");
    let original = include_bytes!("../../tests/fixtures/schema-v1.sqlite");
    fs::write(&path, original).unwrap();
    assert_eq!(
        Storage::open(&path).await.err(),
        Some(StorageError::Unavailable)
    );
    assert_eq!(fs::read(&path).unwrap().as_slice(), original);
    let options = MigrationOptions {
        folder: dir.path().to_owned(),
        created_at_unix_ms: 10,
        limits: LIMITS,
    };
    let storage = Startup::new(&path)
        .automatic(Some(options.clone()))
        .await
        .unwrap();
    storage
        .read(|ctx| {
            assert!(ctx.recorded(b"old-command")?.is_some());
            assert_eq!(
                ctx.document("Историческое ../ 空間")?
                    .get_actor()
                    .to_bytes(),
                &[0x31]
            );
            Ok(())
        })
        .await
        .unwrap();
    storage
        .transact(|ctx| {
            let count: i64 =
                ctx.connection()
                    .query_row("SELECT count(*) FROM access_principals", [], |r| r.get(0))?;
            assert_eq!(count, 0, "migration does not grant anything");
            ctx.bootstrap_access(&PendingAccess::generate()?)
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
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
        1
    );
    assert!(backup.prepare("SELECT * FROM access_principals").is_err());
    drop(backup);
    let storage = Startup::new(&path).automatic(Some(options)).await.unwrap();
    storage.shutdown().await.unwrap();
    let connection = rusqlite::Connection::open(&path).unwrap();
    assert_eq!(
        connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        4
    );
}

#[tokio::test]
async fn serialized_revocation_blocks_queued_calls_and_excludes_other_writers_during_authorization()
{
    let (dir, storage, admin, member) = setup().await;
    let second = Storage::open(dir.path().join("device.db")).await.unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let worker = storage.clone();
    let m = member.clone();
    let blocker = tokio::spawn(async move {
        worker
            .transact(move |ctx| {
                with_authorized(
                    ctx,
                    &m.credential(),
                    &[RequiredAccess::Space {
                        id: "a",
                        permission: SpacePermission::Read,
                    }],
                    &NOW,
                    |_, _| {
                        entered.send(()).unwrap();
                        held.recv().unwrap();
                        Ok(())
                    },
                )?;
                Ok(())
            })
            .await
    });
    waiting.await.unwrap();
    let external = rusqlite::Connection::open(dir.path().join("device.db")).unwrap();
    external.busy_timeout(Duration::ZERO).unwrap();
    assert!(
        external.execute_batch("BEGIN IMMEDIATE").is_err(),
        "rights cannot change inside an authorized operation"
    );
    drop(external);
    let (a, m) = (admin.clone(), member.clone());
    let worker = storage.clone();
    let revocation = tokio::spawn(async move {
        worker
            .transact(move |ctx| ctx.revoke_access(&a.credential(), &NOW, m.subject()))
            .await
    });
    tokio::task::yield_now().await;
    let worker = storage.clone();
    let m = member.clone();
    let queued = tokio::spawn(async move {
        check(
            &worker,
            m.credential().0.to_vec(),
            "a",
            SpacePermission::Read,
            100,
        )
        .await
    });
    tokio::task::yield_now().await;
    release.send(()).unwrap();
    blocker.await.unwrap().unwrap();
    revocation.await.unwrap().unwrap();
    denied(queued.await.unwrap());
    denied(
        check(
            &second,
            member.credential().0.to_vec(),
            "a",
            SpacePermission::Read,
            100,
        )
        .await,
    );
    storage.shutdown().await.unwrap();
    second.shutdown().await.unwrap();
}

#[tokio::test]
async fn space_exports_exclude_authority_while_full_backup_preserves_it_and_retired_keys() {
    let (dir, storage, admin, member) = setup().await;
    let limits = LIMITS.snapshot;
    let before = storage
        .snapshot("a".into(), 10, limits)
        .await
        .unwrap()
        .encode(limits)
        .unwrap();
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| ctx.revoke_access(&a.credential(), &NOW, m.subject()))
        .await
        .unwrap();
    let after = storage
        .snapshot("a".into(), 10, limits)
        .await
        .unwrap()
        .encode(limits)
        .unwrap();
    assert_eq!(before, after);
    let backup = storage
        .prepare_migration_backup(dir.path(), 10, LIMITS)
        .await
        .unwrap();
    backup.verify(LIMITS).unwrap();
    let copy = rusqlite::Connection::open(backup.path().join("database.sqlite")).unwrap();
    let revoked: bool = copy
        .query_row(
            "SELECT revoked FROM access_principals WHERE subject = ?1",
            [member.subject()],
            |r| r.get(0),
        )
        .unwrap();
    assert!(revoked);
    let retired: bool = copy
        .query_row(
            "SELECT active = 0 FROM access_keys WHERE subject = ?1",
            [member.subject()],
            |r| r.get(0),
        )
        .unwrap();
    assert!(retired);
    assert!(
        !after
            .windows(member.subject().len())
            .any(|part| part == member.subject().as_bytes())
    );
    drop(copy);
    backup.discard().unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn invalid_policies_and_partial_bootstrap_never_create_authority() {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    assert!(
        storage
            .transact_checked(
                |ctx| ctx.bootstrap_access(&PendingAccess::generate()?),
                |p| if p == crate::Stage::BeforeCommit {
                    Err(StorageError::Unavailable)
                } else {
                    Ok(())
                }
            )
            .await
            .is_err()
    );
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    storage
        .transact(move |ctx| {
            ctx.create_document("a")?;
            ctx.bootstrap_access(&a)
        })
        .await
        .unwrap();
    for (scope, expires_at) in [
        (GrantScope::Spaces(BTreeMap::new()), None),
        (
            scope(SpacePermission::Read),
            Some(UNIX_EPOCH - Duration::from_millis(1)),
        ),
        (
            scope(SpacePermission::Read),
            Some(UNIX_EPOCH + Duration::from_millis(200) + Duration::from_micros(1)),
        ),
    ] {
        let (a, pending) = (admin.clone(), PendingAccess::generate().unwrap());
        assert!(
            storage
                .transact(move |ctx| ctx.issue_access(
                    &a.credential(),
                    &NOW,
                    &pending,
                    &scope,
                    expires_at
                ))
                .await
                .is_err()
        );
    }
    let a = admin.clone();
    assert!(
        storage
            .transact(move |ctx| ctx.replace_access(
                &a.credential(),
                &NOW,
                a.subject(),
                &AccessKey::generate()?,
                &scope(SpacePermission::Read),
                None
            ))
            .await
            .is_err(),
        "cannot downgrade final administrator"
    );
    storage.shutdown().await.unwrap();
}
