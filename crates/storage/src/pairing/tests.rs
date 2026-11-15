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

fn confirmation(
    version: zedone_core::replay::StateVersion,
    id: u8,
) -> CommandRequest<ConfirmPairing> {
    CommandRequest {
        id: [id; 16],
        operation: "pairing.confirm".into(),
        parameters: ConfirmPairing {
            request: sid(3),
            expected: version,
            scope: zedone_core::access_commands::AccessScope::administration(),
            expires_at_unix_ms: None,
        },
    }
}
async fn approve(
    db: &Storage,
    admin: &Arc<PendingAccess>,
    version: zedone_core::replay::StateVersion,
    id: u8,
) -> StorageResult<Execution<PairingSnapshot>> {
    let a = admin.clone();
    db.transact(move |tx| {
        Ok(pairing::confirm(
            tx,
            &a.credential(),
            &TestClock(102),
            &confirmation(version, id),
        )?)
    })
    .await
}
#[tokio::test]
async fn confirmation_is_atomic_replayable_and_unlink_preserves_documents_after_restart() {
    use zedone_core::{access::AccessTransaction, access_commands::AccessStore};
    let (dir, db, admin) = setup().await;
    db.transact(|tx| {
        tx.create_document("preserved")?;
        Ok(())
    })
    .await
    .unwrap();
    let pending = submit(&db, 3, 1, 101).await.unwrap().unwrap().result;
    let confirmed = approve(&db, &admin, pending.version, 10).await.unwrap();
    assert_eq!(confirmed.result.status, PairingStatus::Confirmed);
    assert_eq!(confirmed.result.subject, Some(sid(3)));
    assert!(
        approve(&db, &admin, pending.version, 10)
            .await
            .unwrap()
            .replayed
    );
    assert!(
        matches!(approve(&db,&admin,pending.version,11).await,Err(StorageError::Rejected(e)) if e.code==ErrorCode::StaleState)
    );
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
    assert_eq!(current(&db, 3, 12).await, confirmed.result);
    let version = confirmed.result.version;
    let unlinked = db
        .transact(move |tx| {
            Ok(pairing::peer_unlink(
                tx,
                &CommandRequest {
                    id: [13; 16],
                    operation: "pairing.unlink".into(),
                    parameters: UnlinkPairing {
                        request: sid(3),
                        expected: version,
                    },
                },
                PendingVerifier([3; 32]),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(unlinked.result.status, PairingStatus::Unlinked);
    db.transact(|tx| {
        assert!(tx.access_snapshot(&sid(3))?.unwrap().revoked);
        // The pending bearer is never an ordinary access credential.
        assert!(
            tx.resolve_access(&zedone_core::access::Credential(&[3; 32]))?
                .is_none()
        );
        assert_eq!(
            tx.connection()
                .query_row("SELECT count(*) FROM documents", [], |r| r.get::<_, i64>(0))?,
            1
        );
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        approve(&db, &admin, pending.version, 10)
            .await
            .unwrap()
            .result,
        confirmed.result
    );
    assert_eq!(current(&db, 3, 14).await.status, PairingStatus::Unlinked);
    assert!(submit(&db, 4, 1, 104).await.unwrap().is_none());
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
    assert_eq!(current(&db, 3, 15).await.status, PairingStatus::Unlinked);
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn reject_and_cancel_are_terminal_and_never_issue_or_revive_access() {
    for reject in [true, false] {
        let (_dir, db, admin) = setup().await;
        let before = submit(&db, 3, 1, 101).await.unwrap().unwrap().result;
        let a = admin.clone();
        let result = db
            .transact(move |tx| {
                if reject {
                    Ok(pairing::reject(
                        tx,
                        &a.credential(),
                        &TestClock(102),
                        &CommandRequest {
                            id: [10; 16],
                            operation: "pairing.reject".into(),
                            parameters: RejectPairing {
                                request: sid(3),
                                expected: before.version,
                            },
                        },
                    )?)
                } else {
                    Ok(pairing::peer_cancel(
                        tx,
                        &CommandRequest {
                            id: [10; 16],
                            operation: "pairing.cancel".into(),
                            parameters: CancelPairing {
                                request: sid(3),
                                expected: before.version,
                            },
                        },
                        PendingVerifier([3; 32]),
                    )?)
                }
            })
            .await
            .unwrap();
        assert_eq!(
            result.result.status,
            if reject {
                PairingStatus::Rejected
            } else {
                PairingStatus::Cancelled
            }
        );
        assert!(
            matches!(approve(&db,&admin,result.result.version,11).await,Err(StorageError::Rejected(e)) if e.code==ErrorCode::InvalidRequest)
        );
        db.transact(|tx| {
            assert_eq!(
                tx.connection()
                    .query_row("SELECT count(*) FROM access_principals", [], |r| r
                        .get::<_, i64>(0))?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
        assert!(submit(&db, 4, 1, 103).await.unwrap().is_none());
        db.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn confirmation_and_rejection_race_serializes_with_a_single_winner() {
    let (dir, db, admin) = setup().await;
    let other = Storage::open(dir.path().join("pair.db")).await.unwrap();
    let before = submit(&db, 3, 1, 101).await.unwrap().unwrap().result;
    let a = admin.clone();
    let reject = other.transact(move |tx| {
        Ok(pairing::reject(
            tx,
            &a.credential(),
            &TestClock(102),
            &CommandRequest {
                id: [11; 16],
                operation: "pairing.reject".into(),
                parameters: RejectPairing {
                    request: sid(3),
                    expected: before.version,
                },
            },
        )?)
    });
    let (approved, rejected) = tokio::join!(approve(&db, &admin, before.version, 10), reject);
    assert_eq!(
        usize::from(approved.is_ok()) + usize::from(rejected.is_ok()),
        1
    );
    let status = current(&db, 3, 12).await.status;
    assert!(matches!(
        status,
        PairingStatus::Confirmed | PairingStatus::Rejected
    ));
    db.shutdown().await.unwrap();
    other.shutdown().await.unwrap();
}
#[tokio::test]
async fn confirmation_rolls_back_grant_decision_and_receipt_on_every_write_failure() {
    for failure in 0..5 {
        let (_dir, db, admin) = setup().await;
        let before = submit(&db, 3, 1, 101).await.unwrap().unwrap().result;
        if failure < 3 {
            db.transact(move |tx| {
                let sql=match failure {
                    0=>"CREATE TRIGGER fail BEFORE INSERT ON access_keys BEGIN SELECT RAISE(ABORT,'test'); END;",
                    1=>"CREATE TRIGGER fail BEFORE UPDATE ON pairing_requests BEGIN SELECT RAISE(ABORT,'test'); END;",
                    _=>"CREATE TRIGGER fail BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT,'test'); END;",
                };
                tx.connection().execute_batch(sql)?; Ok(())
            }).await.unwrap();
        }
        let a = admin.clone();
        let result = db
            .transact_checked(
                move |tx| {
                    Ok(pairing::confirm(
                        tx,
                        &a.credential(),
                        &TestClock(102),
                        &confirmation(before.version, 10),
                    )?)
                },
                move |stage| {
                    if (failure == 3 && matches!(stage, crate::Stage::AfterWork))
                        || (failure == 4 && matches!(stage, crate::Stage::BeforeCommit))
                    {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await;
        assert!(result.is_err());
        if failure < 3 {
            db.transact(|tx| {
                tx.connection().execute_batch("DROP TRIGGER fail")?;
                Ok(())
            })
            .await
            .unwrap();
        }
        assert_eq!(current(&db, 3, 11).await, before);
        db.transact(|tx| {
            assert_eq!(
                tx.connection()
                    .query_row("SELECT count(*) FROM access_principals", [], |r| r
                        .get::<_, i64>(0))?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
        assert!(
            !approve(&db, &admin, before.version, 10)
                .await
                .unwrap()
                .replayed
        );
        db.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn invalid_scopes_expiries_and_colliding_subjects_never_partially_confirm() {
    use std::collections::BTreeMap;
    use zedone_core::access_commands::{AccessMode, AccessScope};
    let (_dir, db, admin) = setup().await;
    let before = submit(&db, 3, 1, 101).await.unwrap().unwrap().result;
    for end in [Some(-1), Some(102), Some(101), Some(i64::MAX)] {
        let a = admin.clone();
        assert!(
            db.transact(move |tx| {
                let mut r = confirmation(before.version, 10);
                r.parameters.expires_at_unix_ms = end;
                Ok(pairing::confirm(tx, &a.credential(), &TestClock(102), &r)?)
            })
            .await
            .is_err()
        );
    }
    for spaces in [
        BTreeMap::new(),
        BTreeMap::from([(sid(99), AccessMode::Read)]),
    ] {
        let a = admin.clone();
        assert!(
            db.transact(move |tx| {
                let mut r = confirmation(before.version, 10);
                r.parameters.scope = AccessScope::spaces(spaces);
                Ok(pairing::confirm(tx, &a.credential(), &TestClock(102), &r)?)
            })
            .await
            .is_err()
        );
    }
    db.transact(move |tx| {
        use zedone_core::access_commands::AccessStore;
        tx.issue_subject(
            &sid(3),
            zedone_core::access_commands::KeyVerifier([88; 32]),
            &AccessScope::administration(),
            None,
        )?;
        // A principal collision cannot replace or revive the existing subject.
        Ok(())
    })
    .await
    .unwrap();
    assert!(approve(&db, &admin, before.version, 10).await.is_err());
    assert_eq!(current(&db, 3, 11).await, before);
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn ordinary_access_revocation_and_rotation_invalidate_the_connection_and_stale_unlink() {
    use zedone_core::access_commands::{AccessScope, AccessStore, KeyVerifier};
    for rotate in [false, true] {
        let (_dir, db, admin) = setup().await;
        let before = submit(&db, 3, 1, 101).await.unwrap().unwrap().result;
        let confirmed = approve(&db, &admin, before.version, 10)
            .await
            .unwrap()
            .result;
        db.transact(move |tx| {
            if rotate {
                tx.replace_subject(
                    &sid(3),
                    KeyVerifier([99; 32]),
                    &AccessScope::administration(),
                    None,
                )?;
            } else {
                tx.revoke_subject(&sid(3))?;
            }
            Ok(())
        })
        .await
        .unwrap();
        let now = current(&db, 3, 11).await;
        assert_eq!(now.status, PairingStatus::Unlinked);
        let a = admin.clone();
        let stale = db
            .transact(move |tx| {
                Ok(pairing::unlink(
                    tx,
                    &a.credential(),
                    &TestClock(103),
                    &CommandRequest {
                        id: [12; 16],
                        operation: "pairing.unlink".into(),
                        parameters: UnlinkPairing {
                            request: sid(3),
                            expected: confirmed.version,
                        },
                    },
                )?)
            })
            .await;
        assert!(matches!(stale,Err(StorageError::Rejected(e)) if e.code==ErrorCode::StaleState));
        assert!(approve(&db, &admin, now.version, 13).await.is_err());
        db.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn confirmation_uses_ordinary_key_expiry_and_does_not_grant_space_access_to_admin() {
    use zedone_core::{
        access::{RequiredAccess, SpacePermission, with_authorized},
        access_commands::AccessStore,
    };
    let (_dir, db, admin) = setup().await;
    let recipient = Arc::new(crate::access::AccessKey::generate().unwrap());
    let key = recipient.clone();
    let before = db
        .transact(move |tx| {
            tx.create_document(&sid(90))?;
            let mut r = submit_request(3);
            r.parameters.access_verifier = key.registration_verifier();
            Ok(pairing::request(
                tx,
                &TestClock(101),
                &r,
                InvitationVerifier([1; 32]),
                PendingVerifier([3; 32]),
            )?)
        })
        .await
        .unwrap()
        .unwrap()
        .result;
    let a = admin.clone();
    db.transact(move |tx| {
        let mut r = confirmation(before.version, 10);
        r.parameters.expires_at_unix_ms = Some(200);
        Ok(pairing::confirm(tx, &a.credential(), &TestClock(102), &r)?)
    })
    .await
    .unwrap();
    let key = recipient.clone();
    db.transact(move |tx| {
        with_authorized(
            tx,
            &key.credential(),
            &[RequiredAccess::ManageDevices],
            &TestClock(199),
            |_, _| Ok(()),
        )?;
        assert_eq!(
            with_authorized(
                tx,
                &key.credential(),
                &[RequiredAccess::ManageDevices],
                &TestClock(200),
                |_, _| Ok(())
            )
            .unwrap_err()
            .code,
            ErrorCode::AccessDenied
        );
        assert_eq!(
            with_authorized(
                tx,
                &key.credential(),
                &[RequiredAccess::Space {
                    id: &sid(90),
                    permission: SpacePermission::Read
                }],
                &TestClock(199),
                |_, _| Ok(())
            )
            .unwrap_err()
            .code,
            ErrorCode::AccessDenied
        );
        assert_eq!(
            tx.access_snapshot(&sid(3))?.unwrap().expires_at_unix_ms,
            Some(200)
        );
        Ok(())
    })
    .await
    .unwrap();
    // Expiration is not an implicit unlink or permission extension.
    assert_eq!(current(&db, 3, 11).await.status, PairingStatus::Confirmed);
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn unlink_cannot_remove_the_last_administrator_and_rolls_back_on_receipt_failure() {
    use zedone_core::access_commands::AccessStore;
    for last_admin in [false, true] {
        let (_dir, db, admin) = setup().await;
        let pending = submit(&db, 3, 1, 101).await.unwrap().unwrap().result;
        let confirmed = approve(&db, &admin, pending.version, 10)
            .await
            .unwrap()
            .result;
        let a = admin.clone();
        db.transact(move |tx| {
            if last_admin {tx.revoke_subject(a.subject())?;} else {
                tx.connection().execute_batch("CREATE TRIGGER fail_unlink_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT,'test'); END;")?;
            }
            Ok(())
        }).await.unwrap();
        let result = db
            .transact(move |tx| {
                Ok(pairing::peer_unlink(
                    tx,
                    &CommandRequest {
                        id: [11; 16],
                        operation: "pairing.unlink".into(),
                        parameters: UnlinkPairing {
                            request: sid(3),
                            expected: confirmed.version,
                        },
                    },
                    PendingVerifier([3; 32]),
                )?)
            })
            .await;
        assert!(result.is_err());
        if !last_admin {
            db.transact(|tx| {
                tx.connection()
                    .execute_batch("DROP TRIGGER fail_unlink_receipt")?;
                Ok(())
            })
            .await
            .unwrap();
        }
        assert_eq!(current(&db, 3, 12).await, confirmed);
        db.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn revoked_administrator_cannot_replay_a_confirmation_receipt() {
    use zedone_core::access_commands::AccessStore;
    let (_dir, db, admin) = setup().await;
    let pending = submit(&db, 3, 1, 101).await.unwrap().unwrap().result;
    approve(&db, &admin, pending.version, 10).await.unwrap();
    let a = admin.clone();
    db.transact(move |tx| {
        tx.revoke_subject(a.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    assert!(
        matches!(approve(&db,&admin,pending.version,10).await,Err(StorageError::Rejected(e)) if e.code==ErrorCode::AccessDenied)
    );
    assert_eq!(current(&db, 3, 11).await.status, PairingStatus::Confirmed);
    db.shutdown().await.unwrap();
}
