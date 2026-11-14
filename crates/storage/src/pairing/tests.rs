use super::*;
use crate::{Storage, access::PendingAccess};
use serde_json::json;
use std::{
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use zedone_core::{
    access::Clock,
    command::CommandResult,
    error::ErrorCode,
    invitations::{self, InvitationStatus, InvitationVerifier},
    pairing::{self, *},
    replay::Execution,
    wire::{CommandRequest, decode_request},
};
struct TestClock(u64);
impl Clock for TestClock {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_millis(self.0))
    }
}
fn sid(n: u8) -> String {
    format!("{n:032x}")
}
fn submit_request(n: u8) -> CommandRequest<RequestPairing> {
    CommandRequest {
        id: [n; 16],
        operation: "pairing.request".into(),
        parameters: RequestPairing {
            request: sid(n),
            invitation: sid(1),
            label: "Test device".into(),
            access_verifier: zedone_core::access_commands::KeyVerifier([n; 32]),
        },
    }
}
async fn setup() -> (tempfile::TempDir, Storage, Arc<PendingAccess>) {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
    let a = Arc::new(PendingAccess::generate().unwrap());
    let admin = a.clone();
    db.transact(move |tx| {
        tx.bootstrap_access(&admin)?;
        let r = decode_request::<invitations::CreateInvitation>(&serde_json::to_vec(&json!({
            "id":vec![1;16], "operation":"invitation.create", "parameters":{"invitation":sid(1),"verifier":vec![1;32]}
        })).unwrap())?;
        invitations::create(tx, &admin.credential(), &TestClock(100), &r)?;
        Ok(())
    }).await.unwrap();
    (dir, db, a)
}
async fn submit(
    db: &Storage,
    n: u8,
    code: u8,
    now: u64,
) -> StorageResult<Option<Execution<PairingSnapshot>>> {
    db.transact(move |tx| {
        Ok(pairing::request(
            tx,
            &TestClock(now),
            &submit_request(n),
            InvitationVerifier([code; 32]),
            PendingVerifier([n; 32]),
        )?)
    })
    .await
}
async fn current(db: &Storage, n: u8, id: u8) -> PairingSnapshot {
    db.transact(move |tx| {
        Ok(pairing::peer_inspect(
            tx,
            &CommandRequest {
                id: [id; 16],
                operation: "pairing.inspect".into(),
                parameters: InspectPairing { request: sid(n) },
            },
            PendingVerifier([n; 32]),
        )?
        .result)
    })
    .await
    .unwrap()
}
async fn invite(
    db: &Storage,
    admin: &Arc<PendingAccess>,
    id: u8,
    now: u64,
) -> invitations::InvitationSnapshot {
    let a = admin.clone();
    db.transact(move |tx| {
        Ok(invitations::inspect(
            tx,
            &a.credential(),
            &TestClock(now),
            &CommandRequest {
                id: [id; 16],
                operation: "invitation.inspect".into(),
                parameters: invitations::InspectInvitation { invitation: sid(1) },
            },
        )?
        .result)
    })
    .await
    .unwrap()
}
#[tokio::test]
async fn pending_retry_cancel_and_restart_never_create_access_or_revive_invitation() {
    let (dir, db, admin) = setup().await;
    let before = invite(&db, &admin, 2, 100).await;
    let accepted = submit(&db, 3, 1, 101).await.unwrap().unwrap();
    assert_eq!(accepted.result.status, PairingStatus::Pending);
    assert!(!accepted.replayed);
    assert_eq!(
        invite(&db, &admin, 4, 102).await.status,
        InvitationStatus::Used
    );
    assert_ne!(before.version, invite(&db, &admin, 5, 103).await.version);
    assert!(submit(&db, 4, 1, 103).await.unwrap().is_none());
    let replay = submit(&db, 3, 1, 900000).await.unwrap().unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, accepted.result);
    let mismatch = db
        .transact(|tx| {
            let mut r = submit_request(3);
            r.parameters.label = "Changed".into();
            Ok(pairing::request(
                tx,
                &TestClock(102),
                &r,
                InvitationVerifier([1; 32]),
                PendingVerifier([3; 32]),
            )?)
        })
        .await;
    assert!(
        matches!(mismatch,Err(StorageError::Rejected(e)) if e.code == ErrorCode::CommandMismatch)
    );
    let denied = db
        .transact(|tx| {
            Ok(pairing::peer_inspect(
                tx,
                &CommandRequest {
                    id: [6; 16],
                    operation: "pairing.inspect".into(),
                    parameters: InspectPairing { request: sid(3) },
                },
                PendingVerifier([9; 32]),
            )?)
        })
        .await;
    assert!(matches!(denied,Err(StorageError::Rejected(e)) if e.code == ErrorCode::AccessDenied));
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
    assert_eq!(current(&db, 3, 7).await, accepted.result);
    let r = CommandRequest {
        id: [8; 16],
        operation: "pairing.cancel".into(),
        parameters: CancelPairing {
            request: sid(3),
            expected: accepted.result.version,
        },
    };
    let cancelled = db
        .transact(move |tx| Ok(pairing::peer_cancel(tx, &r, PendingVerifier([3; 32]))?))
        .await
        .unwrap();
    assert_eq!(cancelled.result.status, PairingStatus::Cancelled);
    assert_ne!(cancelled.result.version, accepted.result.version);
    assert!(submit(&db, 4, 1, 110).await.unwrap().is_none());
    assert_eq!(
        submit(&db, 3, 1, 110).await.unwrap().unwrap().result,
        accepted.result
    );
    assert_eq!(current(&db, 3, 9).await.status, PairingStatus::Cancelled);
    db.transact(|tx| {
        let c = tx.connection();
        for (table, expected) in [
            ("access_principals", 1),
            ("access_keys", 1),
            ("documents", 0),
            ("peers", 0),
            ("pairing_requests", 1),
        ] {
            assert_eq!(
                c.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r
                    .get::<_, i64>(0))?,
                expected
            );
        }
        Ok(())
    })
    .await
    .unwrap();
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn failed_attempts_are_durable_and_expiry_has_an_exact_boundary() {
    let (dir, db, admin) = setup().await;
    let before = invite(&db, &admin, 2, 100).await;
    for n in 2..=4 {
        assert!(submit(&db, n, 9, 101).await.unwrap().is_none());
    }
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
    for n in 5..=6 {
        assert!(submit(&db, n, 9, 102).await.unwrap().is_none());
    }
    let locked = invite(&db, &admin, 7, 103).await;
    assert_eq!(locked.status, InvitationStatus::Locked);
    assert_ne!(locked.version, before.version);
    assert!(submit(&db, 8, 1, 104).await.unwrap().is_none());
    db.shutdown().await.unwrap();
    let (_dir, db, _admin) = setup().await;
    assert!(submit(&db, 3, 1, 600100).await.unwrap().is_none());
    assert!(submit(&db, 3, 1, 600099).await.unwrap().is_some());
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn cancelling_invitation_cancels_its_pending_request_without_reusing_the_code() {
    let (_dir, db, admin) = setup().await;
    submit(&db, 3, 1, 101).await.unwrap().unwrap();
    let before = invite(&db, &admin, 4, 102).await;
    let a = admin.clone();
    db.transact(move |tx| {
        Ok(invitations::cancel(
            tx,
            &a.credential(),
            &TestClock(103),
            &CommandRequest {
                id: [5; 16],
                operation: "invitation.cancel".into(),
                parameters: invitations::CancelInvitation {
                    invitation: sid(1),
                    expected: before.version,
                },
            },
        )?)
    })
    .await
    .unwrap();
    assert_eq!(current(&db, 3, 6).await.status, PairingStatus::Cancelled);
    assert!(submit(&db, 4, 1, 104).await.unwrap().is_none());
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn concurrent_devices_consume_one_invitation_exactly_once() {
    let (dir, db, _admin) = setup().await;
    let second = Storage::open(dir.path().join("pair.db")).await.unwrap();
    let (a, b) = tokio::join!(submit(&db, 3, 1, 101), submit(&second, 4, 1, 101));
    assert_eq!(
        usize::from(a.unwrap().is_some()) + usize::from(b.unwrap().is_some()),
        1
    );
    db.shutdown().await.unwrap();
    second.shutdown().await.unwrap();
}
#[tokio::test]
async fn request_and_consumption_roll_back_if_receipt_insert_or_commit_preparation_fails() {
    for stage in [crate::Stage::AfterWork, crate::Stage::BeforeCommit] {
        let (_dir, db, admin) = setup().await;
        let result = db
            .transact_checked(
                |tx| {
                    Ok(pairing::request(
                        tx,
                        &TestClock(101),
                        &submit_request(3),
                        InvitationVerifier([1; 32]),
                        PendingVerifier([3; 32]),
                    )?)
                },
                move |actual| {
                    if std::mem::discriminant(&actual) == std::mem::discriminant(&stage) {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await;
        assert!(matches!(result, Err(StorageError::Unavailable)));
        assert_eq!(
            invite(&db, &admin, 4, 102).await.status,
            InvitationStatus::Open
        );
        assert!(submit(&db, 3, 1, 103).await.unwrap().is_some());
        db.shutdown().await.unwrap();
    }
    let (_dir, db, admin) = setup().await;
    db.transact(|tx| { tx.connection().execute_batch("CREATE TRIGGER fail_pending_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'test'); END;")?; Ok(()) }).await.unwrap();
    assert!(submit(&db, 3, 1, 101).await.is_err());
    db.transact(|tx| {
        tx.connection()
            .execute_batch("DROP TRIGGER fail_pending_receipt")?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        invite(&db, &admin, 4, 102).await.status,
        InvitationStatus::Open
    );
    assert!(submit(&db, 3, 1, 103).await.unwrap().is_some());
    db.shutdown().await.unwrap();
}
