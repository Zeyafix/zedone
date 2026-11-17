use std::sync::Arc;
use zedone_core::{
    cards::{CardStore, CreateCard},
    columns::ColumnStore,
    ordering::{CardPlacement, OrderStore, Rank},
    spaces::SpaceStore,
};
use zedone_storage::{Storage, access::PendingAccess};

fn id(n: u8) -> String {
    format!("{n:032x}")
}
pub async fn seed(storage: &Storage, access: &Arc<PendingAccess>, same: bool) {
    let owner = access.clone();
    let initial = storage
        .transact(move |tx| {
            tx.create_space(&id(201), "Места карточек", owner.subject())?;
            for (column, name) in [(202, "Первая колонка"), (205, "Вторая колонка")]
            {
                tx.create_column(&id(201), &id(column), name)?;
            }
            for (card, title) in [
                (203, "Спорная карточка"),
                (210, "Сосед А"),
                (211, "Сосед Б"),
            ] {
                tx.create_card(
                    &CreateCard {
                        space: id(201),
                        column: id(202),
                        card: id(card),
                        title: title.into(),
                        description: "Сохранённое содержимое".into(),
                    },
                    owner.subject(),
                    &id(card),
                )?;
            }
            zedone_storage::sync_engine::page(tx.read().document(&id(201))?, &id(201), &vec![])
        })
        .await
        .unwrap();
    let directory = tempfile::tempdir().unwrap();
    let remote = Storage::open(directory.path().join("peer.sqlite"))
        .await
        .unwrap();
    let owner = access.clone();
    let page = remote
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let selection = tx.read().sync_selection(&id(201), &id(204))?;
            tx.set_sync_selected(&id(201), &id(204), true, selection.version)?;
            tx.receive_sync_page(&id(201), &id(204), owner.subject(), &initial)?;
            tx.place_card(
                &id(201),
                &id(203),
                &CardPlacement {
                    column: id(202),
                    rank: Rank::between(
                        Some(&Rank::Legacy(0).key(&id(210))),
                        Some(&Rank::Legacy(1).key(&id(210))),
                        &id(203),
                    )?,
                },
            )?;
            zedone_storage::sync_engine::page(tx.read().document(&id(201))?, &id(201), &vec![])
        })
        .await
        .unwrap();
    storage
        .transact(move |tx| {
            tx.place_card(
                &id(201),
                &id(203),
                &CardPlacement {
                    column: id(if same { 202 } else { 205 }),
                    rank: if same {
                        Rank::between(
                            Some(&Rank::Legacy(1).key(&id(210))),
                            Some(&Rank::Legacy(2).key(&id(211))),
                            &id(203),
                        )?
                    } else {
                        Rank::Legacy(0)
                    },
                },
            )?;
            tx.apply_changes(
                &id(201),
                zedone_storage::sync_engine::decode_changes(&page.changes)?,
            )
        })
        .await
        .unwrap();
    remote.shutdown().await.unwrap();
}
