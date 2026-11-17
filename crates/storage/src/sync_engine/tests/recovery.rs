use super::*;
use crate::Stage;
use automerge::transaction::Transactable;
use zedone_core::{
    card_history::AutosaveCard,
    cards::{CardStore, CreateCard},
    columns::ColumnStore,
};

const COLUMN: &str = "cccccccccccccccccccccccccccccccc";
const CARD: &str = "dddddddddddddddddddddddddddddddd";

async fn prepared(path: &std::path::Path) -> (Storage, SyncPage) {
    let db = Storage::open(path).await.unwrap();
    let owner = PendingAccess::generate().unwrap();
    let remote = db
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            tx.create_space(SPACE, "Shared", owner.subject())?;
            tx.create_column(SPACE, COLUMN, "Column")?;
            tx.create_card(
                &CreateCard {
                    space: SPACE.into(),
                    column: COLUMN.into(),
                    card: CARD.into(),
                    title: "Before".into(),
                    description: "Saved".into(),
                },
                owner.subject(),
                CARD,
            )?;
            let remote = tx
                .read()
                .document(SPACE)?
                .clone()
                .with_actor(automerge::ActorId::from(vec![99]));
            let c = tx.read().sync_selection(SPACE, PEER)?;
            tx.set_sync_selected(SPACE, PEER, true, c.version)?;
            let base = heads(&remote);
            tx.pull_sync(SPACE, PEER, &base)?;
            tx.start_text_session(SPACE, CARD, owner.subject(), PEER)?;
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
                owner.subject(),
                PEER,
                0,
            )?;
            Ok(remote)
        })
        .await
        .unwrap();
    let mut remote = remote;
    let base = heads(&remote);
    let card = crate::cards::object(&remote, CARD).unwrap();
    let mut t = remote.transaction();
    t.put(&card, "description", "Remote saved").unwrap();
    t.commit();
    let incoming = page(&remote, SPACE, &base).unwrap();
    (db, incoming)
}

