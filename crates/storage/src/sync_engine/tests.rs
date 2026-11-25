use super::*;
use crate::{Storage, access::PendingAccess};
use std::sync::Arc;
const SPACE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const PEER: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
mod recovery;
mod snapshot_recovery;

#[tokio::test]
async fn incoming_changes_only_finish_pending_series_of_affected_cards() {
    use automerge::transaction::Transactable;
    use zedone_core::{
        card_history::AutosaveCard,
        cards::{CardStore, CreateCard},
        columns::ColumnStore,
    };
    const COLUMN: &str = "cccccccccccccccccccccccccccccccc";
    const CARD: &str = "dddddddddddddddddddddddddddddddd";
    const OTHER: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("db")).await.unwrap();
    let owner = Arc::new(PendingAccess::generate().unwrap());
    let a = owner.clone();
    let mut remote = db
        .transact(move |tx| {
            tx.bootstrap_access(&a)?;
            tx.create_space(SPACE, "Space", a.subject())?;
            tx.create_column(SPACE, COLUMN, "Column")?;
            for card in [CARD, OTHER] {
                tx.create_card(
                    &CreateCard {
                        space: SPACE.into(),
                        column: COLUMN.into(),
                        card: card.into(),
                        title: "Before".into(),
                        description: "Saved".into(),
                    },
                    a.subject(),
                    card,
                )?;
            }
            let remote = tx
                .read()
                .document(SPACE)?
                .clone()
                .with_actor(automerge::ActorId::from(vec![99]));
            tx.start_text_session(SPACE, CARD, a.subject(), PEER)?;
            let expected = tx.card_snapshot(SPACE, CARD)?.version;
            tx.save_text_series(
                &AutosaveCard {
                    space: SPACE.into(),
                    card: CARD.into(),
                    session: PEER.into(),
                    title: "Local typing".into(),
                    description: "Saved".into(),
                    expected,
                },
                a.subject(),
                PEER,
                0,
            )?;
            Ok(remote)
        })
        .await
        .unwrap();
    for card in [OTHER, CARD] {
        let base = heads(&remote);
        let object = crate::cards::object(&remote, card).unwrap();
        {
            let mut t = remote.transaction();
            t.put(&object, "title", "Remote edit").unwrap();
            t.commit();
        }
        let changes = remote
            .get_changes(&hashes(&base))
            .iter()
            .map(encode)
            .collect::<Vec<_>>();
        db.transact(move |tx| {
            tx.push_sync(SPACE, PEER, &base, &changes)?;
            Ok(())
        })
        .await
        .unwrap();
        let pending = db
            .transact(|tx| {
                Ok(tx.connection().query_row(
                    "SELECT pending_id FROM card_editing WHERE space = ?1 AND card = ?2",
                    (SPACE, CARD),
                    |r| r.get::<_, Option<String>>(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(pending.is_some(), card == OTHER);
    }
    let history = db
        .transact(|tx| Ok(tx.card_history(SPACE, CARD)?))
        .await
        .unwrap();
    assert!(history.versions.iter().any(|v| v.title == "Local typing"));
}

#[tokio::test]
async fn staged_pages_are_hidden_resumable_atomic_and_use_a_distinct_local_actor() {
    let dir = tempfile::tempdir().unwrap();
    let source = Storage::open(dir.path().join("source")).await.unwrap();
    let owner = Arc::new(PendingAccess::generate().unwrap());
    let a = owner.clone();
    source
        .transact(move |tx| {
            tx.bootstrap_access(&a)?;
            tx.create_space(SPACE, "Initial", a.subject())?;
            for i in 0..270 {
                tx.rename_space(SPACE, &format!("Name {i}"))?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let receiver = Storage::open(dir.path().join("receiver")).await.unwrap();
    let a = owner.clone();
    receiver
        .transact(move |tx| {
            tx.bootstrap_access(&a)?;
            let c = tx.read().sync_selection(SPACE, PEER)?;
            tx.set_sync_selected(SPACE, PEER, true, c.version)?;
            Ok(())
        })
        .await
        .unwrap();
    let first = source
        .transact(|tx| page(tx.read().document(SPACE)?, SPACE, &vec![]))
        .await
        .unwrap();
    assert!(!first.complete);
    assert_eq!(first.changes.len(), 256);
    let a = owner.clone();
    let original = first.clone();
    receiver
        .transact(move |tx| tx.receive_sync_page(SPACE, PEER, a.subject(), &first))
        .await
        .unwrap();
    assert!(
        !receiver
            .transact(|tx| Ok(tx.sync_space_exists(SPACE)?))
            .await
            .unwrap()
    );
    receiver.shutdown().await.unwrap();
    let receiver = Storage::open(dir.path().join("receiver")).await.unwrap();
    let known = receiver
        .transact(|tx| tx.sync_known(SPACE, PEER))
        .await
        .unwrap();
    assert_eq!(known, original.heads);
    let last = source
        .transact(move |tx| page(tx.read().document(SPACE)?, SPACE, &known))
        .await
        .unwrap();
    assert!(last.complete);
    let a = owner.clone();
    let failed = last.clone();
    assert_eq!(
        receiver
            .transact_checked(
                move |tx| tx.receive_sync_page(SPACE, PEER, a.subject(), &failed),
                |stage| if stage == crate::Stage::BeforeCommit {
                    Err(StorageError::Unavailable)
                } else {
                    Ok(())
                }
            )
            .await,
        Err(StorageError::Unavailable)
    );
    assert!(
        !receiver
            .transact(|tx| Ok(tx.sync_space_exists(SPACE)?))
            .await
            .unwrap()
    );
    let a = owner.clone();
    receiver
        .transact(move |tx| tx.receive_sync_page(SPACE, PEER, a.subject(), &last))
        .await
        .unwrap();
    let snapshot = receiver
        .transact(|tx| Ok(tx.space_snapshot(SPACE)?))
        .await
        .unwrap();
    assert_eq!(snapshot.names, ["Name 269"]);
    let source_actor = source
        .read(|tx| Ok(tx.document(SPACE)?.get_actor().to_bytes().to_vec()))
        .await
        .unwrap();
    let received_actor = receiver
        .read(|tx| Ok(tx.document(SPACE)?.get_actor().to_bytes().to_vec()))
        .await
        .unwrap();
    assert_ne!(source_actor, received_actor);
    assert_eq!(
        receiver
            .transact(|tx| Ok(tx.connection().query_row(
                "SELECT count(*) FROM sync_incoming",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn forged_frontiers_malformed_changes_and_unknown_format_never_create_documents() {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("receiver")).await.unwrap();
    let owner = Arc::new(PendingAccess::generate().unwrap());
    let a = owner.clone();
    db.transact(move |tx| {
        tx.bootstrap_access(&a)?;
        let c = tx.read().sync_selection(SPACE, PEER)?;
        tx.set_sync_selected(SPACE, PEER, true, c.version)?;
        Ok(())
    })
    .await
    .unwrap();
    for changes in [vec!["not hex".into()], vec!["00".into()], vec![]] {
        let a = owner.clone();
        let p = SyncPage {
            protocol: PROTOCOL,
            space: SPACE.into(),
            base: vec![],
            heads: vec![[1; 32]],
            changes,
            complete: true,
        };
        assert!(
            db.transact(move |tx| tx.receive_sync_page(SPACE, PEER, a.subject(), &p))
                .await
                .is_err()
        );
        assert!(
            !db.transact(|tx| Ok(tx.sync_space_exists(SPACE)?))
                .await
                .unwrap()
        );
    }
    let mut graph = Automerge::new();
    {
        use automerge::transaction::Transactable;
        let mut t = graph.transaction();
        t.put(automerge::ROOT, "format", "foreign").unwrap();
        t.commit();
    }
    let p = page(&graph, SPACE, &vec![]).unwrap();
    let a = owner.clone();
    assert!(
        db.transact(move |tx| tx.receive_sync_page(SPACE, PEER, a.subject(), &p))
            .await
            .is_err()
    );
    assert!(
        !db.transact(|tx| Ok(tx.sync_space_exists(SPACE)?))
            .await
            .unwrap()
    );
}
