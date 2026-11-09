use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering},
};

use automerge::{ROOT, ReadDoc, transaction::Transactable};
use tempfile::TempDir;
use zedone_core::access::{
    AccessTransaction, Credential, GrantScope, RequiredAccess, with_authorized,
};

use super::*;
use crate::access::PendingAccess;

fn source(time: Arc<AtomicU64>) -> impl Fn() -> CommandResult<SystemTime> + Send + 'static {
    static START: std::sync::OnceLock<Instant> = std::sync::OnceLock::new();
    let start = *START.get_or_init(Instant::now);
    move || {
        Ok(UNIX_EPOCH
            + Duration::from_millis(
                time.fetch_add(1, Ordering::SeqCst) + start.elapsed().as_millis() as u64,
            ))
    }
}

async fn setup() -> (TempDir, Storage, Arc<PendingAccess>, Arc<AtomicU64>) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let access = Arc::new(PendingAccess::generate().unwrap());
    let a = access.clone();
    storage
        .transact(move |ctx| {
            ctx.create_document("a")?;
            ctx.bootstrap_access(&a)
        })
        .await
        .unwrap();
    (dir, storage, access, Arc::new(AtomicU64::new(1_000_000)))
}

async fn authorize(
    storage: &Storage,
    access: &PendingAccess,
    time: Arc<AtomicU64>,
) -> StorageResult<()> {
    let key = access.credential().0.to_vec();
    storage
        .timed_with_source(source(time), move |ctx, clock| {
            Ok(with_authorized(
                ctx,
                &Credential(&key),
                &[RequiredAccess::ManageAccess],
                clock,
                |_, _| Ok(()),
            )?)
        })
        .await
}

#[tokio::test]
async fn rollback_and_restart_never_reduce_observed_time_and_recovery_is_explicit() {
    let (dir, storage, admin, time) = setup().await;
    authorize(&storage, &admin, time.clone()).await.unwrap();
    time.store(100, Ordering::SeqCst);
    assert!(authorize(&storage, &admin, time.clone()).await.is_err());
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    assert!(authorize(&storage, &admin, time.clone()).await.is_err());
    time.store(2_000_000, Ordering::SeqCst);
    authorize(&storage, &admin, time).await.unwrap();
}

