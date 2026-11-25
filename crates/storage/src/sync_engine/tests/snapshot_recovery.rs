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
    for conflict in [false, true] {
        let directory = tempfile::tempdir().unwrap();
        let source = Storage::open(directory.path().join("source"))
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
                    assert_eq!(current.titles.contains(&"Original".into()), conflict);
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
