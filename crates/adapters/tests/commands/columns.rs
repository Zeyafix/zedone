use super::spaces::{cli, setup_spaces, sid};
use super::*;

async fn setup_column_space() -> (TempDir, Storage, Service, Arc<PendingAccess>) {
    let (dir, storage, service, admin) = setup_spaces().await;
    assert_eq!(
        post(
            &service,
            "space.create",
            &request(250, "space.create", json!({"space":sid(1),"name":"Board"})),
            &key(&admin)
        )
        .await["status"],
        "ok"
    );
    (dir, storage, service, admin)
}

#[tokio::test]
async fn column_groups_preview_members_and_share_stale_rights_replay_and_restart_across_adapters() {
    let (dir, storage, service, admin) = setup_column_space().await;
    let secret = key(&admin);
    post(
        &service,
        "column.create",
        &request(
            1,
            "column.create",
            json!({"space":sid(1),"column":sid(2),"name":"Column"}),
        ),
        &secret,
    )
    .await;
    for id in [8, 9] {
        assert_eq!(post(&service, "card.create", &request(id, "card.create",
            json!({"space":sid(1),"column":sid(2),"card":sid(id),"title":"Task","description":"Saved"})), &secret).await["status"], "ok");
    }
    let before = post(
        &service,
        "column.read",
        &request(10, "column.read", json!({"space":sid(1),"column":sid(2)})),
        &secret,
    )
    .await;
    assert_eq!(before["result"]["contents"].as_array().unwrap().len(), 2);
    let first = &before["result"]["contents"][0];
    let edited = cli(
        &dir,
        &secret,
        "card.edit",
        &request(
            11,
            "card.edit",
            json!({"space":sid(1),"card":first["card"],"title":"Later","description":"Saved","expected":first["version"]}),
        ),
    );
    assert_eq!(edited["status"], "ok");
    let old_trash = request(
        12,
        "column.trash",
        json!({"space":sid(1),"column":sid(2),"expected":before["result"]["version"]}),
    );
    for result in [
        post(&service, "column.trash", &old_trash, &secret).await,
        serde_json::from_str::<Value>(
            &ipc::invoke(&service, "column.trash", &old_trash, &secret).await,
        )
        .unwrap(),
        cli(&dir, &secret, "column.trash", &old_trash),
    ] {
        assert_eq!(result["error"]["code"], "stale_state");
    }
    let current = cli(
        &dir,
        &secret,
        "column.read",
        &request(13, "column.read", json!({"space":sid(1),"column":sid(2)})),
    );
    assert!(
        current["result"]["contents"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["state"] == "active")
    );
    let trash = request(
        14,
        "column.trash",
        json!({"space":sid(1),"column":sid(2),"expected":current["result"]["version"]}),
    );
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
                None,
            )
        })
        .await
        .unwrap();
    let member_key = key(&member);
    assert_eq!(
        post(&service, "column.trash", &trash, &member_key).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        cli(
            &dir,
            &member_key,
            "column.read",
            &request(15, "column.read", json!({"space":sid(1),"column":sid(2)}))
        )["result"],
        current["result"]
    );
    let trashed = post(&service, "column.trash", &trash, &secret).await;
    assert_eq!(trashed["status"], "ok");
    for card in trashed["result"]["contents"].as_array().unwrap() {
        assert_eq!(card["state"], "trashed");
        assert_eq!(
            card["deletion_groups"],
            trashed["result"]["deletion_groups"]
        );
    }
    let restore = request(
        16,
        "column.restore",
        json!({"space":sid(1),"column":sid(2),"expected":trashed["result"]["version"]}),
    );
    let restored: Value =
        serde_json::from_str(&ipc::invoke(&service, "column.restore", &restore, &secret).await)
            .unwrap();
    assert!(
        restored["result"]["contents"]
            .as_array()
            .unwrap()
            .iter()
            .all(|c| c["state"] == "active")
    );
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_transact(move |ctx, clock| ctx.revoke_access(&a.credential(), clock, m.subject()))
        .await
        .unwrap();
    assert_eq!(
        cli(
            &dir,
            &member_key,
            "column.read",
            &request(15, "column.read", json!({"space":sid(1),"column":sid(2)}))
        )["error"]["code"],
        "access_denied"
    );
    storage.shutdown().await.unwrap();
    let replay = cli(&dir, &secret, "column.trash", &trash);
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], trashed["result"]);
    let now = cli(
        &dir,
        &secret,
        "column.read",
        &request(17, "column.read", json!({"space":sid(1),"column":sid(2)})),
    );
    assert_eq!(now["result"], restored["result"]);
}

