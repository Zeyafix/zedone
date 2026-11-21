use super::*;
use std::collections::BTreeMap;
use zedone_core::{
    access_commands::{AccessMode, AccessScope, AccessStore},
    spaces::{SpaceState, SpaceStore},
    sync::{self, CatalogSync, PullSync, PushSync, SyncStore},
};

fn request<P: zedone_core::command::Command>(id: u8, parameters: P) -> CommandRequest<P> {
    CommandRequest {
        id: [id; 16],
        operation: P::NAME.into(),
        parameters,
    }
}

#[tokio::test]
async fn long_offline_pairing_keeps_selection_progress_and_independent_changes_until_explicit_revocation()
 {
    for years in [1, 50] {
        let (dir, db, admin) = setup().await;
        let recipient = Arc::new(crate::access::AccessKey::generate().unwrap());
        let owner = admin.clone();
        let key = recipient.clone();
        let initial = db
            .transact(move |tx| {
                tx.create_space(&sid(90), "Initial", owner.subject())?;
                let mut r = submit_request(3);
                r.parameters.access_verifier = key.registration_verifier();
                let pending = pairing::request(
                    tx,
                    &TestClock(101),
                    &r,
                    InvitationVerifier([1; 32]),
                    PendingVerifier([3; 32]),
                )?
                .unwrap()
                .result;
                let mut r = confirmation(pending.version, 10);
                r.parameters.scope =
                    AccessScope::spaces(BTreeMap::from([(sid(90), AccessMode::ReadWrite)]));
                pairing::confirm(tx, &owner.credential(), &TestClock(102), &r)?;
                let control = tx.sync_control(&sid(90), &sid(3))?;
                sync::select(
                    tx,
                    &owner.credential(),
                    &TestClock(102),
                    &request(
                        11,
                        sync::SelectSync {
                            space: sid(90),
                            pairing: sid(3),
                            selected: true,
                            expected: control.selection_version,
                        },
                    ),
                )?;
                Ok(sync::pull(
                    tx,
                    &key.credential(),
                    &TestClock(102),
                    &request(
                        12,
                        PullSync {
                            protocol: sync::PROTOCOL,
                            space: sid(90),
                            known: vec![],
                        },
                    ),
                )?
                .result)
            })
            .await
            .unwrap();
        let remote = Storage::open(dir.path().join("remote")).await.unwrap();
        let owner = admin.clone();
        let copy = initial.clone();
        remote
            .transact(move |tx| {
                tx.bootstrap_access(&owner)?;
                let control = tx.sync_control(&sid(90), &sid(3))?;
                tx.set_sync_selected(&sid(90), &sid(3), true, control.selection_version)?;
                tx.receive_sync_page(&sid(90), &sid(3), owner.subject(), &copy)?;
                tx.rename_space(&sid(90), "Saved on remote")?;
                Ok(())
            })
            .await
            .unwrap();
        db.transact(|tx| {
            tx.rename_space(&sid(90), "Saved on host")?;
            Ok(())
        })
        .await
        .unwrap();
        let before = db
            .transact(|tx| {
                Ok((
                    tx.sync_control(&sid(90), &sid(3))?,
                    tx.sync_known(&sid(90), &sid(3))?,
                ))
            })
            .await
            .unwrap();
        db.shutdown().await.unwrap();
        remote.shutdown().await.unwrap();
        let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
        let remote = Storage::open(dir.path().join("remote")).await.unwrap();
        let now = 102 + years * 365 * 24 * 60 * 60 * 1000;
        assert_eq!(current(&db, 3, 13).await.status, PairingStatus::Confirmed);
        let key = recipient.clone();
        let known = initial.heads.clone();
        let incoming = db
            .transact(move |tx| {
                assert_eq!(
                    (
                        tx.sync_control(&sid(90), &sid(3))?,
                        tx.sync_known(&sid(90), &sid(3))?
                    ),
                    before
                );
                let catalog = sync::catalog(
                    tx,
                    &key.credential(),
                    &TestClock(now),
                    &request(14, CatalogSync {}),
                )?
                .result;
                assert_eq!(catalog.pairing, sid(3));
                assert_eq!(catalog.spaces.len(), 1);
                assert!(catalog.spaces[0].selected && !catalog.spaces[0].paused);
                Ok(sync::pull(
                    tx,
                    &key.credential(),
                    &TestClock(now),
                    &request(
                        15,
                        PullSync {
                            protocol: sync::PROTOCOL,
                            space: sid(90),
                            known,
                        },
                    ),
                )?
                .result)
            })
            .await
            .unwrap();
        let owner = admin.clone();
        let outgoing = remote
            .transact(move |tx| {
                tx.receive_sync_page(&sid(90), &sid(3), owner.subject(), &incoming)?;
                crate::sync_engine::page(tx.read().document(&sid(90))?, &sid(90), &initial.heads)
            })
            .await
            .unwrap();
        let key = recipient.clone();
        let expected = outgoing.heads.clone();
        db.transact(move |tx| {
            let push = request(
                16,
                PushSync {
                    protocol: sync::PROTOCOL,
                    space: sid(90),
                    base: outgoing.base,
                    changes: outgoing.changes,
                },
            );
            for _ in 0..3 {
                let ack = sync::push(tx, &key.credential(), &TestClock(now), &push)?.result;
                assert_eq!(ack.heads, expected);
                assert!(ack.complete);
            }
            assert_eq!(tx.sync_known(&sid(90), &sid(3))?, expected);
            Ok(())
        })
        .await
        .unwrap();
        let snapshot = db
            .transact(|tx| Ok(tx.space_snapshot(&sid(90))?))
            .await
            .unwrap();
        assert_eq!(snapshot.state, SpaceState::Active);
        assert_eq!(snapshot.names, ["Saved on host", "Saved on remote"]);
        assert_eq!(
            snapshot,
            remote
                .transact(|tx| Ok(tx.space_snapshot(&sid(90))?))
                .await
                .unwrap()
        );
        let key = recipient.clone();
        db.transact(move |tx| {
            tx.revoke_subject(&sid(3))?;
            Ok(())
        })
        .await
        .unwrap();
        let denied = db
            .transact(move |tx| {
                Ok(sync::pull(
                    tx,
                    &key.credential(),
                    &TestClock(now),
                    &request(
                        17,
                        PullSync {
                            protocol: sync::PROTOCOL,
                            space: sid(90),
                            known: vec![],
                        },
                    ),
                )?)
            })
            .await
            .unwrap_err();
        assert!(matches!(denied, StorageError::Rejected(e) if e.code == ErrorCode::AccessDenied));
        assert_eq!(current(&db, 3, 18).await.status, PairingStatus::Unlinked);
        assert_eq!(
            snapshot,
            db.transact(|tx| Ok(tx.space_snapshot(&sid(90))?))
                .await
                .unwrap()
        );
        db.shutdown().await.unwrap();
        remote.shutdown().await.unwrap();
    }
}
