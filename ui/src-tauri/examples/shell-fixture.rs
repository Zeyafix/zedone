#![forbid(unsafe_code)]
use std::sync::Arc;
#[path = "fixtures/backups.rs"]
mod backup_fixture;
#[path = "fixtures/column_positions.rs"]
mod column_position_fixture;
#[path = "fixtures/container_deletion.rs"]
mod container_deletion_fixture;
#[path = "fixtures/placements.rs"]
mod placement_fixture;
#[path = "fixtures/tags.rs"]
mod tag_fixture;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    #[cfg(windows)]
    if args.len() == 2 && args[0] == "--cleanup-devices" {
        use zedone_adapters::secrets::{SecretStore, Slot};
        let directory = std::path::Path::new(&args[1]);
        assert!(directory.join("host-namespace-v1").is_file());
        let namespace = zedone_adapters::device_host::namespace(directory).unwrap();
        let store = zedone_adapters::secrets::windows::WindowsSecrets::new(
            namespace,
            &directory.join("host-locks"),
        );
        for slot in [
            Slot::ChannelIdentity,
            Slot::PendingPairing,
            Slot::ConnectedPairing,
            Slot::OutgoingInvitation,
        ] {
            store.remove(slot).await.unwrap();
        }
        return;
    }
    assert_eq!(
        args.len(),
        2,
        "Expected --prepare or --serve and an empty test directory"
    );
    assert!(
        args[0] == "--prepare"
            || args[0] == "--serve"
            || args[0] == "--serve-backups"
            || args[0] == "--prepare-backups"
            || args[0] == "--prepare-conflicts"
            || args[0] == "--serve-conflicts"
            || args[0] == "--prepare-text-conflicts"
            || args[0] == "--serve-text-conflicts"
            || args[0] == "--prepare-field-conflicts"
            || args[0] == "--serve-field-conflicts"
            || args[0] == "--prepare-card-deletion"
            || args[0] == "--serve-card-deletion"
            || args[0] == "--prepare-container-deletion"
            || args[0] == "--serve-container-deletion"
            || args[0] == "--prepare-placement-conflicts"
            || args[0] == "--serve-placement-conflicts"
            || args[0] == "--serve-same-column-conflicts"
            || args[0] == "--serve-column-position-conflicts"
            || args[0] == "--prepare-column-position-conflicts"
            || args[0] == "--serve-position-conflicts"
            || args[0] == "--serve-same-position-conflicts"
            || args[0] == "--prepare-position-conflicts"
            || args[0] == "--prepare-tag-conflicts"
            || args[0] == "--serve-tag-conflicts"
    );
    let directory = std::path::Path::new(&args[1]);
    std::fs::create_dir_all(directory).unwrap();
    assert_eq!(
        std::fs::read_dir(directory).unwrap().count(),
        0,
        "Fixture refuses nonempty directory"
    );
    let storage = zedone_storage::Storage::open(directory.join("zedone.sqlite"))
        .await
        .unwrap();
    let access = Arc::new(zedone_storage::access::PendingAccess::generate().unwrap());
    let pending = access.clone();
    storage
        .timed_transact(move |ctx, _| ctx.bootstrap_access(&pending))
        .await
        .unwrap();
    let key: String = access
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    std::fs::write(
        directory.join("local-session.json"),
        serde_json::to_vec(&serde_json::json!({
            "subject": access.subject(), "key": key,
        }))
        .unwrap(),
    )
    .unwrap();
    if args[0] == "--prepare-conflicts" || args[0] == "--serve-conflicts" {
        seed_conflicts(&storage, &access, true, false, false).await;
    }
    if args[0] == "--serve-backups" || args[0] == "--prepare-backups" {
        backup_fixture::seed(&storage, access.clone()).await;
        storage.daily_tick(true).await.unwrap();
        if args[0] == "--serve-backups" {
            storage
                .select_backup_folder(access.credential().0.to_vec(), None, true)
                .await
                .unwrap();
            let target = directory.join("catalog-target");
            std::fs::create_dir(&target).unwrap();
            storage
                .select_backup_folder(
                    access.credential().0.to_vec(),
                    Some(target.to_str().unwrap().into()),
                    true,
                )
                .await
                .unwrap();
        }
    }
    if args[0] == "--prepare-text-conflicts" || args[0] == "--serve-text-conflicts" {
        seed_conflicts(&storage, &access, false, true, false).await;
    }
    if args[0] == "--prepare-field-conflicts" || args[0] == "--serve-field-conflicts" {
        seed_conflicts(&storage, &access, true, true, false).await;
    }
    if args[0] == "--prepare-card-deletion" || args[0] == "--serve-card-deletion" {
        seed_conflicts(&storage, &access, false, true, true).await;
    }
    if args[0] == "--prepare-container-deletion" || args[0] == "--serve-container-deletion" {
        container_deletion_fixture::seed(&storage, &access).await;
    }
    if args[0] == "--prepare-placement-conflicts"
        || args[0] == "--serve-placement-conflicts"
        || args[0] == "--serve-same-column-conflicts"
    {
        placement_fixture::seed(
            &storage,
            &access,
            args[0] == "--serve-same-column-conflicts",
        )
        .await;
    }
    if args[0] == "--prepare-column-position-conflicts"
        || args[0] == "--serve-column-position-conflicts"
    {
        column_position_fixture::seed(&storage, &access).await;
    }
    if args[0] == "--serve-position-conflicts"
        || args[0] == "--serve-same-position-conflicts"
        || args[0] == "--prepare-position-conflicts"
    {
        column_position_fixture::seed_combined(
            &storage,
            &access,
            args[0] == "--serve-same-position-conflicts",
        )
        .await;
    }
    if args[0] == "--prepare-tag-conflicts" || args[0] == "--serve-tag-conflicts" {
        tag_fixture::seed(&storage, &access).await;
    }
    if args[0] == "--prepare"
        || args[0] == "--prepare-backups"
        || args[0] == "--prepare-tag-conflicts"
        || args[0] == "--prepare-position-conflicts"
        || args[0] == "--prepare-column-position-conflicts"
        || args[0] == "--prepare-conflicts"
        || args[0] == "--prepare-text-conflicts"
        || args[0] == "--prepare-field-conflicts"
        || args[0] == "--prepare-card-deletion"
        || args[0] == "--prepare-container-deletion"
        || args[0] == "--prepare-placement-conflicts"
    {
        storage.shutdown().await.unwrap();
        return;
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    println!("http://{address}");
    #[allow(unused_mut)]
    let mut shell = zedone_shell::Shell::new(
        Some(if args[0] == "--serve-backups" {
            zedone_adapters::Service::new(storage).with_daily_backups()
        } else {
            zedone_adapters::Service::new(storage)
        }),
        directory.join("local-session.json"),
    );
    #[cfg(not(windows))]
    {
        shell.devices = Some(Arc::new(
            zedone_adapters::device_control::Controller::new(Arc::new(
                zedone_adapters::device_host::DeviceHost::new(
                    shell.service.clone().unwrap(),
                    Memory::default(),
                ),
            ))
            .with_automatic(directory.join("local-session.json")),
        ));
    }
    axum::serve(listener, zedone_shell::web::router(shell, address))
        .await
        .unwrap();
}

