use super::spaces::{cli, setup_spaces, sid};
use super::*;

async fn setup_cards() -> (TempDir, Storage, Service, Arc<PendingAccess>) {
    let (dir, storage, service, admin) = setup_spaces().await;
    for (id, operation, params) in [
        (250, "space.create", json!({"space":sid(1),"name":"Board"})),
        (
            251,
            "column.create",
            json!({"space":sid(1),"column":sid(2),"name":"To do"}),
        ),
    ] {
        assert_eq!(
            post(
                &service,
                operation,
                &request(id, operation, params),
                &key(&admin)
            )
            .await["status"],
            "ok"
        );
    }
    (dir, storage, service, admin)
}
fn creation() -> String {
    request(
        1,
        "card.create",
        json!({"space":sid(1),"column":sid(2),"card":sid(3),"title":"Ёж 空間 😀","description":"<b>raw</b>\n**text**"}),
    )
}
#[tokio::test]
async fn explicit_completion_and_undo_share_http_ipc_cli_receipts_across_restart() {
    let (dir, storage, service, admin) = setup_cards().await;
    let secret = key(&admin);
    let created = post(&service, "card.create", &creation(), &secret).await;
    let body = |id, completed: Value, version: &Value| {
        request(
            id,
            "card.set_completed",
            json!({"space":sid(1),"card":sid(3),"completed":completed,"expected":version}),
        )
    };
    for value in [json!("true"), json!(1), Value::Null] {
        let invalid = body(2, value, &created["result"]["version"]);
        let denied = post(&service, "card.set_completed", &invalid, &secret).await;
        assert_eq!(denied["error"]["code"], "invalid_request");
        assert_eq!(cli(&dir, &secret, "card.set_completed", &invalid), denied);
        assert_eq!(
            serde_json::from_str::<Value>(
                &ipc::invoke(&service, "card.set_completed", &invalid, &secret).await
            )
            .unwrap(),
            denied
        );
    }
    let completion = body(2, json!(true), &created["result"]["version"]);
    let done = post(&service, "card.set_completed", &completion, &secret).await;
    assert_eq!(done["result"]["completed"], json!([true]));
    let repeated: Value = serde_json::from_str(
        &ipc::invoke(&service, "card.set_completed", &completion, &secret).await,
    )
    .unwrap();
    assert_eq!(repeated["replayed"], true);
    assert_eq!(
        cli(&dir, &secret, "card.set_completed", &completion),
        repeated
    );
    let undo = body(3, json!(false), &done["result"]["version"]);
    let undone = cli(&dir, &secret, "card.set_completed", &undo);
    assert_eq!(undone["result"]["completed"], json!([false]));
    let stale = body(4, json!(true), &created["result"]["version"]);
    let denied = post(&service, "card.set_completed", &stale, &secret).await;
    assert_eq!(denied["error"]["code"], "stale_state");
    assert_eq!(
        serde_json::from_str::<Value>(
            &ipc::invoke(&service, "card.set_completed", &stale, &secret).await
        )
        .unwrap(),
        denied
    );
    assert_eq!(cli(&dir, &secret, "card.set_completed", &stale), denied);
    storage.shutdown().await.unwrap();
    assert_eq!(
        cli(&dir, &secret, "card.set_completed", &completion),
        repeated
    );
    assert_eq!(
        cli(
            &dir,
            &secret,
            "card.read",
            &request(5, "card.read", json!({"space":sid(1),"card":sid(3)}))
        )["result"],
        undone["result"]
    );
}
#[tokio::test]
async fn card_lifecycle_and_history_share_receipts_across_http_ipc_cli_and_restart() {
    let (dir, storage, service, admin) = setup_cards().await;
    let secret = key(&admin);
    let created = post(&service, "card.create", &creation(), &secret).await;
    assert_eq!(created["status"], "ok");
    let repeated: Value =
        serde_json::from_str(&ipc::invoke(&service, "card.create", &creation(), &secret).await)
            .unwrap();
    assert_eq!(repeated["replayed"], true);
    assert_eq!(cli(&dir, &secret, "card.create", &creation()), repeated);
    let edit = request(
        2,
        "card.edit",
        json!({"space":sid(1),"card":sid(3),"title":"Updated","description":"","expected":created["result"]["version"]}),
    );
    let edited = cli(&dir, &secret, "card.edit", &edit);
    assert_eq!(edited["result"]["titles"], json!(["Updated"]));
    assert!(edited["result"].get("text_versions").is_none());
    let history = cli(
        &dir,
        &secret,
        "card.history",
        &request(
            20,
            "card.history",
            json!({"space":sid(1),"card":sid(3),"after":null,"limit":100,"expected":null}),
        ),
    );
    assert_eq!(history["result"]["versions"].as_array().unwrap().len(), 2);
    let old = request(
        3,
        "card.trash",
        json!({"space":sid(1),"card":sid(3),"expected":created["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "card.trash", &old, &secret).await["error"]["code"],
        "stale_state"
    );
    let trash = request(
        4,
        "card.trash",
        json!({"space":sid(1),"card":sid(3),"expected":edited["result"]["version"]}),
    );
    let trashed = post(&service, "card.trash", &trash, &secret).await;
    assert_eq!(trashed["result"]["state"], "trashed");
    assert_eq!(
        cli(
            &dir,
            &secret,
            "card.list",
            &request(
                5,
                "card.list",
                json!({"space":sid(1),"include_trashed":false})
            )
        )["result"]["cards"],
        json!([])
    );
    let restore = request(
        6,
        "card.restore",
        json!({"space":sid(1),"card":sid(3),"expected":trashed["result"]["version"]}),
    );
    let restored: Value =
        serde_json::from_str(&ipc::invoke(&service, "card.restore", &restore, &secret).await)
            .unwrap();
    assert_eq!(restored["result"]["state"], "active");
    assert_eq!(
        post(&service, "card.trash", &trash, &secret).await["result"],
        trashed["result"]
    );
    let read = request(7, "card.read", json!({"space":sid(1),"card":sid(3)}));
    assert_eq!(
        post(&service, "card.read", &read, &secret).await["result"],
        restored["result"]
    );
    storage.shutdown().await.unwrap();
    assert_eq!(
        cli(&dir, &secret, "card.read", &read)["result"],
        restored["result"]
    );
    assert_eq!(cli(&dir, &secret, "card.edit", &edit)["replayed"], true);
}

#[tokio::test]
async fn invalid_cards_and_readonly_and_revoked_receipts_are_denied_at_all_entries() {
    let (dir, storage, service, admin) = setup_cards().await;
    let secret = key(&admin);
    for p in [
        json!({"space":sid(1),"column":sid(2),"card":sid(3),"title":"","description":""}),
        json!({"space":sid(1),"column":sid(2),"card":"A".repeat(32),"title":"A","description":""}),
        json!({"space":sid(1),"column":"0".repeat(32),"card":sid(3),"title":"A","description":""}),
        json!({"space":sid(1),"column":sid(2),"card":sid(3),"title":"A","description":"","subject":admin.subject()}),
    ] {
        let body = request(1, "card.create", p);
        let denied = post(&service, "card.create", &body, &secret).await;
        assert_eq!(denied["error"]["code"], "invalid_request");
        assert_eq!(cli(&dir, &secret, "card.create", &body), denied);
        assert_eq!(
            serde_json::from_str::<Value>(
                &ipc::invoke(&service, "card.create", &body, &secret).await
            )
            .unwrap(),
            denied
        );
    }
    let duplicate = creation().replace("\"title\":", "\"title\":\"duplicate\",\"title\":");
    assert_eq!(
        post(&service, "card.create", &duplicate, &secret).await["error"]["code"],
        "invalid_request"
    );
    let created = post(&service, "card.create", &creation(), &secret).await;
    assert_eq!(created["status"], "ok");
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
    let edit = request(
        2,
        "card.edit",
        json!({"space":sid(1),"card":sid(3),"title":"no","description":"","expected":created["result"]["version"]}),
    );
    let read = request(3, "card.read", json!({"space":sid(1),"card":sid(3)}));
    assert_eq!(cli(&dir, &readonly, "card.read", &read)["status"], "ok");
    let denied = post(&service, "card.edit", &edit, &readonly).await;
    assert_eq!(denied["error"]["code"], "access_denied");
    assert_eq!(cli(&dir, &readonly, "card.edit", &edit), denied);
    assert_eq!(
        serde_json::from_str::<Value>(&ipc::invoke(&service, "card.edit", &edit, &readonly).await)
            .unwrap(),
        denied
    );
    let completion = request(
        6,
        "card.set_completed",
        json!({"space":sid(1),"card":sid(3),"completed":true,"expected":created["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "card.set_completed", &completion, &readonly).await,
        denied
    );
    assert_eq!(
        cli(&dir, &readonly, "card.set_completed", &completion),
        denied
    );
    assert_eq!(
        serde_json::from_str::<Value>(
            &ipc::invoke(&service, "card.set_completed", &completion, &readonly).await
        )
        .unwrap(),
        denied
    );
    let access = post(
        &service,
        "access.inspect",
        &request(4, "access.inspect", json!({"subject":member.subject()})),
        &secret,
    )
    .await;
    let revoke = request(
        5,
        "access.revoke",
        json!({"subject":member.subject(),"expected":access["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "access.revoke", &revoke, &secret).await["status"],
        "ok"
    );
    let denied = post(&service, "card.read", &read, &readonly).await;
    assert_eq!(denied["error"]["code"], "access_denied");
    assert_eq!(cli(&dir, &readonly, "card.read", &read), denied);
    assert_eq!(
        serde_json::from_str::<Value>(&ipc::invoke(&service, "card.read", &read, &readonly).await)
            .unwrap(),
        denied
    );
}
