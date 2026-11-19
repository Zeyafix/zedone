use std::sync::Arc;
use zedone_core::{
    cards::{CardStore, CreateCard, EditCard},
    cleanup::{self, InspectTrash, PurgeTrash, TrashSelection},
    columns::{self, ColumnStore, TrashColumn},
    command::Command,
    spaces::{self, SpaceStore, TrashSpace},
    wire::CommandRequest,
};
use zedone_storage::{Storage, access::PendingAccess, sync_engine};

fn request<C: Command>(id: u8, parameters: C) -> CommandRequest<C> {
    CommandRequest {
        id: [id; 16],
        operation: C::NAME.into(),
        parameters,
    }
}

pub async fn seed(storage: &Storage, access: &Arc<PendingAccess>) {
    let directory = tempfile::tempdir().unwrap();
    let peer = Storage::open(directory.path().join("peer.db"))
        .await
        .unwrap();
    let owner = access.clone();
    peer.transact(move |tx| tx.bootstrap_access(&owner))
        .await
        .unwrap();
    for (index, name) in [
        (1, "Спорная колонка"),
        (2, "Спорное пространство"),
        (3, "Поздняя правка"),
    ] {
        let space = format!("{index:032x}");
        let column = format!("{:032x}", 20);
        let card = format!("{:032x}", 21);
        let owner = access.clone();
        let (space, column, card, initial) = storage
            .transact(move |tx| {
                tx.create_space(&space, name, owner.subject())?;
                tx.create_column(&space, &column, "Первая колонка")?;
                tx.create_card(
                    &CreateCard {
                        space: space.clone(),
                        column: column.clone(),
                        card: card.clone(),
                        title: "Исходная карточка".into(),
                        description: String::new(),
                    },
                    owner.subject(),
                    "00000000000000000000000000000016",
                )?;
                let initial = sync_engine::page(tx.read().document(&space)?, &space, &vec![])?;
                Ok((space, column, card, initial))
            })
            .await
            .unwrap();
        let remote_space = space.clone();
        let owner = access.clone();
        let page = peer
            .transact(move |tx| {
                const PAIRING: &str = "00000000000000000000000000000018";
                let selection = tx.read().sync_selection(&remote_space, PAIRING)?;
                tx.set_sync_selected(&remote_space, PAIRING, true, selection.version)?;
                tx.receive_sync_page(&remote_space, PAIRING, owner.subject(), &initial)?;
                let current = tx.card_snapshot(&remote_space, &card)?;
                tx.edit_card(
                    &EditCard {
                        space: remote_space.clone(),
                        card,
                        expected: current.version,
                        title: "Правка с телефона".into(),
                        description: "Сохранённый текст".into(),
                    },
                    owner.subject(),
                    "00000000000000000000000000000017",
                )?;
                sync_engine::page(tx.read().document(&remote_space)?, &remote_space, &vec![])
            })
            .await
            .unwrap();
        let owner = access.clone();
        storage
            .timed_transact(move |tx, clock| {
                if index == 1 {
                    let before = tx.column_snapshot(&space, &column)?;
                    columns::trash(
                        tx,
                        &owner.credential(),
                        clock,
                        &request(
                            10,
                            TrashColumn {
                                space: space.clone(),
                                column,
                                expected: before.version,
                            },
                        ),
                    )?;
                } else {
                    let before = tx.space_snapshot(&space)?;
                    spaces::trash(
                        tx,
                        &owner.credential(),
                        clock,
                        &request(
                            10 + index,
                            TrashSpace {
                                space: space.clone(),
                                expected: before.version,
                            },
                        ),
                    )?;
                    if index == 3 {
                        let inspect = cleanup::inspect_trash(
                            tx,
                            &owner.credential(),
                            clock,
                            &request(
                                14,
                                InspectTrash {
                                    space: space.clone(),
                                    selection: TrashSelection::Space,
                                },
                            ),
                        )?
                        .result;
                        cleanup::purge_trash(
                            tx,
                            &owner.credential(),
                            clock,
                            &request(
                                15,
                                PurgeTrash {
                                    space: space.clone(),
                                    selection: TrashSelection::Space,
                                    expected: inspect.version,
                                },
                            ),
                        )?;
                    }
                }
                tx.apply_changes(&space, sync_engine::decode_changes(&page.changes)?)
            })
            .await
            .unwrap();
    }
    peer.shutdown().await.unwrap();
}