async fn seed_conflicts(
    storage: &zedone_storage::Storage,
    access: &Arc<zedone_storage::access::PendingAccess>,
    names: bool,
    text: bool,
    deletion: bool,
) {
    use zedone_core::{
        cards::{CardStore, CreateCard},
        columns::ColumnStore,
        ordering::{CardPlacement, OrderStore, Rank},
        spaces::SpaceStore,
        tags::{CreateTag, SetCardTag, TagStore},
    };
    const SPACE: &str = "000000000000000000000000000000c9";
    const COLUMN: &str = "000000000000000000000000000000ca";
    const OTHER_COLUMN: &str = "000000000000000000000000000000d0";
    const CARD: &str = "000000000000000000000000000000cb";
    const TAG: &str = "000000000000000000000000000000d1";
    let owner = access.clone();
    let initial = storage
        .transact(move |tx| {
            tx.create_space(SPACE, "Общее имя", owner.subject())?;
            tx.create_column(SPACE, COLUMN, "Общая колонка")?;
            if deletion {
                tx.create_column(SPACE, OTHER_COLUMN, "Колонка с телефона")?;
                tx.create_tag(&CreateTag {
                    space: SPACE.into(),
                    tag: TAG.into(),
                    name: "Спорный тег".into(),
                })?;
            }
            tx.create_card(
                &CreateCard {
                    space: SPACE.into(),
                    column: COLUMN.into(),
                    card: "000000000000000000000000000000cb".into(),
                    title: "Сохранённая карточка".into(),
                    description: "Содержимое при конфликте названия".into(),
                },
                owner.subject(),
                "000000000000000000000000000000cc",
            )?;
            zedone_storage::sync_engine::page(tx.read().document(SPACE)?, SPACE, &vec![])
        })
        .await
        .unwrap();
    let temporary = tempfile::tempdir().unwrap();
    let remote = zedone_storage::Storage::open(temporary.path().join("peer.sqlite"))
        .await
        .unwrap();
    let owner = access.clone();
    remote
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let pairing = "000000000000000000000000000000cd";
            let selection = tx.read().sync_selection(SPACE, pairing)?;
            tx.set_sync_selected(SPACE, pairing, true, selection.version)?;
            tx.receive_sync_page(SPACE, pairing, owner.subject(), &initial)?;
            if deletion {
                tx.place_card(
                    SPACE,
                    CARD,
                    &CardPlacement {
                        column: OTHER_COLUMN.into(),
                        rank: Rank::Legacy(0),
                    },
                )?;
                let card = tx.card_snapshot(SPACE, CARD)?;
                let tag = tx.tag_snapshot(SPACE, TAG)?;
                tx.set_card_tag(&SetCardTag {
                    space: SPACE.into(),
                    card: CARD.into(),
                    tag: TAG.into(),
                    present: true,
                    expected: card.version,
                    expected_tag: tag.version,
                })?;
            }
            if text {
                let card = tx.card_snapshot(SPACE, "000000000000000000000000000000cb")?;
                tx.edit_card(
                    &zedone_core::cards::EditCard {
                        space: SPACE.into(),
                        card: card.card,
                        title: "Заголовок с телефона".into(),
                        description: "Описание с телефона\n**Markdown**".into(),
                        expected: card.version,
                    },
                    owner.subject(),
                    "000000000000000000000000000000ce",
                )?;
            }
            if names {
                tx.rename_space(SPACE, "Название с телефона")?;
                tx.rename_column(SPACE, COLUMN, "Колонка с телефона")?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let page = remote
        .transact(|tx| {
            zedone_storage::sync_engine::page(tx.read().document(SPACE)?, SPACE, &vec![])
        })
        .await
        .unwrap();
    let owner = access.clone();
    storage
        .transact(move |tx| {
            if deletion {
                tx.place_card(
                    SPACE,
                    CARD,
                    &CardPlacement {
                        column: COLUMN.into(),
                        rank: Rank::Legacy(0),
                    },
                )?;
                let card = tx.card_snapshot(SPACE, CARD)?;
                let tag = tx.tag_snapshot(SPACE, TAG)?;
                tx.set_card_tag(&SetCardTag {
                    space: SPACE.into(),
                    card: CARD.into(),
                    tag: TAG.into(),
                    present: false,
                    expected: card.version,
                    expected_tag: tag.version,
                })?;
                tx.set_card_lifecycle(
                    SPACE,
                    CARD,
                    &format!("trash:{}:000000000000000000000000000000cf", owner.subject()),
                )?;
            } else if text {
                let card = tx.card_snapshot(SPACE, "000000000000000000000000000000cb")?;
                tx.edit_card(
                    &zedone_core::cards::EditCard {
                        space: SPACE.into(),
                        card: card.card,
                        title: "Заголовок с компьютера".into(),
                        description: "Описание с компьютера\n<script>сырой текст</script>".into(),
                        expected: card.version,
                    },
                    owner.subject(),
                    "000000000000000000000000000000cf",
                )?;
            }
            if names {
                tx.rename_space(SPACE, "Название с компьютера")?;
                tx.rename_column(SPACE, COLUMN, "Колонка с компьютера")?;
            }
            tx.apply_changes(
                SPACE,
                zedone_storage::sync_engine::decode_changes(&page.changes)?,
            )
        })
        .await
        .unwrap();
    remote.shutdown().await.unwrap();
}

#[cfg(not(windows))]
#[derive(Default)]
struct Memory(
    std::sync::Mutex<std::collections::HashMap<&'static str, zedone_adapters::secrets::Secret>>,
);
#[cfg(not(windows))]
impl zedone_adapters::secrets::SecretStore for Memory {
    async fn load(
        &self,
        slot: zedone_adapters::secrets::Slot,
    ) -> Result<Option<zedone_adapters::secrets::Secret>, zedone_adapters::secrets::SecretError>
    {
        Ok(self
            .0
            .lock()
            .unwrap()
            .get(slot.name())
            .map(|bytes| zedone_adapters::secrets::Secret::new(bytes.to_vec())))
    }
    async fn create(
        &self,
        slot: zedone_adapters::secrets::Slot,
        bytes: &[u8],
    ) -> Result<(), zedone_adapters::secrets::SecretError> {
        let mut map = self.0.lock().unwrap();
        if map.contains_key(slot.name()) {
            return Err(zedone_adapters::secrets::SecretError::AlreadyExists);
        }
        map.insert(
            slot.name(),
            zedone_adapters::secrets::Secret::new(bytes.to_vec()),
        );
        Ok(())
    }
    async fn remove(
        &self,
        slot: zedone_adapters::secrets::Slot,
    ) -> Result<(), zedone_adapters::secrets::SecretError> {
        self.0.lock().unwrap().remove(slot.name());
        Ok(())
    }
}
