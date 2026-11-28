use super::*;
use zedone_core::{
    cards::{CardStore, CreateCard, EditCard},
    columns::ColumnStore,
    recovery::{RecoveryStore, RestoreSnapshot},
};
const COLUMN: &str = "cccccccccccccccccccccccccccccccc";
const CARD: &str = "dddddddddddddddddddddddddddddddd";
async fn send(source: &Storage, receiver: &Storage, subject: String) -> StorageResult<()> {
    let known = receiver.transact(|ctx| ctx.sync_known(SPACE, PEER)).await?;
    let page = source
        .transact(move |ctx| page(ctx.read().document(SPACE)?, SPACE, &known))
        .await?;
    receiver
        .transact(move |ctx| ctx.receive_sync_page(SPACE, PEER, &subject, &page))
        .await
}
#[tokio::test]
async fn restored_snapshot_is_exchanged_as_new_changes_and_preserves_compatible_and_conflicting_peer_edits()
 {
    for (reinstall, conflict) in [(false, false), (false, true), (true, false), (true, true)] {
        let directory = tempfile::tempdir().unwrap();
        let mut source = Storage::open(directory.path().join("source"))
            .await
            .unwrap();
        let peer = Storage::open(directory.path().join("peer")).await.unwrap();
        let admin = Arc::new(PendingAccess::generate().unwrap());
        let a = admin.clone();
        source
            .transact(move |ctx| {
                ctx.bootstrap_access(&a)?;
                ctx.create_space(SPACE, "Space", a.subject())?;
                ctx.create_column(SPACE, COLUMN, "Column")?;
                ctx.create_card(
                    &CreateCard {
                        space: SPACE.into(),
                        column: COLUMN.into(),
                        card: CARD.into(),
                        title: "Original".into(),
                        description: "Before".into(),
                    },
                    a.subject(),
                    PEER,
                )?;
                let policy = ctx.read().sync_selection(SPACE, PEER)?;
                ctx.set_sync_selected(SPACE, PEER, true, policy.version)?;
                Ok(())
            })
            .await
            .unwrap();
        let a = admin.clone();
        peer.transact(move |ctx| {
            ctx.bootstrap_access(&a)?;
            let policy = ctx.read().sync_selection(SPACE, PEER)?;
            ctx.set_sync_selected(SPACE, PEER, true, policy.version)?;
            Ok(())
        })
        .await
        .unwrap();
        send(&source, &peer, admin.subject().into()).await.unwrap();
        let limits = crate::snapshot::SnapshotLimits {
            max_bytes: 1024 * 1024,
            max_changes: 10000,
        };
        let archive: String = source
            .export_snapshot(SPACE.into(), 42, limits)
            .await
            .unwrap()
            .encode(limits)
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let a = admin.clone();
        source
            .transact(move |ctx| {
                let expected = ctx.card_snapshot(SPACE, CARD)?.version;
                ctx.edit_card(
                    &EditCard {
                        space: SPACE.into(),
                        card: CARD.into(),
                        title: if conflict { "Local" } else { "Original" }.into(),
                        description: "Local description".into(),
                        expected,
                    },
                    a.subject(),
                    "11111111111111111111111111111111",
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let a = admin.clone();
        peer.transact(move |ctx| {
            let expected = ctx.card_snapshot(SPACE, CARD)?.version;
            ctx.edit_card(
                &EditCard {
                    space: SPACE.into(),
                    card: CARD.into(),
                    title: "Remote".into(),
                    description: "Before".into(),
                    expected,
                },
                a.subject(),
                "22222222222222222222222222222222",
            )?;
            Ok(())
        })
        .await
        .unwrap();
        if reinstall {
            use zedone_core::recovery::missing::{MissingRecoveryStore, RestoreMissingSnapshot};
            source.shutdown().await.unwrap();
            source = Storage::open(directory.path().join("reinstalled"))
                .await
                .unwrap();
            let fresh = Arc::new(PendingAccess::generate().unwrap());
            source
                .transact(move |ctx| {
                    ctx.bootstrap_access(&fresh)?;
                    let expected = ctx.missing_recovery_state(&archive)?.version;
                    ctx.restore_missing_snapshot(
                        &RestoreMissingSnapshot {
                            space: SPACE.into(),
                            archive,
                            expected,
                        },
                        fresh.subject(),
                    )?;
                    assert!(ctx.recovery_state(SPACE)?.paused);
                    let selection = ctx.read().sync_selection(SPACE, PEER)?;
                    assert!(!selection.selected);
                    ctx.set_sync_selected(SPACE, PEER, true, selection.version)?;
                    assert!(ctx.recovery_state(SPACE)?.paused);
                    Ok(())
                })
                .await
                .unwrap();
            confirm_reinstalled_device(&peer, &admin).await;
            source.shutdown().await.unwrap();
            source = Storage::open(directory.path().join("reinstalled"))
                .await
                .unwrap();
        } else {
            source
                .transact(move |ctx| {
                    let expected = ctx.recovery_state(SPACE)?.version;
                    ctx.restore_snapshot(
                        &RestoreSnapshot {
                            space: SPACE.into(),
                            archive,
                            expected,
                        },
                        50,
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        assert!(
            source
                .transact(|ctx| Ok(ctx.recovery_state(SPACE)?.paused))
                .await
                .unwrap()
        );
        source
            .transact(|ctx| {
                let p = ctx.read().space_sync_control(SPACE)?;
                ctx.set_space_sync_paused(SPACE, false, p.version)?;
                Ok(())
            })
            .await
            .unwrap();
        send(&source, &peer, admin.subject().into()).await.unwrap();
        send(&peer, &source, admin.subject().into()).await.unwrap();
        for storage in [&source, &peer] {
            storage
                .transact(move |ctx| {
                    let current = ctx.card_snapshot(SPACE, CARD)?;
                    assert!(current.titles.contains(&"Remote".into()));
                    assert_eq!(
                        current.titles.contains(&"Original".into()),
                        conflict && !reinstall
                    );
                    assert_eq!(current.descriptions, ["Before"]);
                    assert!(
                        ctx.card_history(SPACE, CARD)?
                            .versions
                            .iter()
                            .any(|v| v.title == "Remote")
                    );
                    Ok(())
                })
                .await
                .unwrap();
        }
        source.shutdown().await.unwrap();
        peer.shutdown().await.unwrap();
    }
}

async fn confirm_reinstalled_device(peer: &Storage, admin: &Arc<PendingAccess>) {
    use serde_json::json;
    use zedone_core::{
        access::{AccessTransaction, Clock},
        command::CommandResult,
        invitations::{self, InvitationVerifier},
        pairing::{self, PendingVerifier},
        wire::decode_request,
    };
    struct Now;
    impl Clock for Now {
        fn now(&self) -> CommandResult<std::time::SystemTime> {
            Ok(std::time::UNIX_EPOCH + std::time::Duration::from_millis(100))
        }
    }
    let a = admin.clone();
    peer.transact(move |ctx| {
        invitations::create(ctx, &a.credential(), &Now, &decode_request::<invitations::CreateInvitation>(&serde_json::to_vec(&json!({"id":vec![70;16],"operation":"invitation.create","parameters":{"invitation":"ffffffffffffffffffffffffffffffff","verifier":vec![77;32]}})).unwrap())?)?;
        let pending = pairing::request(ctx, &Now, &decode_request::<pairing::RequestPairing>(&serde_json::to_vec(&json!({"id":vec![71;16],"operation":"pairing.request","parameters":{"request":PEER,"invitation":"ffffffffffffffffffffffffffffffff","label":"Reinstalled device","access_verifier":vec![78;32]}})).unwrap())?, InvitationVerifier([77;32]), PendingVerifier([79;32]))?.unwrap().result;
        assert_eq!(pending.status, pairing::PairingStatus::Pending);
        let before = ctx.resolve_access(&a.credential())?.unwrap().grant;
        let confirmed = pairing::confirm(ctx, &a.credential(), &Now, &decode_request::<pairing::ConfirmPairing>(&serde_json::to_vec(&json!({"id":vec![72;16],"operation":"pairing.confirm","parameters":{"request":PEER,"expected":pending.version,"scope":{"kind":"spaces","spaces":{SPACE:"read_write"}},"expires_at_unix_ms":null}})).unwrap())?)?.result;
        assert_eq!(confirmed.status, pairing::PairingStatus::Confirmed);
        assert_eq!(before.scope.spaces(), ctx.resolve_access(&a.credential())?.unwrap().grant.scope.spaces());
        assert_eq!(before.scope.administration(), ctx.resolve_access(&a.credential())?.unwrap().grant.scope.administration());
        Ok(())
    }).await.unwrap();
}