async fn fingerprint(db: &Storage) -> (Heads, Heads, usize, bool) {
    db.transact(|tx| {
        Ok((
            heads(tx.read().document(SPACE)?),
            tx.sync_known(SPACE, PEER)?,
            tx.card_history(SPACE, CARD)?.versions.len(),
            tx.connection().query_row(
                "SELECT pending_id IS NOT NULL FROM card_editing WHERE space = ?1 AND card = ?2",
                (SPACE, CARD),
                |r| r.get(0),
            )?,
        ))
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn process_exit_keeps_graph_history_and_progress_atomic_and_repeated_delivery_preserves_later_edits()
 {
    for push in [false, true] {
        for stage in [Stage::BeforeCommit, Stage::AfterCommit] {
            let dir = tempfile::tempdir().unwrap();
            let path = dir.path().join("db");
            let (db, p) = prepared(&path).await;
            let before = fingerprint(&db).await;
            db.shutdown().await.unwrap();
            let page_path = dir.path().join("page.json");
            std::fs::write(&page_path, serde_json::to_vec(&p).unwrap()).unwrap();
            let status = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "sync_engine::tests::recovery::crash_child",
                    "--nocapture",
                ])
                .env("ZEDONE_EXCHANGE_CRASH_DB", &path)
                .env("ZEDONE_EXCHANGE_CRASH_PAGE", &page_path)
                .env("ZEDONE_EXCHANGE_CRASH_STAGE", format!("{stage:?}"))
                .env("ZEDONE_EXCHANGE_CRASH_PUSH", push.to_string())
                .status()
                .unwrap();
            assert_eq!(status.code(), Some(74));
            let db = Storage::open(&path).await.unwrap();
            let after = fingerprint(&db).await;
            if stage == Stage::BeforeCommit {
                assert_eq!(after, before);
            } else {
                assert_eq!(after.1, p.heads);
                assert!(!after.3);
                assert_eq!(after.2, before.2 + 1);
            }
            let copy = p.clone();
            db.transact(move |tx| {
                tx.push_sync(SPACE, PEER, &copy.base, &copy.changes)?;
                Ok(())
            })
            .await
            .unwrap();
            db.transact(|tx| {
                tx.set_card_completed(SPACE, CARD, true)?;
                Ok(())
            })
            .await
            .unwrap();
            let saved = fingerprint(&db).await;
            for _ in 0..3 {
                let copy = p.clone();
                db.transact(move |tx| {
                    tx.push_sync(SPACE, PEER, &copy.base, &copy.changes)?;
                    Ok(())
                })
                .await
                .unwrap();
            }
            assert_eq!(fingerprint(&db).await, saved);
            assert_eq!(
                db.transact(|tx| Ok(tx.card_snapshot(SPACE, CARD)?))
                    .await
                    .unwrap()
                    .completed,
                [true]
            );
            db.shutdown().await.unwrap();
        }
    }
}

#[tokio::test]
async fn crash_child() {
    let Some(path) = std::env::var_os("ZEDONE_EXCHANGE_CRASH_DB") else {
        return;
    };
    let p: SyncPage = serde_json::from_slice(
        &std::fs::read(std::env::var_os("ZEDONE_EXCHANGE_CRASH_PAGE").unwrap()).unwrap(),
    )
    .unwrap();
    let db = Storage::open(std::path::PathBuf::from(path)).await.unwrap();
    let stage = std::env::var("ZEDONE_EXCHANGE_CRASH_STAGE").unwrap();
    let push = std::env::var("ZEDONE_EXCHANGE_CRASH_PUSH").unwrap() == "true";
    db.transact_checked(
        move |tx| {
            if push {
                tx.push_sync(SPACE, PEER, &p.base, &p.changes)?;
                Ok(())
            } else {
                tx.receive_sync_page(SPACE, PEER, "unused-existing-document", &p)
            }
        },
        move |here| {
            if format!("{here:?}") == stage {
                std::process::exit(74);
            }
            Ok(())
        },
    )
    .await
    .unwrap();
    panic!("checkpoint not reached");
}

#[tokio::test]
async fn sql_failure_and_unsupported_document_leave_local_history_and_progress_intact() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("db");
    let (db, p) = prepared(&path).await;
    let before = fingerprint(&db).await;
    let sql = rusqlite::Connection::open(&path).unwrap();
    sql.execute_batch("CREATE TRIGGER reject_progress BEFORE INSERT ON peers BEGIN SELECT RAISE(ABORT, 'fault'); END;").unwrap();
    let copy = p.clone();
    assert_eq!(
        db.transact(move |tx| {
            assert!(
                tx.push_sync(SPACE, PEER, &copy.base, &copy.changes)
                    .is_err()
            );
            Ok(())
        })
        .await,
        Err(StorageError::Unavailable)
    );
    assert_eq!(fingerprint(&db).await, before);
    sql.execute_batch("DROP TRIGGER reject_progress;").unwrap();
    let mut remote = db
        .read(|tx| {
            Ok(tx
                .document(SPACE)?
                .clone()
                .with_actor(automerge::ActorId::from(vec![98])))
        })
        .await
        .unwrap();
    let base = heads(&remote);
    let mut t = remote.transaction();
    t.put(automerge::ROOT, "format", "zedone.space.v999")
        .unwrap();
    t.commit();
    let unsupported = page(&remote, SPACE, &base).unwrap();
    assert_eq!(
        db.transact(move |tx| Ok(tx.push_sync(
            SPACE,
            PEER,
            &unsupported.base,
            &unsupported.changes
        )?))
        .await
        .unwrap_err(),
        incompatible()
    );
    assert_eq!(fingerprint(&db).await, before);
    db.transact(move |tx| {
        tx.push_sync(SPACE, PEER, &p.base, &p.changes)?;
        Ok(())
    })
    .await
    .unwrap();
    assert!(!fingerprint(&db).await.3);
    db.shutdown().await.unwrap();
}