#[tokio::test]
async fn columns_share_receipts_and_reverse_actions_across_http_cli_ipc_and_restart() {
    let (dir, storage, service, admin) = setup_column_space().await;
    let secret = key(&admin);
    let creation = request(
        1,
        "column.create",
        json!({"space":sid(1),"column":sid(2),"name":"Ёж 空間 😀"}),
    );
    let created = post(&service, "column.create", &creation, &secret).await;
    assert_eq!(created["status"], "ok");
    assert_eq!(created["result"]["names"], json!(["Ёж 空間 😀"]));
    let ipc_repeat: Value =
        serde_json::from_str(&ipc::invoke(&service, "column.create", &creation, &secret).await)
            .unwrap();
    assert_eq!(ipc_repeat["replayed"], true);
    assert_eq!(ipc_repeat, cli(&dir, &secret, "column.create", &creation));
    let rename = request(
        2,
        "column.rename",
        json!({"space":sid(1),"column":sid(2),"name":"","expected":created["result"]["version"]}),
    );
    let renamed = cli(&dir, &secret, "column.rename", &rename);
    assert_eq!(renamed["result"]["names"], json!([""]));
    let trash = request(
        3,
        "column.trash",
        json!({"space":sid(1),"column":sid(2),"expected":renamed["result"]["version"]}),
    );
    let trashed = post(&service, "column.trash", &trash, &secret).await;
    assert_eq!(trashed["result"]["state"], "trashed");
    let hidden = post(
        &service,
        "column.list",
        &request(
            4,
            "column.list",
            json!({"space":sid(1),"include_trashed":false}),
        ),
        &secret,
    )
    .await;
    assert_eq!(hidden["result"]["columns"], json!([]));
    let restore = request(
        5,
        "column.restore",
        json!({"space":sid(1),"column":sid(2),"expected":trashed["result"]["version"]}),
    );
    let restored: Value =
        serde_json::from_str(&ipc::invoke(&service, "column.restore", &restore, &secret).await)
            .unwrap();
    assert_eq!(restored["result"]["state"], "active");
    assert_eq!(
        cli(&dir, &secret, "column.trash", &trash)["result"],
        trashed["result"]
    );
    let reading = request(6, "column.read", json!({"space":sid(1),"column":sid(2)}));
    assert_eq!(
        post(&service, "column.read", &reading, &secret).await["result"],
        restored["result"]
    );
    let listed = cli(
        &dir,
        &secret,
        "column.list",
        &request(
            7,
            "column.list",
            json!({"space":sid(1),"include_trashed":true}),
        ),
    );
    assert_eq!(listed["result"]["columns"], json!([restored["result"]]));
    let stale = request(
        8,
        "column.trash",
        json!({"space":sid(1),"column":sid(2),"expected":created["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "column.trash", &stale, &secret).await["error"]["code"],
        "stale_state"
    );
    storage.shutdown().await.unwrap();
    assert_eq!(
        cli(&dir, &secret, "column.read", &reading)["result"],
        restored["result"]
    );
    assert_eq!(
        cli(&dir, &secret, "column.create", &creation)["replayed"],
        true
    );
}

#[tokio::test]
async fn column_decoding_rights_and_receipt_authorization_are_identical_at_every_entry() {
    let (dir, storage, service, admin) = setup_column_space().await;
    let secret = key(&admin);
    let creation = request(
        1,
        "column.create",
        json!({"space":sid(1),"column":sid(1),"name":"Base"}),
    );
    let created = post(&service, "column.create", &creation, &secret).await;
    for params in [
        json!({"space":sid(1),"column":"0".repeat(32),"name":"No"}),
        json!({"space":sid(1),"column":"A".repeat(32),"name":"No"}),
        json!({"space":sid(1),"column":sid(2),"name":"No","subject":admin.subject()}),
        json!({"space":sid(1),"column":sid(2),"name":"No","permission":"read_write"}),
    ] {
        let body = request(2, "column.create", params);
        let denied = post(&service, "column.create", &body, &secret).await;
        assert_eq!(denied["error"]["code"], "invalid_request");
        assert_eq!(cli(&dir, &secret, "column.create", &body), denied);
        assert_eq!(
            serde_json::from_str::<Value>(
                &ipc::invoke(&service, "column.create", &body, &secret).await
            )
            .unwrap(),
            denied
        );
    }
    let duplicate = creation.replace(
        "\"name\":\"Base\"",
        "\"name\":\"Base\",\"name\":\"Override\"",
    );
    assert_ne!(duplicate, creation);
    assert_eq!(
        post(&service, "column.create", &duplicate, &secret).await["error"]["code"],
        "invalid_request"
    );
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
                None,
            )
        })
        .await
        .unwrap();
    let reading = request(3, "column.read", json!({"space":sid(1),"column":sid(1)}));
    assert_eq!(
        post(&service, "column.read", &reading, &key(&member)).await["result"],
        created["result"]
    );
    let denied = post(&service, "column.create", &creation, &key(&member)).await;
    assert_eq!(denied["error"]["code"], "access_denied");
    assert_eq!(cli(&dir, &key(&member), "column.create", &creation), denied);
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_transact(move |ctx, clock| ctx.revoke_access(&a.credential(), clock, m.subject()))
        .await
        .unwrap();
    let denied = post(&service, "column.read", &reading, &key(&member)).await;
    assert_eq!(denied["error"]["code"], "access_denied");
    assert_eq!(cli(&dir, &key(&member), "column.read", &reading), denied);
    assert_eq!(
        serde_json::from_str::<Value>(
            &ipc::invoke(&service, "column.read", &reading, &key(&member)).await
        )
        .unwrap(),
        denied
    );
    // Even a creator's already committed receipt cannot bypass lost RW rights.
    let backup = Arc::new(PendingAccess::generate().unwrap());
    let (a, b) = (admin.clone(), backup.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &b,
                &GrantScope::Administration,
                None,
            )
        })
        .await
        .unwrap();
    let a = admin.clone();
    storage
        .timed_transact(move |ctx, clock| ctx.revoke_access(&a.credential(), clock, a.subject()))
        .await
        .unwrap();
    assert_eq!(
        post(&service, "column.create", &creation, &secret).await["error"]["code"],
        "access_denied"
    );
}
