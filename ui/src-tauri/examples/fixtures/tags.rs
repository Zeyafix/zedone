use std::sync::Arc;
use zedone_core::{
    cards::{CardStore, CreateCard},
    spaces::SpaceStore,
    tags::{CreateTag, SetCardTag, TagStore},
};
use zedone_storage::{Storage, access::PendingAccess};
#[path = "tag_collisions.rs"]
mod collisions;

fn id(n: u8) -> String {
    format!("{n:032x}")
}
async fn assign(storage: &Storage, card: u8, tag: u8, present: bool) {
    storage
        .transact(move |tx| {
            let c = tx.card_snapshot(&id(201), &id(card))?;
            let t = tx.tag_snapshot(&id(201), &id(tag))?;
            tx.set_card_tag(&SetCardTag {
                space: id(201),
                card: id(card),
                tag: id(tag),
                present,
                expected: c.version,
                expected_tag: t.version,
            })?;
            Ok(())
        })
        .await
        .unwrap();
}
pub async fn seed(storage: &Storage, access: &Arc<PendingAccess>) {
    let owner = access.clone();
    storage
        .transact(move |tx| {
            use zedone_core::columns::ColumnStore;
            tx.create_space(&id(201), "Согласование тегов", owner.subject())?;
            tx.create_column(&id(201), &id(202), "Карточки")?;
            for (card, title) in [(203, "Три отдельных тега"), (209, "Спорное назначение")]
            {
                tx.create_card(
                    &CreateCard {
                        space: id(201),
                        column: id(202),
                        card: id(card),
                        title: title.into(),
                        description: "Сохранённое описание".into(),
                    },
                    owner.subject(),
                    &id(card),
                )?;
            }
            tx.create_tag(&CreateTag {
                space: id(201),
                tag: id(208),
                name: "Связь".into(),
            })?;
            Ok(())
        })
        .await
        .unwrap();
    assign(storage, 209, 208, true).await;
    let initial = storage
        .transact(|tx| {
            zedone_storage::sync_engine::page(tx.read().document(&id(201))?, &id(201), &vec![])
        })
        .await
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let remote = Storage::open(directory.path().join("peer.sqlite"))
        .await
        .unwrap();
    let owner = access.clone();
    remote
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let selection = tx.read().sync_selection(&id(201), &id(204))?;
            tx.set_sync_selected(&id(201), &id(204), true, selection.version)?;
            tx.receive_sync_page(&id(201), &id(204), owner.subject(), &initial)?;
            tx.create_tag(&CreateTag {
                space: id(201),
                tag: id(207),
                name: "РАБОТА".into(),
            })?;
            Ok(())
        })
        .await
        .unwrap();
    assign(&remote, 203, 207, true).await;
    assign(&remote, 209, 208, false).await;
    assign(&remote, 209, 208, true).await;
    storage
        .transact(|tx| {
            tx.create_tag(&CreateTag {
                space: id(201),
                tag: id(205),
                name: "Работа".into(),
            })?;
            Ok(())
        })
        .await
        .unwrap();
    assign(storage, 203, 205, true).await;
    storage
        .transact(|tx| {
            tx.rename_tag(&id(201), &id(205), "Личное")?;
            tx.create_tag(&CreateTag {
                space: id(201),
                tag: id(206),
                name: "Работа".into(),
            })?;
            Ok(())
        })
        .await
        .unwrap();
    assign(storage, 203, 206, true).await;
    assign(storage, 209, 208, false).await;
    let page = remote
        .transact(|tx| {
            zedone_storage::sync_engine::page(tx.read().document(&id(201))?, &id(201), &vec![])
        })
        .await
        .unwrap();
    storage
        .transact(move |tx| {
            use zedone_core::sync::SyncStore;
            tx.push_sync(&id(201), "fixture", &vec![], &page.changes)?;
            assert_eq!(
                tx.tags(&id(201))?
                    .iter()
                    .filter(|t| t.name_group.len() == 3)
                    .count(),
                3
            );
            assert_eq!(
                tx.card_snapshot(&id(201), &id(209))?.state,
                zedone_core::spaces::SpaceState::Conflicted
            );
            Ok(())
        })
        .await
        .unwrap();
    remote.shutdown().await.unwrap();
    collisions::seed(storage, access).await;
}
