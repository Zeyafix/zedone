use crate::{Storage, StorageError, StorageResult, access::PendingAccess};
use serde_json::{Value, json};
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zedone_core::{
    access::{Clock, GrantScope},
    command::CommandResult,
    error::ErrorCode,
    invitations::{self, InvitationStore},
    replay::Execution,
    wire::{CommandRequest, decode_request},
};

struct TestClock(u64);
impl Clock for TestClock {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_millis(self.0))
    }
}
async fn setup() -> (tempfile::TempDir, Storage, Arc<PendingAccess>) {
    let dir = tempfile::tempdir().unwrap();
    let storage = Storage::open(dir.path().join("test.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    storage
        .transact(move |ctx| ctx.bootstrap_access(&a))
        .await
        .unwrap();
    (dir, storage, admin)
}
fn params(n: u8) -> Value {
    json!({"invitation":format!("{n:032x}"),"verifier":vec![n;32]})
}
async fn invoke(
    storage: &Storage,
    a: &Arc<PendingAccess>,
    now: u64,
    id: u8,
    op: &'static str,
    p: Value,
) -> StorageResult<Execution<Value>> {
    let a = a.clone();
    storage
        .transact(move |ctx| {
            let body = serde_json::to_vec(&json!({"id":vec![id;16],"operation":op,"parameters":p}))
                .unwrap();
            macro_rules! run {
                ($ty:ty, $handler:path) => {{
                    let r: CommandRequest<$ty> = decode_request(&body)?;
                    let e = $handler(ctx, &a.credential(), &TestClock(now), &r)?;
                    Ok(Execution {
                        result: serde_json::to_value(e.result).unwrap(),
                        replayed: e.replayed,
                    })
                }};
            }
            match op {
                "invitation.create" => run!(invitations::CreateInvitation, invitations::create),
                "invitation.inspect" => run!(invitations::InspectInvitation, invitations::inspect),
                "invitation.cancel" => run!(invitations::CancelInvitation, invitations::cancel),
                "invitation.list" => run!(invitations::ListInvitations, invitations::list),
                _ => unreachable!(),
            }
        })
        .await
}
fn rejected<T>(result: StorageResult<T>, code: ErrorCode) {
    assert!(matches!(result, Err(StorageError::Rejected(error)) if error.code == code));
}

#[tokio::test]
async fn expiry_cancel_replay_and_new_invitation_survive_restart_without_granting_access() {
    let (dir, storage, admin) = setup().await;
    let id = format!("{:032x}", 1);
    let created = invoke(&storage, &admin, 100, 1, "invitation.create", params(1))
        .await
        .unwrap();
    assert_eq!(created.result["created_at_unix_ms"], 100);
    assert_eq!(created.result["expires_at_unix_ms"], 600100);
    let inspect = json!({"invitation":id});
    assert_eq!(
        invoke(
            &storage,
            &admin,
            600099,
            2,
            "invitation.inspect",
            inspect.clone()
        )
        .await
        .unwrap()
        .result["status"],
        "open"
    );
    let expired = invoke(
        &storage,
        &admin,
        600100,
        3,
        "invitation.inspect",
        inspect.clone(),
    )
    .await
    .unwrap();
    assert_eq!(expired.result["status"], "expired");
    assert_eq!(expired.result["version"], created.result["version"]);
    let cancel = json!({"invitation":id,"expected":created.result["version"]});
    let cancelled = invoke(
        &storage,
        &admin,
        600100,
        4,
        "invitation.cancel",
        cancel.clone(),
    )
    .await
    .unwrap();
    assert_eq!(cancelled.result["status"], "cancelled");
    assert_ne!(cancelled.result["version"], created.result["version"]);
    rejected(
        invoke(
            &storage,
            &admin,
            600101,
            5,
            "invitation.cancel",
            cancel.clone(),
        )
        .await,
        ErrorCode::StaleState,
    );
    rejected(
        invoke(&storage, &admin, 600101, 5, "invitation.create", params(1)).await,
        ErrorCode::InvalidRequest,
    );
    let mut reused = params(1);
    reused["invitation"] = json!(format!("{:032x}", 2));
    rejected(
        invoke(&storage, &admin, 600101, 5, "invitation.create", reused).await,
        ErrorCode::InvalidRequest,
    );
    invoke(&storage, &admin, 600101, 5, "invitation.create", params(2))
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("test.db")).await.unwrap();
    let repeat = invoke(&storage, &admin, 900000, 1, "invitation.create", params(1))
        .await
        .unwrap();
    assert!(repeat.replayed);
    assert_eq!(repeat.result, created.result);
    assert_eq!(
        invoke(&storage, &admin, 900000, 4, "invitation.cancel", cancel)
            .await
            .unwrap()
            .result,
        cancelled.result
    );
    assert_eq!(
        invoke(&storage, &admin, 900000, 6, "invitation.inspect", inspect)
            .await
            .unwrap()
            .result["status"],
        "cancelled"
    );
    storage
        .transact(|ctx| {
            for table in ["access_principals", "access_keys", "access_bootstrap"] {
                let count: i64 = ctx.connection().query_row(
                    &format!("SELECT count(*) FROM {table}"),
                    [],
                    |r| r.get(0),
                )?;
                assert_eq!(count, 1);
            }
            for table in ["documents", "access_spaces", "changes", "peers"] {
                let count: i64 = ctx.connection().query_row(
                    &format!("SELECT count(*) FROM {table}"),
                    [],
                    |r| r.get(0),
                )?;
                assert_eq!(count, 0);
            }
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn receipt_failure_rolls_back_creation_and_cancel_and_preserves_exact_retry() {
    let (_dir, storage, admin) = setup().await;
    storage.transact(|ctx| {
        ctx.connection().execute_batch("CREATE TRIGGER deny_invitation_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT,'synthetic'); END;")?; Ok(())
    }).await.unwrap();
    assert!(
        invoke(&storage, &admin, 100, 1, "invitation.create", params(1))
            .await
            .is_err()
    );
    storage
        .transact(|ctx| {
            assert!(ctx.invitation(&format!("{:032x}", 1))?.is_none());
            ctx.connection()
                .execute_batch("DROP TRIGGER deny_invitation_receipt")?;
            Ok(())
        })
        .await
        .unwrap();
    let created = invoke(&storage, &admin, 100, 1, "invitation.create", params(1))
        .await
        .unwrap();
    let cancel = json!({"invitation":format!("{:032x}",1),"expected":created.result["version"]});
    storage.transact(|ctx| {ctx.connection().execute_batch("CREATE TRIGGER deny_invitation_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT,'synthetic'); END;")?; Ok(())}).await.unwrap();
    assert!(
        invoke(
            &storage,
            &admin,
            101,
            2,
            "invitation.cancel",
            cancel.clone()
        )
        .await
        .is_err()
    );
    storage
        .transact(|ctx| {
            assert!(!ctx.invitation(&format!("{:032x}", 1))?.unwrap().cancelled);
            ctx.connection()
                .execute_batch("DROP TRIGGER deny_invitation_receipt")?;
            Ok(())
        })
        .await
        .unwrap();
    let cancelled = invoke(&storage, &admin, 102, 2, "invitation.cancel", cancel)
        .await
        .unwrap();
    assert!(!cancelled.replayed);
    assert_eq!(cancelled.result["status"], "cancelled");
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn paginated_results_are_original_receipts_and_rights_are_checked_before_replay() {
    let (_dir, storage, admin) = setup().await;
    for n in 1..=51 {
        invoke(&storage, &admin, 100, n, "invitation.create", params(n))
            .await
            .unwrap();
    }
    let page = invoke(
        &storage,
        &admin,
        100,
        52,
        "invitation.list",
        json!({"after":null}),
    )
    .await
    .unwrap();
    assert_eq!(page.result["entries"].as_array().unwrap().len(), 50);
    assert_eq!(page.result["next"], format!("{:032x}", 50));
    let last = invoke(
        &storage,
        &admin,
        600100,
        53,
        "invitation.list",
        json!({"after":page.result["next"]}),
    )
    .await
    .unwrap();
    assert_eq!(last.result["entries"].as_array().unwrap().len(), 1);
    assert_eq!(last.result["entries"][0]["status"], "expired");
    assert!(last.result["next"].is_null());
    assert_eq!(
        invoke(
            &storage,
            &admin,
            600100,
            52,
            "invitation.list",
            json!({"after":null})
        )
        .await
        .unwrap()
        .result,
        page.result
    );
    let a = admin.clone();
    storage
        .transact(move |ctx| {
            let successor = PendingAccess::generate()?;
            ctx.issue_access(
                &a.credential(),
                &TestClock(100),
                &successor,
                &GrantScope::Administration,
                None,
            )?;
            ctx.connection().execute(
                "UPDATE access_principals SET revoked=1 WHERE subject=?1",
                [a.subject()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    rejected(
        invoke(&storage, &admin, 100, 1, "invitation.create", params(1)).await,
        ErrorCode::AccessDenied,
    );
    rejected(
        invoke(
            &storage,
            &admin,
            100,
            52,
            "invitation.list",
            json!({"after":null}),
        )
        .await,
        ErrorCode::AccessDenied,
    );
    rejected(
        invoke(
            &storage,
            &admin,
            100,
            54,
            "invitation.inspect",
            json!({"invitation":format!("{:032x}",99)}),
        )
        .await,
        ErrorCode::AccessDenied,
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn concurrent_cancels_have_one_winner_and_maximum_time_is_platform_independent() {
    let (dir, storage, admin) = setup().await;
    let max = zedone_core::access_commands::MAX_EXPIRY_UNIX_MS as u64;
    rejected(
        invoke(
            &storage,
            &admin,
            max - 599999,
            1,
            "invitation.create",
            params(1),
        )
        .await,
        ErrorCode::Unavailable,
    );
    let created = invoke(
        &storage,
        &admin,
        max - 600000,
        1,
        "invitation.create",
        params(1),
    )
    .await
    .unwrap();
    assert_eq!(created.result["expires_at_unix_ms"], max);
    let other = Storage::open(dir.path().join("test.db")).await.unwrap();
    let p = json!({"invitation":format!("{:032x}",1),"expected":created.result["version"]});
    let (a, b) = tokio::join!(
        invoke(&storage, &admin, max, 2, "invitation.cancel", p.clone()),
        invoke(&other, &admin, max, 3, "invitation.cancel", p)
    );
    assert_ne!(a.is_ok(), b.is_ok());
    rejected(if a.is_err() { a } else { b }, ErrorCode::StaleState);
    other.shutdown().await.unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn expiring_admin_cannot_replay_or_inspect_at_its_exclusive_deadline() {
    let (_dir, storage, admin) = setup().await;
    let delegate = Arc::new(PendingAccess::generate().unwrap());
    let (a, d) = (admin.clone(), delegate.clone());
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &TestClock(100),
                &d,
                &GrantScope::Administration,
                Some(UNIX_EPOCH + Duration::from_millis(200)),
            )
        })
        .await
        .unwrap();
    invoke(&storage, &delegate, 199, 1, "invitation.create", params(1))
        .await
        .unwrap();
    rejected(
        invoke(&storage, &delegate, 200, 1, "invitation.create", params(1)).await,
        ErrorCode::AccessDenied,
    );
    rejected(
        invoke(
            &storage,
            &delegate,
            200,
            2,
            "invitation.inspect",
            json!({"invitation":format!("{:032x}",1)}),
        )
        .await,
        ErrorCode::AccessDenied,
    );
    storage.shutdown().await.unwrap();
}
