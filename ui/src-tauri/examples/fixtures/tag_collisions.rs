use super::*;
use zedone_core::{columns::ColumnStore, sync::SyncStore};

const SPACE: u8 = 221;
async fn assign_tag(db: &Storage, card: u8, tag: u8) {
    db.transact(move |tx| {
        let c = tx.card_snapshot(&id(SPACE), &id(card))?;
        let t = tx.tag_snapshot(&id(SPACE), &id(tag))?;
        tx.set_card_tag(&SetCardTag {
            space: id(SPACE),
            card: id(card),
            tag: id(tag),
            present: true,
            expected: c.version,
            expected_tag: t.version,
        })?;
        Ok(())
    })
    .await
    .unwrap();
}

pub(super) async fn seed(storage: &Storage, access: &Arc<PendingAccess>) {
    let owner = access.clone();
    storage
        .transact(move |tx| {
            tx.create_space(&id(SPACE), "Коллизии тегов", owner.subject())?;
            tx.create_column(&id(SPACE), &id(222), "Карточки")?;
            for (card, title) in [
                (223, "Два переименования"),
                (224, "Переименование и создание"),
                (225, "Удаление и назначение"),
            ] {
                tx.create_card(
                    &CreateCard {
                        space: id(SPACE),
                        column: id(222),
                        card: id(card),
                        title: title.into(),
                        description: "Сохранённое описание".into(),
                    },
                    owner.subject(),
                    &id(card),
                )?;
            }
            for (tag, name) in [
                (226, "Первый"),
                (227, "Второй"),
                (228, "Третий"),
                (229, "Удаляемый"),
            ] {
                tx.create_tag(&CreateTag {
                    space: id(SPACE),
                    tag: id(tag),
                    name: name.into(),
                })?;
            }
            Ok(())
        })
        .await
        .unwrap();
    for (card, tag) in [(223, 226), (223, 227), (224, 228), (224, 229)] {
        assign_tag(storage, card, tag).await;
    }
    let initial = storage
        .transact(|tx| {
            zedone_storage::sync_engine::page(tx.read().document(&id(SPACE))?, &id(SPACE), &vec![])
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
            let selection = tx.read().sync_selection(&id(SPACE), &id(204))?;
            tx.set_sync_selected(&id(SPACE), &id(204), true, selection.version)?;
            tx.receive_sync_page(&id(SPACE), &id(204), owner.subject(), &initial)?;
            tx.rename_tag(&id(SPACE), &id(227), " СОВПАДЕНИЕ 1 ")?;
            tx.create_tag(&CreateTag {
                space: id(SPACE),
                tag: id(230),
                name: " СОВПАДЕНИЕ 2 ".into(),
            })?;
            Ok(())
        })
        .await
        .unwrap();
    assign_tag(&remote, 224, 230).await;
    assign_tag(&remote, 225, 229).await;
    storage
        .transact(|tx| {
            tx.rename_tag(&id(SPACE), &id(226), "Совпадение 1")?;
            tx.rename_tag(&id(SPACE), &id(228), "Совпадение 2")?;
            tx.delete_tag(&id(SPACE), &id(229))?;
            Ok(())
        })
        .await
        .unwrap();
    let page = remote
        .transact(|tx| {
            zedone_storage::sync_engine::page(tx.read().document(&id(SPACE))?, &id(SPACE), &vec![])
        })
        .await
        .unwrap();
    storage
        .transact(move |tx| {
            tx.push_sync(&id(SPACE), "fixture", &vec![], &page.changes)?;
            Ok(())
        })
        .await
        .unwrap();
    remote.shutdown().await.unwrap();
}
