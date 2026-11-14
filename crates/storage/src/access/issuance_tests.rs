use super::tests::{TestClock, setup};
use super::*;
use zedone_core::{
    access_commands::{self, AccessStore, IssueAccess, ReplaceAccess},
    wire::CommandRequest,
};

fn request(pending: &PendingAccess, end: Option<i64>, id: u8) -> CommandRequest<IssueAccess> {
    CommandRequest {
        id: [id; 16],
        operation: "access.issue".into(),
        parameters: IssueAccess {
            subject: pending.subject().into(),
            verifier: pending.registration_verifier(),
            scope: AccessScope::administration(),
            expires_at_unix_ms: end,
        },
    }
}

#[tokio::test]
async fn expiry_is_exclusive_replay_does_not_extend_it_and_regrant_requires_new_key() {
    let (_dir, storage, admin, _member) = setup().await;
    let recipient = std::sync::Arc::new(PendingAccess::generate().unwrap());
    let (a, r) = (admin.clone(), recipient.clone());
    let issued = storage
        .transact(move |ctx| {
            Ok(access_commands::issue(
                ctx,
                &a.credential(),
                &TestClock(100),
                &request(&r, Some(200), 1),
            )?)
        })
        .await
        .unwrap();
    let (a, r) = (admin.clone(), recipient.clone());
    let replayed = storage
        .transact(move |ctx| {
            // Recipient expiry does not revoke the issuer's right to its receipt.
            let replayed = access_commands::issue(
                ctx,
                &a.credential(),
                &TestClock(200),
                &request(&r, Some(200), 1),
            )?;
            assert!(replayed.replayed);
            assert_eq!(replayed.result.expires_at_unix_ms, Some(200));
            Ok(replayed)
        })
        .await
        .unwrap();
    assert_eq!(issued.result, replayed.result);
    let r = recipient.clone();
    storage
        .transact(move |ctx| {
            with_authorized(
                ctx,
                &r.credential(),
                &[RequiredAccess::ManageAccess],
                &TestClock(199),
                |_, _| Ok(()),
            )?;
            assert_eq!(
                with_authorized(
                    ctx,
                    &r.credential(),
                    &[RequiredAccess::ManageAccess],
                    &TestClock(200),
                    |_, _| Ok(())
                )
                .unwrap_err()
                .code,
                ErrorCode::AccessDenied
            );
            Ok(())
        })
        .await
        .unwrap();
    let fresh = std::sync::Arc::new(PendingAccess::generate().unwrap());
    let (a, r, f) = (admin.clone(), recipient.clone(), fresh.clone());
    storage
        .transact(move |ctx| {
            let current = ctx.access_snapshot(r.subject())?.unwrap();
            let replacement = CommandRequest {
                id: [2; 16],
                operation: "access.replace".into(),
                parameters: ReplaceAccess {
                    subject: r.subject().into(),
                    verifier: f.registration_verifier(),
                    scope: AccessScope::administration(),
                    expires_at_unix_ms: None,
                    expected: current.version,
                },
            };
            access_commands::replace(ctx, &a.credential(), &TestClock(200), &replacement)?;
            assert!(ctx.resolve_access(&r.credential())?.is_none());
            with_authorized(
                ctx,
                &f.credential(),
                &[RequiredAccess::ManageAccess],
                &TestClock(201),
                |_, _| Ok(()),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn failed_expiry_and_receipt_insert_roll_back_all_issuance_and_allow_exact_retry() {
    let (_dir, storage, admin, _member) = setup().await;
    let recipient = std::sync::Arc::new(PendingAccess::generate().unwrap());
    for end in [
        Some(-1),
        Some(100),
        Some(99),
        Some(i64::MAX),
        Some(access_commands::MAX_EXPIRY_UNIX_MS + 1),
    ] {
        let (a, r) = (admin.clone(), recipient.clone());
        let result = storage
            .transact(move |ctx| {
                Ok(access_commands::issue(
                    ctx,
                    &a.credential(),
                    &TestClock(100),
                    &request(&r, end, 1),
                )?)
            })
            .await;
        assert!(matches!(
            result,
            Err(StorageError::Rejected(CommandError {
                code: ErrorCode::InvalidRequest
            }))
        ));
    }
    storage.transact(|ctx| {
        ctx.connection().execute_batch("CREATE TRIGGER deny_issuance_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT,'synthetic'); END;")?;
        Ok(())
    }).await.unwrap();
    let (a, r) = (admin.clone(), recipient.clone());
    assert!(
        storage
            .transact(move |ctx| Ok(access_commands::issue(
                ctx,
                &a.credential(),
                &TestClock(100),
                &request(&r, None, 1)
            )?))
            .await
            .is_err()
    );
    let r = recipient.clone();
    storage
        .transact(move |ctx| {
            assert!(ctx.access_snapshot(r.subject())?.is_none());
            assert!(ctx.resolve_access(&r.credential())?.is_none());
            ctx.connection()
                .execute_batch("DROP TRIGGER deny_issuance_receipt;")?;
            Ok(())
        })
        .await
        .unwrap();
    let (a, r) = (admin.clone(), recipient.clone());
    let issued = storage
        .transact(move |ctx| {
            Ok(access_commands::issue(
                ctx,
                &a.credential(),
                &TestClock(100),
                &request(&r, None, 1),
            )?)
        })
        .await
        .unwrap();
    assert!(!issued.replayed);
    assert_eq!(issued.result.subject, recipient.subject());
    let recipient = PendingAccess::generate().unwrap();
    storage
        .transact(move |ctx| {
            let issued = access_commands::issue(
                ctx,
                &admin.credential(),
                &TestClock(100),
                &request(&recipient, Some(access_commands::MAX_EXPIRY_UNIX_MS), 2),
            )?;
            assert_eq!(
                issued.result.expires_at_unix_ms,
                Some(access_commands::MAX_EXPIRY_UNIX_MS)
            );
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}
