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
        (
            "card.inspect_tags",
            json!({"space":sid(1),"card":sid(3),"extra":true}),
            "invalid_request",
        ),
        (
            "card.resolve_tag",
            json!({"space":sid(1),"card":sid(3),"tag":sid(4),"present":true,"expected":vec![0;32],"expected_tag":t["result"]["version"]}),
            "stale_state",
        ),
        (
            "tag.resolve_names",
            json!({"space":sid(1),"tag":sid(4),"names":[{"tag":sid(4),"name":"New","expected":t["result"]["version"]}]}),
            "conflict",
        ),
        (
            "tag.resolve_deletion",
            json!({"space":sid(1),"tag":sid(4),"keep":true,"expected":vec![0;32]}),
            "stale_state",
        ),
        (
            "tag.resolve_deletion",
            json!({"space":sid(1),"tag":sid(4),"keep":"true","expected":t["result"]["version"]}),
            "invalid_request",
        ),
        (
            "tag.resolve_deletion",
            json!({"space":sid(1),"tag":sid(4),"keep":false,"expected":t["result"]["version"]}),
            "conflict",
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

#[tokio::test]
async fn tag_collision_decisions_share_successful_http_ipc_cli_and_original_receipts() {
    use zedone_core::{
        cards::CardStore,
        tags::{CreateTag, SetCardTag, TagStore},
    };
    let (dir, db, service, admin) = setup_cards().await;
    let secret = key(&admin);
    post(&service, "card.create", &creation(), &secret).await;
    let initial = db
        .transact(|tx| {
            for (tag, name) in [(4, "Work"), (5, "Home"), (6, "Deleted")] {
                tx.create_tag(&CreateTag {
                    space: sid(1),
                    tag: sid(tag),
                    name: name.into(),
                })?;
            }
            let c = tx.card_snapshot(&sid(1), &sid(3))?;
            let t = tx.tag_snapshot(&sid(1), &sid(6))?;
            tx.set_card_tag(&SetCardTag {
                space: sid(1),
                card: sid(3),
                tag: sid(6),
                present: true,
                expected: c.version,
                expected_tag: t.version,
            })?;
            zedone_storage::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
        })
        .await
        .unwrap();
    let remote = Storage::open(dir.path().join("remote.db")).await.unwrap();
    let owner = admin.clone();
    let page = remote
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let selected = tx.read().sync_selection(&sid(1), &sid(9))?;
            tx.set_sync_selected(&sid(1), &sid(9), true, selected.version)?;
            tx.receive_sync_page(&sid(1), &sid(9), owner.subject(), &initial)?;
            tx.rename_tag(&sid(1), &sid(5), " Important ")?;
            let c = tx.card_snapshot(&sid(1), &sid(3))?;
            let t = tx.tag_snapshot(&sid(1), &sid(6))?;
            tx.set_card_tag(&SetCardTag {
                space: sid(1),
                card: sid(3),
                tag: sid(6),
                present: true,
                expected: c.version,
                expected_tag: t.version,
            })?;
            zedone_storage::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
        })
        .await
        .unwrap();
    db.transact(move |tx| {
        tx.rename_tag(&sid(1), &sid(4), "Important")?;
        tx.delete_tag(&sid(1), &sid(6))?;
        tx.apply_changes(
            &sid(1),
            zedone_storage::sync_engine::decode_changes(&page.changes)?,
        )
    })
    .await
    .unwrap();
    let mut names = Vec::new();
    for (tag, name) in [(4, "Все"), (5, "Всё")] {
        let t = same(
            &service,
            &dir,
            &secret,
            "tag.read",
            &request(40 + tag, "tag.read", json!({"space":sid(1),"tag":sid(tag)})),
        )
        .await;
        names.push(json!({"tag":sid(tag),"name":name,"expected":t["version"]}));
    }
    let result = same(
        &service,
        &dir,
        &secret,
        "tag.resolve_names",
        &request(
            50,
            "tag.resolve_names",
            json!({"space":sid(1),"tag":sid(4),"names":names}),
        ),
    )
    .await;
    assert!(
        result["tags"]
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["state"] == "active")
    );
    let t = same(
        &service,
        &dir,
        &secret,
        "tag.read",
        &request(51, "tag.read", json!({"space":sid(1),"tag":sid(6)})),
    )
    .await;
    assert_eq!(t["deletion_conflict"], true);
    let chosen = same(
        &service,
        &dir,
        &secret,
        "tag.resolve_deletion",
        &request(
            52,
            "tag.resolve_deletion",
            json!({"space":sid(1),"tag":sid(6),"keep":true,"expected":t["version"]}),
        ),
    )
    .await;
    assert_eq!(chosen["state"], "active");
    assert!(chosen.get("deletion_conflict").is_none());
    let c = same(
        &service,
        &dir,
        &secret,
        "card.read",
        &request(53, "card.read", json!({"space":sid(1),"card":sid(3)})),
    )
    .await;
    assert_eq!(c["state"], "active");
    assert_eq!(c["tags"], json!([sid(6)]));
}