#[tokio::test]
async fn expiry_before_commit_rolls_back_data_and_receipt_but_keeps_clock() {
    let (dir, storage, admin, time) = setup().await;
    let key = admin.credential().0.to_vec();
    let jump = time.clone();
    let result = storage
        .timed_with_source(source(time.clone()), move |ctx, clock| {
            with_authorized(
                ctx,
                &Credential(&key),
                &[RequiredAccess::ManageAccess],
                clock,
                |ctx, _| {
                    // Register a shorter deadline as a nested authorized operation would.
                    clock.check_expiry(Some(UNIX_EPOCH + Duration::from_millis(1_500_000)))?;
                    ctx.edit("a", |tx| {
                        tx.put(ROOT, "title", "must roll back")?;
                        Ok(())
                    })?;
                    ctx.record(
                        b"timed",
                        &crate::RecordedResult {
                            request: vec![1],
                            result: vec![2],
                        },
                    )?;
                    jump.store(2_000_000, Ordering::SeqCst);
                    Ok(())
                },
            )?;
            Ok(())
        })
        .await;
    assert_eq!(
        result,
        Err(StorageError::Rejected(CommandError {
            code: ErrorCode::AccessDenied
        }))
    );
    storage
        .read(|ctx| {
            assert!(ctx.document("a")?.get(ROOT, "title")?.is_none());
            assert!(ctx.recorded(b"timed")?.is_none());
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    time.store(1_100_000, Ordering::SeqCst);
    assert!(authorize(&storage, &admin, time).await.is_err());
}

#[tokio::test]
async fn error_and_panic_preserve_time_without_partial_changes() {
    for panic in [false, true] {
        let (_dir, storage, admin, time) = setup().await;
        let jump = time.clone();
        assert!(
            storage
                .timed_with_source(
                    source(time.clone()),
                    move |ctx, clock| -> StorageResult<()> {
                        ctx.edit("a", |tx| {
                            tx.put(ROOT, "title", "discard")?;
                            Ok(())
                        })?;
                        jump.store(2_000_000, Ordering::SeqCst);
                        clock.now()?;
                        if panic {
                            panic!("controlled worker failure");
                        }
                        Err(StorageError::Unavailable)
                    }
                )
                .await
                .is_err()
        );
        time.store(1_100_000, Ordering::SeqCst);
        assert!(authorize(&storage, &admin, time).await.is_err());
        storage
            .read(|ctx| {
                assert!(ctx.document("a")?.get(ROOT, "title")?.is_none());
                Ok(())
            })
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn clock_failure_never_runs_work_or_creates_authority() {
    let (_dir, storage, _, _) = setup().await;
    let result = storage
        .timed_with_source(
            || Err(unavailable()),
            |_, _| -> StorageResult<()> { panic!("must not run") },
        )
        .await;
    assert_eq!(result, Err(StorageError::Rejected(unavailable())));
    let result = storage
        .timed_with_source(
            || Ok(UNIX_EPOCH - Duration::from_secs(1)),
            |_, _| -> StorageResult<()> { panic!("must not run") },
        )
        .await;
    assert_eq!(result, Err(StorageError::Rejected(unavailable())));
}

#[tokio::test]
async fn space_creation_expiring_at_the_final_commit_gate_leaves_neither_space_nor_new_access() {
    use zedone_core::{
        spaces::{self, CreateSpace},
        wire::CommandRequest,
    };
    let (_dir, storage, admin, time) = setup().await;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_with_source(source(time.clone()), move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Administration,
                Some(UNIX_EPOCH + Duration::from_millis(1_500_000)),
            )
        })
        .await
        .unwrap();
    let jump = time.clone();
    let m = member.clone();
    let result = storage
        .timed_with_source(source(time), move |ctx, clock| {
            let request = CommandRequest {
                id: [53; 16],
                operation: "space.create".into(),
                parameters: CreateSpace {
                    space: "3".repeat(32),
                    name: "Must roll back".into(),
                },
            };
            let completed = spaces::create(ctx, &m.credential(), clock, &request)?;
            jump.store(2_000_000, Ordering::SeqCst);
            Ok(completed)
        })
        .await;
    assert!(matches!(
        result,
        Err(StorageError::Rejected(CommandError {
            code: ErrorCode::AccessDenied
        }))
    ));
    storage
        .transact(move |ctx| {
            assert!(matches!(
                ctx.resolve_access(&member.credential())?
                    .unwrap()
                    .grant
                    .scope,
                GrantScope::Administration
            ));
            assert_eq!(
                ctx.connection().query_row(
                    "SELECT count(*) FROM documents WHERE space = ?1",
                    ["3".repeat(32)],
                    |row| row.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                ctx.connection().query_row(
                    "SELECT count(*) FROM access_spaces WHERE subject = ?1",
                    [member.subject()],
                    |row| row.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                ctx.connection()
                    .query_row("SELECT count(*) FROM command_results", [], |row| row
                        .get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn observed_expiry_survives_restart_and_only_new_key_regrants_access() {
    let (dir, storage, admin, time) = setup().await;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_with_source(source(time.clone()), move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Administration,
                Some(UNIX_EPOCH + Duration::from_millis(1_500_000)),
            )
        })
        .await
        .unwrap();
    time.store(2_000_000, Ordering::SeqCst);
    assert!(authorize(&storage, &member, time.clone()).await.is_err());
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    time.store(1_100_000, Ordering::SeqCst);
    assert!(authorize(&storage, &member, time.clone()).await.is_err());
    time.store(3_000_000, Ordering::SeqCst);
    let replacement = Arc::new(crate::access::AccessKey::generate().unwrap());
    let (a, m, r) = (admin.clone(), member.clone(), replacement.clone());
    storage
        .timed_with_source(source(time.clone()), move |ctx, clock| {
            ctx.replace_access(
                &a.credential(),
                clock,
                m.subject(),
                &r,
                &GrantScope::Administration,
                None,
            )
        })
        .await
        .unwrap();
    assert!(authorize(&storage, &member, time.clone()).await.is_err());
    let r = replacement.clone();
    storage
        .timed_with_source(source(time), move |ctx, clock| {
            Ok(with_authorized(
                ctx,
                &r.credential(),
                &[RequiredAccess::ManageAccess],
                clock,
                |_, _| Ok(()),
            )?)
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn initial_schema_restart_preserves_authority_and_revoked_keys() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("device.db");
    let old = Storage::open(&path).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let member = Arc::new(PendingAccess::generate().unwrap());
    let replacement = Arc::new(crate::access::AccessKey::generate().unwrap());
    let (a, m, r) = (admin.clone(), member.clone(), replacement.clone());
    struct Fixed;
    impl Clock for Fixed {
        fn now(&self) -> CommandResult<SystemTime> {
            Ok(UNIX_EPOCH + Duration::from_secs(1))
        }
    }
    old.transact(move |ctx| {
        ctx.bootstrap_access(&a)?;
        ctx.issue_access(
            &a.credential(),
            &Fixed,
            &m,
            &GrantScope::Administration,
            None,
        )?;
        ctx.replace_access(
            &a.credential(),
            &Fixed,
            m.subject(),
            &r,
            &GrantScope::Administration,
            None,
        )?;
        ctx.record(
            b"v2-receipt",
            &crate::RecordedResult {
                request: vec![1],
                result: vec![2],
            },
        )
    })
    .await
    .unwrap();
    old.shutdown().await.unwrap();
    let storage = Storage::open(&path).await.unwrap();
    storage
        .timed_transact(move |ctx, clock| {
            assert!(ctx.resolve_access(&member.credential())?.is_none());
            assert_eq!(ctx.read().recorded(b"v2-receipt")?.unwrap().result, vec![2]);
            Ok(with_authorized(
                ctx,
                &replacement.credential(),
                &[RequiredAccess::ManageAccess],
                clock,
                |_, _| Ok(()),
            )?)
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let conn = Connection::open(&path).unwrap();
    assert_eq!(
        conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        crate::schema::REGISTRY.current().version
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM access_principals", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[tokio::test]
async fn a_queued_operation_uses_time_after_waiting_on_the_writer() {
    let (_dir, storage, admin, time) = setup().await;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_with_source(source(time.clone()), move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Administration,
                Some(UNIX_EPOCH + Duration::from_millis(1_500_000)),
            )
        })
        .await
        .unwrap();
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let worker = storage.clone();
    let blocker = tokio::spawn(async move {
        worker
            .transact(move |_| {
                entered.send(()).unwrap();
                held.recv().unwrap();
                Ok(())
            })
            .await
    });
    waiting.await.unwrap();
    let mut queued = Box::pin(storage.timed_with_source(
        source(time.clone()),
        move |ctx, clock| {
            Ok(with_authorized(
                ctx,
                &member.credential(),
                &[RequiredAccess::ManageAccess],
                clock,
                |_, _| -> CommandResult<()> { panic!("expired operation must not execute") },
            )?)
        },
    ));
    std::future::poll_fn(|cx| {
        assert!(queued.as_mut().poll(cx).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    time.store(2_000_000, Ordering::SeqCst);
    release.send(()).unwrap();
    blocker.await.unwrap().unwrap();
    assert_eq!(
        queued.await,
        Err(StorageError::Rejected(CommandError {
            code: ErrorCode::AccessDenied
        }))
    );
}

#[test]
fn advancing_calendar_below_the_monotonic_projection_remains_available() {
    let raw = Cell::new(1_000_000_u64);
    let wall = || Ok(UNIX_EPOCH + Duration::from_millis(raw.get()));
    let timeline = std::sync::Mutex::new(Some((
        Instant::now() - Duration::from_millis(10),
        1_000_000,
    )));
    let clock = TransactionClock {
        source: &wall,
        timeline: &timeline,
        calendar: Cell::new(0),
        effective: Cell::new(0),
        deadline: Cell::new(None),
    };
    let effective = millis(clock.now().unwrap()).unwrap();
    raw.set((effective - 1) as u64);
    assert!(raw.get() > 1_000_000, "calendar strictly advanced");
    assert!(millis(clock.now().unwrap()).unwrap() >= effective);
    raw.set(raw.get() - 1);
    assert_eq!(clock.now(), Err(unavailable()), "real rollback still fails");
}

#[test]
fn forward_jump_rebases_the_monotonic_floor_and_a_frozen_wall_cannot_extend_access() {
    let timeline = std::sync::Mutex::new(Some((Instant::now() - Duration::from_secs(1), 10)));
    let wall = || Ok(UNIX_EPOCH + Duration::from_millis(1_000_000));
    let clock = TransactionClock {
        source: &wall,
        timeline: &timeline,
        calendar: Cell::new(0),
        effective: Cell::new(0),
        deadline: Cell::new(None),
    };
    clock.now().unwrap();
    assert_eq!(timeline.lock().unwrap().unwrap().1, 1_000_000);
    *timeline.lock().unwrap() = Some((Instant::now() - Duration::from_secs(10), 1_000_000));
    assert!(clock.now().unwrap() >= UNIX_EPOCH + Duration::from_millis(1_010_000));
    assert!(clock.now().unwrap() >= UNIX_EPOCH + Duration::from_millis(1_010_000));
    assert_eq!(
        clock.check_expiry(Some(UNIX_EPOCH + Duration::from_millis(1_005_000))),
        Err(CommandError {
            code: ErrorCode::AccessDenied
        }),
        "a frozen calendar cannot extend access past the monotonic deadline"
    );
}

#[tokio::test]
async fn independent_workers_recheck_the_same_watermark_and_corruption_fails_startup() {
    let (dir, storage, admin, time) = setup().await;
    let path = dir.path().join("device.db");
    let other = Storage::open(&path).await.unwrap();
    authorize(&storage, &admin, time.clone()).await.unwrap();
    time.store(100, Ordering::SeqCst);
    assert!(authorize(&other, &admin, time.clone()).await.is_err());
    time.store(2_000_000, Ordering::SeqCst);
    authorize(&other, &admin, time.clone()).await.unwrap();
    time.store(1_500_000, Ordering::SeqCst);
    assert!(authorize(&storage, &admin, time).await.is_err());
    storage.shutdown().await.unwrap();
    other.shutdown().await.unwrap();
    let conn = Connection::open(&path).unwrap();
    conn.execute("DELETE FROM access_clock", []).unwrap();
    drop(conn);
    assert_eq!(
        Storage::open(&path).await.err(),
        Some(StorageError::InvalidDatabase)
    );
}

async fn clock_marks(storage: &Storage) -> (i64, i64) {
    storage
        .run(|connection| {
            Ok(connection.query_row(
                "SELECT calendar_ms, effective_ms FROM access_clock",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )?)
        })
        .await
        .unwrap()
}

fn fixed_source(wall: u64) -> impl Fn() -> CommandResult<SystemTime> + Send + 'static {
    move || Ok(UNIX_EPOCH + Duration::from_millis(wall))
}

#[tokio::test]
async fn monotonic_expiry_on_frozen_calendar_survives_restart() {
    let (dir, storage, admin, _) = setup().await;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_with_source(fixed_source(1_000_000), move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Administration,
                Some(UNIX_EPOCH + Duration::from_millis(1_050_000)),
            )
        })
        .await
        .unwrap();
    *storage.clock_anchor.lock().unwrap() =
        Some((Instant::now() - Duration::from_secs(60), 1_000_000));
    let key = member.credential().0.to_vec();
    let expired = storage
        .timed_with_source(fixed_source(1_000_000), move |ctx, clock| {
            Ok(with_authorized(
                ctx,
                &Credential(&key),
                &[RequiredAccess::ManageAccess],
                clock,
                |_, _| -> CommandResult<()> { panic!("expired work must not run") },
            )?)
        })
        .await;
    assert_eq!(
        expired,
        Err(StorageError::Rejected(CommandError {
            code: ErrorCode::AccessDenied,
        }))
    );
    let before = clock_marks(&storage).await;
    assert_eq!(before.0, 1_000_000);
    assert!(before.1 >= 1_060_000);
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let key = member.credential().0.to_vec();
    assert_eq!(
        storage
            .timed_with_source(fixed_source(1_000_000), move |ctx, clock| {
                // Equal calendar time is valid, but does not revive the key.
                Ok(with_authorized(
                    ctx,
                    &Credential(&key),
                    &[RequiredAccess::ManageAccess],
                    clock,
                    |_, _| -> CommandResult<()> { panic!("restart must not revive access") },
                )?)
            })
            .await,
        expired
    );
    assert!(clock_marks(&storage).await.1 >= before.1);
    assert_eq!(
        storage
            .timed_with_source(fixed_source(999_999), |_, _| -> StorageResult<()> {
                panic!("calendar rollback must fail before work")
            })
            .await,
        Err(StorageError::Rejected(unavailable()))
    );
}

#[tokio::test]
async fn frozen_calendar_expiry_at_commit_rolls_back_work_but_keeps_effective_time() {
    let (_dir, storage, _, _) = setup().await;
    assert_eq!(
        storage
            .timed_with_source(fixed_source(1_000_000), |ctx, clock| {
                clock.check_expiry(Some(UNIX_EPOCH + Duration::from_millis(1_050_000)))?;
                ctx.edit("a", |tx| {
                    tx.put(ROOT, "title", "must roll back at the final gate")?;
                    Ok(())
                })?;
                ctx.record(
                    b"frozen-expiry",
                    &crate::RecordedResult {
                        request: vec![1],
                        result: vec![2],
                    },
                )?;
                *clock.timeline.lock().unwrap() =
                    Some((Instant::now() - Duration::from_secs(60), 1_000_000));
                Ok(())
            })
            .await,
        Err(StorageError::Rejected(CommandError {
            code: ErrorCode::AccessDenied,
        }))
    );
    let marks = clock_marks(&storage).await;
    assert_eq!(marks.0, 1_000_000);
    assert!(marks.1 >= 1_060_000);
    storage
        .read(|ctx| {
            assert!(ctx.document("a")?.get(ROOT, "title")?.is_none());
            assert!(ctx.recorded(b"frozen-expiry")?.is_none());
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn independent_workers_preserve_both_marks_and_rebase_effective_time() {
    let (dir, storage, _, _) = setup().await;
    let other = Storage::open(dir.path().join("device.db")).await.unwrap();
    *storage.clock_anchor.lock().unwrap() =
        Some((Instant::now() - Duration::from_secs(20), 1_000_000));
    storage
        .timed_with_source(fixed_source(1_000_000), |_, _| Ok(()))
        .await
        .unwrap();
    let first = clock_marks(&storage).await;
    assert_eq!(first.0, 1_000_000);
    assert!(first.1 >= 1_020_000);
    other
        .timed_with_source(fixed_source(1_000_009), |_, _| Ok(()))
        .await
        .unwrap();
    assert!(other.clock_anchor.lock().unwrap().unwrap().1 >= first.1);
    *other.clock_anchor.lock().unwrap() = Some((Instant::now() - Duration::from_secs(2), first.1));
    other
        .timed_with_source(fixed_source(1_000_009), |_, _| Ok(()))
        .await
        .unwrap();
    let second = clock_marks(&other).await;
    assert_eq!(second.0, 1_000_009);
    assert!(second.1 >= first.1 + 2_000);
    storage
        .timed_with_source(fixed_source(1_000_009), |_, _| Ok(()))
        .await
        .unwrap();
    assert!(storage.clock_anchor.lock().unwrap().unwrap().1 >= second.1);
    assert_eq!(
        other
            .timed_with_source(fixed_source(1_000_008), |_, _| -> StorageResult<()> {
                panic!("another worker must observe calendar rollback")
            })
            .await,
        Err(StorageError::Rejected(unavailable()))
    );
}

#[tokio::test]
async fn failure_and_panic_preserve_separate_marks_without_partial_work() {
    for panic in [false, true] {
        let (dir, storage, _, _) = setup().await;
        *storage.clock_anchor.lock().unwrap() =
            Some((Instant::now() - Duration::from_secs(20), 1_000_000));
        assert!(
            storage
                .timed_with_source(
                    fixed_source(1_000_009),
                    move |ctx, _| -> StorageResult<()> {
                        ctx.edit("a", |tx| {
                            tx.put(ROOT, "title", "discard")?;
                            Ok(())
                        })?;
                        if panic {
                            panic!("controlled failure with a separated effective floor");
                        }
                        Err(StorageError::Unavailable)
                    }
                )
                .await
                .is_err()
        );
        let marks = clock_marks(&storage).await;
        assert_eq!(marks.0, 1_000_009);
        assert!(marks.1 >= 1_020_000);
        storage.shutdown().await.unwrap();
        let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
        storage
            .timed_with_source(fixed_source(1_000_009), move |ctx, clock| {
                assert!(ctx.read().document("a")?.get(ROOT, "title")?.is_none());
                assert!(millis(clock.now()?)? >= marks.1);
                Ok(())
            })
            .await
            .unwrap();
    }
}
