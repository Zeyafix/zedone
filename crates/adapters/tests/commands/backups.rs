use super::spaces::{cli, sid};
use super::*;
async fn call(s: &Service, key: &str, id: u8, op: &str, p: Value) -> Value {
    post(s, op, &request(id, op, p), key).await
}
#[tokio::test]
async fn daily_retry_delete_read_share_fresh_permissions_receipts_and_restart_on_http_ipc_cli() {
    let (dir, db, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    db.daily_tick(true).await.unwrap();
    let state = call(
        &service,
        &secret,
        1,
        "space.backups.inspect",
        json!({"space":sid(1)}),
    )
    .await;
    assert_eq!(state["status"], "ok");
    assert_eq!(state["result"]["copies"].as_array().unwrap().len(), 1);
    let copy = state["result"]["copies"][0]["id"].clone();
    let read = request(2, "space.backups.read", json!({"space":sid(1),"copy":copy}));
    let http = post(&service, "space.backups.read", &read, &secret).await;
    assert_eq!(http["status"], "ok");
    assert!(
        http["result"]["archive"]
            .as_str()
            .unwrap()
            .starts_with("5a45445350414345")
    );
    let ipc = ipc::invoke(&service, "space.backups.read", &read, &secret).await;
    assert_eq!(serde_json::from_str::<Value>(&ipc).unwrap(), http);
    assert_eq!(cli(&dir, &secret, "space.backups.read", &read), http);
    let limited = Arc::new(PendingAccess::generate().unwrap());
    let l = limited.clone();
    let a = admin.clone();
    db.timed_transact(move |ctx, clock| {
        ctx.issue_access(
            &a.credential(),
            clock,
            &l,
            &GrantScope::Spaces([(sid(1), SpacePermission::Read)].into_iter().collect()),
            None,
        )
    })
    .await
    .unwrap();
    let limited_key = key(&limited);
    let remove = request(
        3,
        "space.backups.delete",
        json!({"space":sid(1),"copy":copy}),
    );
    assert_eq!(
        post(&service, "space.backups.delete", &remove, &limited_key).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        call(
            &service,
            &limited_key,
            4,
            "space.backups.retry",
            json!({"space":sid(1),"failed_sequence":1})
        )
        .await["error"]["code"],
        "access_denied"
    );
    let removed = post(&service, "space.backups.delete", &remove, &secret).await;
    assert_eq!(removed["status"], "ok");
    let replay = cli(&dir, &secret, "space.backups.delete", &remove);
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], removed["result"]);
    assert_eq!(
        call(
            &service,
            &secret,
            5,
            "space.backups.inspect",
            json!({"space":sid(1)})
        )
        .await["result"]["copies"]
            .as_array()
            .unwrap()
            .len(),
        0
    );
    assert_eq!(
        post(&service, "space.backups.read", &read, &secret).await["error"]["code"],
        "not_found"
    );
    drop(service);
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(db.clone());
    assert_eq!(
        post(&service, "space.backups.delete", &remove, &secret).await["replayed"],
        true
    );
    assert!(
        call(
            &service,
            &secret,
            6,
            "space.backups.inspect",
            json!({"space":sid(1)})
        )
        .await["result"]["last_success"]
            .is_object()
    );
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn automatic_startup_and_first_save_use_one_persistent_daily_limit() {
    let (dir, db, service, admin) = super::cards::setup_cards().await;
    let service = service.with_daily_backups();
    let secret = key(&admin);
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let state = call(
                &service,
                &secret,
                1,
                "space.backups.inspect",
                json!({"space":sid(1)}),
            )
            .await;
            if state["result"]["copies"]
                .as_array()
                .is_some_and(|v| v.len() == 1)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    assert_eq!(
        post(&service, "card.create", &super::cards::creation(), &secret).await["status"],
        "ok"
    );
    let state = call(
        &service,
        &secret,
        2,
        "space.backups.inspect",
        json!({"space":sid(1)}),
    )
    .await;
    assert_eq!(state["result"]["status"], "daily_limit");
    drop(service);
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(db.clone()).with_daily_backups();
    assert_eq!(
        call(
            &service,
            &secret,
            3,
            "space.backups.inspect",
            json!({"space":sid(1)})
        )
        .await["result"]["copies"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    drop(service);
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn manual_retry_replays_original_result_after_lost_reply_and_preserves_fresh_revocation() {
    let (dir, db, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    let root = dir.path().join("device.db.backups");
    std::fs::write(&root, b"unavailable folder").unwrap();
    let _ = db.daily_tick(true).await;
    let failed = call(
        &service,
        &secret,
        1,
        "space.backups.inspect",
        json!({"space":sid(1)}),
    )
    .await;
    let sequence = failed["result"]["failed_sequence"].as_u64().unwrap();
    assert_eq!(failed["result"]["status"], "waiting");
    assert!(failed["result"]["retry_after_ms"].as_u64().unwrap() > 0);
    std::fs::remove_file(&root).unwrap();
    let retry = request(
        2,
        "space.backups.retry",
        json!({"space":sid(1),"failed_sequence":sequence}),
    );
    let saved = post(&service, "space.backups.retry", &retry, &secret).await;
    assert_eq!(saved["result"]["status"], "saved");
    assert_eq!(saved["result"]["copies"].as_array().unwrap().len(), 1);
    let replay = serde_json::from_str::<Value>(
        &ipc::invoke(&service, "space.backups.retry", &retry, &secret).await,
    )
    .unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], saved["result"]);
    assert_eq!(
        cli(&dir, &secret, "space.backups.retry", &retry)["result"],
        saved["result"]
    );
    let other = request(
        3,
        "space.backups.retry",
        json!({"space":sid(1),"failed_sequence":sequence}),
    );
    assert_eq!(
        post(&service, "space.backups.retry", &other, &secret).await["result"]["copies"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    let restricted = Arc::new(PendingAccess::generate().unwrap());
    let r = restricted.clone();
    let a = admin.clone();
    db.timed_transact(move |ctx, clock| {
        ctx.issue_access(
            &a.credential(),
            clock,
            &r,
            &GrantScope::Spaces([(sid(1), SpacePermission::ReadWrite)].into_iter().collect()),
            None,
        )
    })
    .await
    .unwrap();
    let restricted_key = key(&restricted);
    let accepted = post(&service, "space.backups.retry", &other, &restricted_key).await;
    assert_eq!(accepted["status"], "ok");
    let inspected = call(
        &service,
        &secret,
        4,
        "access.inspect",
        json!({"subject":restricted.subject()}),
    )
    .await;
    assert_eq!(inspected["status"], "ok", "{inspected}");
    let revoke = request(
        5,
        "access.revoke",
        json!({"subject":restricted.subject(),"expected":inspected["result"]["version"]}),
    );
    let revoked = post(&service, "access.revoke", &revoke, &secret).await;
    assert_eq!(revoked["status"], "ok", "{revoked}");
    assert_eq!(
        post(&service, "space.backups.retry", &other, &restricted_key).await["error"]["code"],
        "access_denied"
    );
    db.shutdown().await.unwrap();
}
