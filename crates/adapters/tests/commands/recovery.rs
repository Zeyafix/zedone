use super::spaces::{cli, sid};
use super::*;
async fn call(service: &Service, secret: &str, id: u8, op: &str, p: Value) -> Value {
    post(service, op, &request(id, op, p), secret).await
}
async fn state(service: &Service, secret: &str) -> Value {
    call(
        service,
        secret,
        99,
        "space.recovery.inspect",
        json!({"space":sid(1)}),
    )
    .await["result"]
        .clone()
}

#[tokio::test]
async fn recovery_http_ipc_cli_restart_reverse_and_old_receipts_use_one_authorized_operation() {
    let (dir, storage, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    let initial = post(&service, "card.create", &super::cards::creation(), &secret).await;
    let exported = call(
        &service,
        &secret,
        2,
        "space.export",
        json!({"space":sid(1)}),
    )
    .await;
    let old_edit = request(
        3,
        "card.edit",
        json!({"space":sid(1),"card":sid(3),"title":"Позже","description":"Late","expected":initial["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "card.edit", &old_edit, &secret).await["status"],
        "ok"
    );
    let current = state(&service, &secret).await;
    assert_eq!(current["paused"], false);
    let body = request(
        4,
        "space.recovery.restore",
        json!({"space":sid(1),"archive":exported["result"]["archive"],"expected":current["version"]}),
    );
    let restored = post(&service, "space.recovery.restore", &body, &secret).await;
    assert_eq!(restored["status"], "ok", "{restored}");
    assert_eq!(restored["result"]["space"], sid(1));
    let repeated: Value = serde_json::from_str(
        &ipc::invoke(&service, "space.recovery.restore", &body, &secret).await,
    )
    .unwrap();
    assert_eq!(repeated["replayed"], true);
    assert_eq!(repeated["result"], restored["result"]);
    assert_eq!(cli(&dir, &secret, "card.edit", &old_edit)["replayed"], true);
    let card = call(
        &service,
        &secret,
        5,
        "card.read",
        json!({"space":sid(1),"card":sid(3)}),
    )
    .await;
    assert_eq!(card["result"]["titles"], initial["result"]["titles"]);
    assert_eq!(state(&service, &secret).await["paused"], true);
    let safety = call(
        &service,
        &secret,
        6,
        "space.recovery.read_backup",
        json!({"space":sid(1),"safety":restored["result"]["safety"]}),
    )
    .await;
    assert_eq!(safety["status"], "ok");
    let reverse = request(
        7,
        "space.recovery.restore",
        json!({"space":sid(1),"archive":safety["result"]["archive"],"expected":state(&service,&secret).await["version"]}),
    );
    let reversed = cli(&dir, &secret, "space.recovery.restore", &reverse);
    assert_eq!(reversed["status"], "ok", "{reversed}");
    assert_ne!(reversed["result"]["safety"], restored["result"]["safety"]);
    let again = post(&service, "space.recovery.restore", &body, &secret).await;
    assert_eq!(again["replayed"], true);
    assert_eq!(
        call(
            &service,
            &secret,
            8,
            "card.read",
            json!({"space":sid(1),"card":sid(3)})
        )
        .await["result"]["titles"],
        json!(["Позже"])
    );
    let pause = state(&service, &secret).await;
    let resume = request(
        9,
        "space.recovery.pause",
        json!({"space":sid(1),"paused":false,"expected":pause["pause_version"]}),
    );
    assert_eq!(
        post(&service, "space.recovery.pause", &resume, &secret).await["status"],
        "ok"
    );
    assert_eq!(state(&service, &secret).await["paused"], false);
    drop(service);
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage.clone());
    assert_eq!(
        post(&service, "space.recovery.restore", &body, &secret).await["replayed"],
        true
    );
    assert_eq!(state(&service, &secret).await["paused"], false);
    assert_eq!(
        call(
            &service,
            &secret,
            10,
            "space.list",
            json!({"include_trashed":true})
        )
        .await["result"]["spaces"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    drop(service);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn malformed_stale_and_unauthorized_restoration_cannot_create_a_safety_or_change_the_space() {
    let (dir, storage, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    post(&service, "card.create", &super::cards::creation(), &secret).await;
    let exported = call(
        &service,
        &secret,
        2,
        "space.export",
        json!({"space":sid(1)}),
    )
    .await;
    let observed = state(&service, &secret).await;
    for archive in [
        json!("bad"),
        json!("ff"),
        json!("00".repeat(1024 * 1024 + 1)),
    ] {
        assert_eq!(
            call(
                &service,
                &secret,
                3,
                "space.recovery.restore",
                json!({"space":sid(1),"archive":archive,"expected":observed["version"]})
            )
            .await["error"]["code"],
            "invalid_request"
        );
    }
    let rename = call(&service, &secret, 4, "space.read", json!({"space":sid(1)})).await;
    call(
        &service,
        &secret,
        5,
        "space.rename",
        json!({"space":sid(1),"name":"Changed","expected":rename["result"]["version"]}),
    )
    .await;
    assert_eq!(call(&service,&secret,6,"space.recovery.restore",json!({"space":sid(1),"archive":exported["result"]["archive"],"expected":observed["version"]})).await["error"]["code"],"stale_state");
    assert_eq!(state(&service, &secret).await["backup"], Value::Null);
    assert!(!dir.path().join("device.db.backups").exists());
    let valid = request(
        7,
        "space.recovery.restore",
        json!({"space":sid(1),"archive":exported["result"]["archive"],"expected":state(&service,&secret).await["version"]}),
    );
    assert_eq!(
        post(&service, "space.recovery.restore", &valid, &"00".repeat(32)).await["error"]["code"],
        "access_denied"
    );
    drop(service);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn read_only_access_cannot_restore_and_current_permissions_are_required_even_for_receipts() {
    let (_dir, storage, service, admin) = super::cards::setup_cards().await;
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
    let admin_key = key(&admin);
    let exported = call(
        &service,
        &admin_key,
        1,
        "space.export",
        json!({"space":sid(1)}),
    )
    .await;
    let current = state(&service, &member_key).await;
    let body = request(
        2,
        "space.recovery.restore",
        json!({"space":sid(1),"archive":exported["result"]["archive"],"expected":current["version"]}),
    );
    assert_eq!(
        post(&service, "space.recovery.restore", &body, &member_key).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        post(&service, "space.recovery.restore", &body, &admin_key).await["status"],
        "ok"
    );
    let (a, subject) = (admin.clone(), admin.subject().to_string());
    let replacement = Arc::new(AccessKey::generate().unwrap());
    let next_key = replacement.clone();
    let fresh_key: String = replacement
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    storage
        .timed_transact(move |ctx, clock| {
            ctx.replace_access(
                &a.credential(),
                clock,
                &subject,
                &next_key,
                &GrantScope::AdministrationAndSpaces(BTreeMap::from([(
                    sid(1),
                    SpacePermission::Read,
                )])),
                None,
            )
        })
        .await
        .unwrap();
    assert_eq!(
        post(&service, "space.recovery.restore", &body, &admin_key).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(state(&service, &fresh_key).await["paused"], true);
    assert_eq!(
        post(&service, "space.recovery.restore", &body, &fresh_key).await["error"]["code"],
        "access_denied"
    );
    drop(service);
    storage.shutdown().await.unwrap();
}
