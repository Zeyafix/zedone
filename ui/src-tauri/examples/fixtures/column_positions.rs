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
pub async fn seed(storage: &Storage, access: &Arc<PendingAccess>) {
    seed_positions(storage, access, false, false).await;
}

pub async fn seed_combined(storage: &Storage, access: &Arc<PendingAccess>, same: bool) {
    seed_positions(storage, access, true, same).await;
}

async fn seed_positions(
    storage: &Storage,
    access: &Arc<PendingAccess>,
    combined: bool,
    same: bool,
) {
    let owner = access.clone();
    let initial = storage
        .transact(move |tx| {
            tx.create_space(&id(201), "Позиции колонок", owner.subject())?;
            for (column, name) in [(202, "Спорная колонка"), (205, "Сосед А"), (206, "Сосед Б")]
            {
                tx.create_column(&id(201), &id(column), name)?;
            }
            tx.create_card(
                &CreateCard {
                    space: id(201),
                    column: id(202),
                    card: id(203),
                    title: "Доступная карточка".into(),
                    description: "Сохранённое содержимое".into(),
                },
                owner.subject(),
                &id(203),
            )?;
            if combined {
                for (card, title) in [
                    (207, "Спорная карточка"),
                    (210, "Сосед карточки А"),
                    (211, "Сосед карточки Б"),
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
            tx.place_column(
                &id(201),
                &id(202),
                &Rank::between(
                    Some(&Rank::Legacy(0).key(&id(205))),
                    Some(&Rank::Legacy(1).key(&id(205))),
                    &id(202),
                )?,
            )?;
            if combined {
                tx.place_card(
                    &id(201),
                    &id(207),
                    &CardPlacement {
                        column: id(202),
                        rank: Rank::between(
                            Some(&Rank::Legacy(2).key(&id(210))),
                            Some(&Rank::Legacy(3).key(&id(211))),
                            &id(207),
                        )?,
                    },
                )?;
            }
            zedone_storage::sync_engine::page(tx.read().document(&id(201))?, &id(201), &vec![])
        })
        .await
        .unwrap();
    storage
        .transact(move |tx| {
            tx.place_column(&id(201), &id(202), &Rank::Legacy(3))?;
            if combined {
                tx.place_card(
                    &id(201),
                    &id(207),
                    &CardPlacement {
                        column: id(if same { 202 } else { 205 }),
                        rank: Rank::Legacy(4),
                    },
                )?;
            }
            tx.apply_changes(
                &id(201),
                zedone_storage::sync_engine::decode_changes(&page.changes)?,
            )
        })
        .await
        .unwrap();
    remote.shutdown().await.unwrap();
}
