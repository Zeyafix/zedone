use super::cards::{creation, setup_cards};
use super::spaces::{cli, sid};
use super::*;

async fn same(service: &Service, dir: &TempDir, secret: &str, op: &str, body: &str) -> Value {
    let first = post(service, op, body, secret).await;
    assert_eq!(first["status"], "ok", "{first}");
    let mut replay = first.clone();
    replay["replayed"] = json!(true);
    assert_eq!(
        serde_json::from_str::<Value>(&ipc::invoke(service, op, body, secret).await).unwrap(),
        replay
    );
    assert_eq!(cli(dir, secret, op, body), replay);
    first["result"].clone()
}

#[tokio::test]
async fn tag_lifecycle_and_assignment_share_real_http_cli_ipc_and_restart_receipts() {
    let (dir, storage, service, admin) = setup_cards().await;
    let secret = key(&admin);
    let c = same(&service, &dir, &secret, "card.create", &creation()).await;
    let t = same(
        &service,
        &dir,
        &secret,
        "tag.create",
        &request(
            2,
            "tag.create",
            json!({"space":sid(1),"tag":sid(4),"name":" Работа 😀 "}),
        ),
    )
    .await;
    let assigned=same(&service,&dir,&secret,"card.set_tag",&request(3,"card.set_tag",json!({"space":sid(1),"card":sid(3),"tag":sid(4),"present":true,"expected":c["version"],"expected_tag":t["version"]}))).await;
    assert_eq!(assigned["tags"], json!([sid(4)]));
    let t = same(
        &service,
        &dir,
        &secret,
        "tag.read",
        &request(4, "tag.read", json!({"space":sid(1),"tag":sid(4)})),
    )
    .await;
    let renamed = same(
        &service,
        &dir,
        &secret,
        "tag.rename",
        &request(
            5,
            "tag.rename",
            json!({"space":sid(1),"tag":sid(4),"name":"Личное","expected":t["version"]}),
        ),
    )
    .await;
    assert_eq!(renamed["names"], json!(["Личное"]));
    let list = same(
        &service,
        &dir,
        &secret,
        "tag.list",
        &request(
            6,
            "tag.list",
            json!({"space":sid(1),"include_deleted":false}),
        ),
    )
    .await;
    assert_eq!(list["tags"], json!([renamed.clone()]));
    let c = same(
        &service,
        &dir,
        &secret,
        "card.read",
        &request(7, "card.read", json!({"space":sid(1),"card":sid(3)})),
    )
    .await;
    let removed=same(&service,&dir,&secret,"card.set_tag",&request(8,"card.set_tag",json!({"space":sid(1),"card":sid(3),"tag":sid(4),"present":false,"expected":c["version"],"expected_tag":renamed["version"]}))).await;
    assert_eq!(removed["tags"], json!([]));
    let t = same(
        &service,
        &dir,
        &secret,
        "tag.read",
        &request(9, "tag.read", json!({"space":sid(1),"tag":sid(4)})),
    )
    .await;
    let body = request(
        10,
        "tag.delete",
        json!({"space":sid(1),"tag":sid(4),"expected":t["version"]}),
    );
    let deleted = same(&service, &dir, &secret, "tag.delete", &body).await;
    assert_eq!(deleted["state"], "deleted");
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage.clone());
    assert_eq!(
        post(&service, "tag.delete", &body, &secret).await,
        json!({"status":"ok","replayed":true,"result":deleted})
    );
    let recreated = same(
        &service,
        &dir,
        &secret,
        "tag.create",
        &request(
            11,
            "tag.create",
            json!({"space":sid(1),"tag":sid(5),"name":"Личное"}),
        ),
    )
    .await;
    assert_ne!(recreated["tag"], deleted["tag"]);
    let c = post(
        &service,
        "card.read",
        &request(12, "card.read", json!({"space":sid(1),"card":sid(3)})),
        &secret,
    )
    .await;
    assert_eq!(c["result"]["tags"], json!([]));
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn tag_commands_reject_malformed_fields_and_stale_conditions_on_every_adapter() {
    let (dir, _storage, service, admin) = setup_cards().await;
    let secret = key(&admin);
    post(&service, "card.create", &creation(), &secret).await;
    let t = post(
        &service,
        "tag.create",
        &request(
            2,
            "tag.create",
            json!({"space":sid(1),"tag":sid(4),"name":"Все"}),
        ),
        &secret,
    )
    .await;
    for (op, params, code) in [
        (
            "tag.create",
            json!({"space":sid(1),"tag":sid(5),"name":" ВСЕ "}),
            "conflict",
        ),
        (
            "tag.create",
            json!({"space":sid(1),"tag":sid(5),"name":"\u{2003}"}),
            "invalid_request",
        ),
        (
            "tag.read",
            json!({"space":sid(1),"tag":sid(4),"extra":true}),
            "invalid_request",
        ),
        (
            "tag.list",
            json!({"space":sid(1),"include_deleted":"false"}),
            "invalid_request",
        ),
        (
            "tag.rename",
            json!({"space":sid(1),"tag":sid(4),"name":"New","expected":vec![0;32]}),
            "stale_state",
        ),
        (
            "tag.delete",
            json!({"space":sid(1),"tag":sid(4),"expected":vec![0;32]}),
            "stale_state",
        ),
        (
            "card.set_tag",
            json!({"space":sid(1),"card":sid(3),"tag":sid(4),"present":"true","expected":vec![0;32],"expected_tag":t["result"]["version"]}),
            "invalid_request",
        ),
        (
            "tag.restore",
            json!({"space":sid(1),"tag":sid(4)}),
            "invalid_request",
        ),
    ] {
        let body = request(3, op, params);
        let expected = json!({"status":"error","error":{"code":code}});
        assert_eq!(post(&service, op, &body, &secret).await, expected);
        assert_eq!(cli(&dir, &secret, op, &body), expected);
        assert_eq!(
            serde_json::from_str::<Value>(&ipc::invoke(&service, op, &body, &secret).await)
                .unwrap(),
            expected
        );
    }
}
