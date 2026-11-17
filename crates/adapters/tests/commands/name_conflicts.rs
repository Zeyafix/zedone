use super::spaces::{cli, setup_spaces, sid};
use super::*;
use zedone_core::{columns::ColumnStore, spaces::SpaceStore};

#[tokio::test]
async fn name_resolution_shares_http_ipc_cli_validation_and_retained_receipts() {
    let (dir, db, service, admin) = setup_spaces().await;
    let owner = admin.clone();
    let initial = db
        .transact(move |tx| {
            tx.create_space(&sid(1), "Base", owner.subject())?;
            tx.create_column(&sid(1), &sid(2), "Column")?;
            zedone_storage::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
        })
        .await
        .unwrap();
    let remote = Storage::open(dir.path().join("remote.db")).await.unwrap();
    let owner = admin.clone();
    let page = remote
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let selection = tx.read().sync_selection(&sid(1), &sid(9))?;
            tx.set_sync_selected(&sid(1), &sid(9), true, selection.version)?;
            tx.receive_sync_page(&sid(1), &sid(9), owner.subject(), &initial)?;
            tx.rename_space(&sid(1), "Remote")?;
            tx.rename_column(&sid(1), &sid(2), "Remote column")?;
            zedone_storage::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
        })
        .await
        .unwrap();
    db.transact(move |tx| {
        tx.rename_space(&sid(1), "Local")?;
        tx.rename_column(&sid(1), &sid(2), "Local column")?;
        tx.apply_changes(
            &sid(1),
            zedone_storage::sync_engine::decode_changes(&page.changes)?,
        )
    })
    .await
    .unwrap();
    let secret = key(&admin);
    for (index, kind) in ["space", "column"].into_iter().enumerate() {
        let id = 10 + 20 * index as u8;
        let operation = format!("{kind}.resolve_name");
        let read = format!("{kind}.read");
        let mut parameters = json!({"space":sid(1)});
        if kind == "column" {
            parameters["column"] = json!(sid(2));
        }
        let read_parameters = parameters.clone();
        let before = post(
            &service,
            &read,
            &request(id, &read, parameters.clone()),
            &secret,
        )
        .await;
        parameters["expected"] = before["result"]["version"].clone();
        parameters["name"] = json!("Chosen");
        let body = request(id + 1, &operation, parameters.clone());
        let first = post(&service, &operation, &body, &secret).await;
        assert_eq!(first["status"], "ok");
        assert_eq!(first["result"]["names"], json!(["Chosen"]));
        let ipc_result: Value =
            serde_json::from_str(&ipc::invoke(&service, &operation, &body, &secret).await).unwrap();
        assert_eq!(ipc_result["replayed"], true);
        assert_eq!(ipc_result["result"], first["result"]);
        assert_eq!(cli(&dir, &secret, &operation, &body), ipc_result);
        parameters["name"] = json!("Overwrite");
        assert_eq!(
            post(
                &service,
                &operation,
                &request(id + 2, &operation, parameters.clone()),
                &secret
            )
            .await["error"]["code"],
            "stale_state"
        );
        parameters["expected"] = first["result"]["version"].clone();
        assert_eq!(
            post(
                &service,
                &operation,
                &request(id + 3, &operation, parameters.clone()),
                &secret
            )
            .await["error"]["code"],
            "conflict"
        );
        let rename = format!("{kind}.rename");
        let late = cli(
            &dir,
            &secret,
            &rename,
            &request(id + 4, &rename, parameters),
        );
        assert_eq!(late["status"], "ok");
        assert_eq!(
            cli(&dir, &secret, &operation, &body)["result"],
            first["result"]
        );
        assert_eq!(
            post(
                &service,
                &read,
                &request(id + 5, &read, read_parameters),
                &secret
            )
            .await["result"]["names"],
            json!(["Overwrite"])
        );
    }
}
