use std::{path::Path, sync::Arc};
use zedone_core::{
    cards::{CardStore, CreateCard},
    columns::ColumnStore,
    spaces::SpaceStore,
};
use zedone_storage::{
    Storage, access::PendingAccess, migration_backup::BackupLimits, snapshot::SnapshotLimits,
};

async fn prepare(directory: &Path, unsupported: bool) {
    std::fs::create_dir_all(directory).unwrap();
    let path = directory.join("zedone.sqlite");
    if path.exists() {
        return;
    }
    let db = Storage::open(&path).await.unwrap();
    let owner = Arc::new(PendingAccess::generate().unwrap());
    let key = owner
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    std::fs::write(
        directory.join("local-session.json"),
        serde_json::to_vec(&serde_json::json!({"subject":owner.subject(),"key":key})).unwrap(),
    )
    .unwrap();
    db.transact(move |ctx| {
        ctx.bootstrap_access(&owner)?;
        let space = "1".repeat(32);
        let column = "2".repeat(32);
        ctx.create_space(&space, "База до сбоя", owner.subject())?;
        ctx.create_column(&space, &column, "Работа")?;
        ctx.create_card(
            &CreateCard {
                space,
                column,
                card: "3".repeat(32),
                title: "Вернуть документы".into(),
                description: "Полная копия сохраняет карточку.".into(),
            },
            owner.subject(),
            &"4".repeat(32),
        )?;
        Ok(())
    })
    .await
    .unwrap();
    let folder = directory.join("zedone.sqlite.backups");
    std::fs::create_dir(&folder).unwrap();
    let backup = db
        .prepare_migration_backup(
            folder,
            1795852800000,
            BackupLimits {
                max_database_bytes: 64 * 1024 * 1024,
                max_spaces: 10,
                max_total_snapshot_bytes: 32 * 1024 * 1024,
                snapshot: SnapshotLimits {
                    max_bytes: 3 * 1024 * 1024,
                    max_changes: 10000,
                },
            },
        )
        .await
        .unwrap();
    drop(backup);
    db.shutdown().await.unwrap();
    if unsupported {
        let mut bytes = std::fs::read(&path).unwrap();
        std::fs::write(directory.join("supported.sqlite"), &bytes).unwrap();
        bytes[60..64].copy_from_slice(&99_u32.to_be_bytes());
        std::fs::write(path, bytes).unwrap();
    } else {
        std::fs::write(path, b"damaged maintenance fixture").unwrap();
    }
}
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(args.len(), 2);
    let directory = Path::new(&args[1]);
    assert!(
        [
            "--prepare",
            "--serve",
            "--prepare-unsupported",
            "--serve-unsupported"
        ]
        .iter()
        .any(|mode| args[0] == *mode)
    );
    let unsupported = args[0] == "--prepare-unsupported" || args[0] == "--serve-unsupported";
    prepare(directory, unsupported).await;
    if args[0] == "--prepare" || args[0] == "--prepare-unsupported" {
        return;
    }
    let shell = zedone_shell::open_shell(directory).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    println!("http://{address}");
    axum::serve(listener, zedone_shell::web::router(shell, address))
        .await
        .unwrap();
}
