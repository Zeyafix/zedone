use super::spaces::{cli, setup_spaces, sid};
use super::*;

async fn all_transports(
    service: &Service,
    dir: &TempDir,
    secret: &str,
    id: u8,
    op: &str,
    parameters: Value,
) -> Value {
    let body = request(id, op, parameters);
    let first = post(service, op, &body, secret).await;
    assert_eq!(first["status"], "ok", "{op}: {first}");
    let ipc: Value = serde_json::from_str(&ipc::invoke(service, op, &body, secret).await).unwrap();
    let cli = cli(dir, secret, op, &body);
    for replay in [ipc, cli] {
        assert_eq!(replay["status"], "ok");
        assert_eq!(replay["replayed"], true);
        assert_eq!(replay["result"], first["result"]);
    }
    first["result"].clone()
}
#[tokio::test]
async fn space_groups_destination_cleanup_and_history_share_http_cli_ipc_and_restart() {
    let (dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    all_transports(
        &service,
        &dir,
        &secret,
        1,
        "space.create",
        json!({"space":sid(1),"name":"Board"}),
    )
    .await;
    for id in [2, 3] {
        all_transports(
            &service,
            &dir,
            &secret,
            id,
            "column.create",
            json!({"space":sid(1),"column":sid(id),"name":"Column"}),
        )
        .await;
    }
    let card = all_transports(
        &service,
        &dir,
        &secret,
        4,
        "card.create",
        json!({"space":sid(1),"column":sid(2),"card":sid(9),"title":"Task","description":"Saved"}),
    )
    .await;
    let h = all_transports(
        &service,
        &dir,
        &secret,
        5,
        "history.inspect",
        json!({"space":sid(1),"card":sid(9)}),
    )
    .await;
    assert_eq!(h["entries"].as_array().unwrap().len(), 1);
    let clear = json!({"space":sid(1),"card":sid(9),"expected":h["version"]});
    all_transports(&service, &dir, &secret, 6, "history.clear", clear.clone()).await;
    let edited=all_transports(&service,&dir,&secret,7,"card.edit",json!({"space":sid(1),"card":sid(9),"title":"Later","description":"New","expected":card["version"]})).await;
    let column = all_transports(
        &service,
        &dir,
        &secret,
        8,
        "column.read",
        json!({"space":sid(1),"column":sid(2)}),
    )
    .await;
    all_transports(
        &service,
        &dir,
        &secret,
        9,
        "column.trash",
        json!({"space":sid(1),"column":sid(2),"expected":column["version"]}),
    )
    .await;
    let parent = all_transports(
        &service,
        &dir,
        &secret,
        10,
        "space.read",
        json!({"space":sid(1)}),
    )
    .await;
    let deleted = all_transports(
        &service,
        &dir,
        &secret,
        11,
        "space.trash",
        json!({"space":sid(1),"expected":parent["version"]}),
    )
    .await;
    all_transports(
        &service,
        &dir,
        &secret,
        12,
        "space.restore",
        json!({"space":sid(1),"expected":deleted["version"]}),
    )
    .await;
    let card = all_transports(
        &service,
        &dir,
        &secret,
        13,
        "card.read",
        json!({"space":sid(1),"card":sid(9)}),
    )
    .await;
    let target = all_transports(
        &service,
        &dir,
        &secret,
        14,
        "column.read",
        json!({"space":sid(1),"column":sid(3)}),
    )
    .await;
    let restored=all_transports(&service,&dir,&secret,15,"card.restore",json!({"space":sid(1),"card":sid(9),"expected":card["version"],"column":sid(3),"expected_column":target["version"]})).await;
    assert_eq!(restored["titles"], edited["titles"]);
    let selection = json!({"kind":"objects","columns":[sid(2)],"cards":[]});
    let preview = all_transports(
        &service,
        &dir,
        &secret,
        16,
        "trash.inspect",
        json!({"space":sid(1),"selection":selection}),
    )
    .await;
    assert_eq!(preview["cards"], json!([]));
    let purge = json!({"space":sid(1),"selection":selection,"expected":preview["version"]});
    all_transports(&service, &dir, &secret, 17, "trash.purge", purge.clone()).await;
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage);
    let replay = post(
        &service,
        "history.clear",
        &request(6, "history.clear", clear),
        &secret,
    )
    .await;
    assert_eq!(replay["replayed"], true);
    let replay = post(
        &service,
        "trash.purge",
        &request(17, "trash.purge", purge),
        &secret,
    )
    .await;
    assert_eq!(replay["replayed"], true);
    let card = post(
        &service,
        "card.read",
        &request(18, "card.read", json!({"space":sid(1),"card":sid(9)})),
        &secret,
    )
    .await;
    assert_eq!(card["result"], restored);
    let h = post(
        &service,
        "card.history",
        &request(
            19,
            "card.history",
            json!({"space":sid(1),"card":sid(9),"limit":100,"after":null,"expected":null}),
        ),
        &secret,
    )
    .await;
    assert_eq!(h["result"]["versions"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn cleanup_permissions_apply_before_preview_mutation_and_receipt_replay() {
    let (dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    all_transports(
        &service,
        &dir,
        &secret,
        1,
        "space.create",
        json!({"space":sid(1),"name":"Board"}),
    )
    .await;
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
    let readonly = key(&member);
    let h = all_transports(
        &service,
        &dir,
        &readonly,
        2,
        "history.inspect",
        json!({"space":sid(1),"card":null}),
    )
    .await;
    let t = all_transports(
        &service,
        &dir,
        &readonly,
        3,
        "trash.inspect",
        json!({"space":sid(1),"selection":{"kind":"all"}}),
    )
    .await;
    for (op, parameters) in [
        (
            "history.clear",
            json!({"space":sid(1),"card":null,"expected":h["version"]}),
        ),
        (
            "trash.purge",
            json!({"space":sid(1),"selection":{"kind":"all"},"expected":t["version"]}),
        ),
    ] {
        let body = request(4, op, parameters);
        for result in [
            post(&service, op, &body, &readonly).await,
            serde_json::from_str::<Value>(&ipc::invoke(&service, op, &body, &readonly).await)
                .unwrap(),
            cli(&dir, &readonly, op, &body),
        ] {
            assert_eq!(result["error"]["code"], "access_denied");
        }
    }
    let access = post(
        &service,
        "access.inspect",
        &request(5, "access.inspect", json!({"subject":member.subject()})),
        &secret,
    )
    .await;
    let revoked = post(
        &service,
        "access.revoke",
        &request(
            6,
            "access.revoke",
            json!({"subject":member.subject(),"expected":access["result"]["version"]}),
        ),
        &secret,
    )
    .await;
    assert_eq!(revoked["status"], "ok");
    let result = post(
        &service,
        "history.inspect",
        &request(2, "history.inspect", json!({"space":sid(1),"card":null})),
        &readonly,
    )
    .await;
    assert_eq!(result["error"]["code"], "access_denied");
}
