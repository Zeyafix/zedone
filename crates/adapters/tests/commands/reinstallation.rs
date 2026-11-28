use super::spaces::{cli, setup_spaces, sid};
use super::*;

async fn call(service: &Service, secret: &str, id: u8, op: &str, p: Value) -> Value {
    post(service, op, &request(id, op, p), secret).await
}

#[tokio::test]
async fn reinstallation_uses_http_ipc_cli_fresh_permissions_receipts_and_explicit_pause_after_restart()
 {
    let (_source_dir, source, old_service, old_admin) = super::cards::setup_cards().await;
    let old_key = key(&old_admin);
    post(
        &old_service,
        "card.create",
        &super::cards::creation(),
        &old_key,
    )
    .await;
    let archive = call(
        &old_service,
        &old_key,
        2,
        "space.export",
        json!({"space":sid(1)}),
    )
    .await["result"]["archive"]
        .clone();
    let (dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    let inspected = call(
        &service,
        &secret,
        3,
        "space.recovery.inspect_missing",
        json!({"archive":archive}),
    )
    .await;
    assert_eq!(inspected["status"], "ok", "{inspected}");
    let body = request(
        4,
        "space.recovery.restore_missing",
        json!({"space":sid(1),"archive":archive,"expected":inspected["result"]["version"]}),
    );
    for op in [
        "space.recovery.inspect_missing",
        "space.recovery.restore_missing",
    ] {
        let denied = if op.ends_with("inspect_missing") {
            request(5, op, json!({"archive":archive}))
        } else {
            body.clone()
        };
        assert_eq!(
            post(&service, op, &denied, &old_key).await["error"]["code"],
            "access_denied"
        );
    }
    let restored: Value = serde_json::from_str(
        &ipc::invoke(&service, "space.recovery.restore_missing", &body, &secret).await,
    )
    .unwrap();
    assert_eq!(restored["status"], "ok", "{restored}");
    assert_eq!(restored["result"]["space"], sid(1));
    assert!(!dir.path().join("device.db.backups").exists());
    assert_eq!(
        cli(&dir, &secret, "space.recovery.restore_missing", &body)["replayed"],
        true
    );
    let state = call(
        &service,
        &secret,
        6,
        "space.recovery.inspect",
        json!({"space":sid(1)}),
    )
    .await["result"]
        .clone();
    assert_eq!(state["paused"], true);
    assert_eq!(state["backup"], Value::Null);

    assert_eq!(
        call(
            &service,
            &old_key,
            7,
            "card.read",
            json!({"space":sid(1),"card":sid(3)})
        )
        .await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        call(
            &old_service,
            &old_key,
            7,
            "card.read",
            json!({"space":sid(1),"card":sid(3)})
        )
        .await["status"],
        "ok"
    );
    let member = PendingAccess::generate().unwrap();
    let issued = call(&service, &secret, 8, "access.issue", json!({"subject":member.subject(),"verifier":member.registration_verifier(),"scope":{"kind":"spaces","spaces":{sid(1):"read_write"}},"expires_at_unix_ms":null})).await;
    assert_eq!(issued["status"], "ok", "{issued}");
    assert_eq!(
        call(
            &service,
            &key(&member),
            9,
            "card.read",
            json!({"space":sid(1),"card":sid(3)})
        )
        .await["status"],
        "ok"
    );
    assert_eq!(
        call(
            &service,
            &secret,
            10,
            "access.revoke",
            json!({"subject":member.subject(),"expected":issued["result"]["version"]})
        )
        .await["status"],
        "ok"
    );
    assert_eq!(
        call(
            &service,
            &key(&member),
            11,
            "card.read",
            json!({"space":sid(1),"card":sid(3)})
        )
        .await["error"]["code"],
        "access_denied"
    );
    drop(service);
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage.clone());
    assert_eq!(
        post(&service, "space.recovery.restore_missing", &body, &secret).await["replayed"],
        true
    );
    let after = call(
        &service,
        &secret,
        12,
        "space.recovery.inspect",
        json!({"space":sid(1)}),
    )
    .await;
    assert_eq!(after["result"], state);
    assert_eq!(
        call(
            &service,
            &secret,
            13,
            "space.recovery.pause",
            json!({"space":sid(1),"paused":false,"expected":state["pause_version"]})
        )
        .await["status"],
        "ok"
    );
    assert_eq!(
        post(&service, "space.recovery.restore_missing", &body, &secret).await["replayed"],
        true
    );
    assert_eq!(
        call(
            &service,
            &secret,
            14,
            "space.recovery.inspect",
            json!({"space":sid(1)})
        )
        .await["result"]["paused"],
        false
    );
    let (a, subject) = (admin.clone(), admin.subject().to_owned());
    let replacement = Arc::new(AccessKey::generate().unwrap());
    let next = replacement.clone();
    let rotated: String = replacement
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
                &next,
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
        post(&service, "space.recovery.restore_missing", &body, &secret).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        post(&service, "space.recovery.restore_missing", &body, &rotated).await["error"]["code"],
        "access_denied"
    );
    drop(service);
    storage.shutdown().await.unwrap();
    drop(old_service);
    source.shutdown().await.unwrap();
}

#[tokio::test]
async fn existing_and_trashed_spaces_malformed_archives_and_changed_confirmation_are_rejected() {
    let (_dir, storage, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    let archive = call(
        &service,
        &secret,
        1,
        "space.export",
        json!({"space":sid(1)}),
    )
    .await["result"]["archive"]
        .clone();
    for trash in [false, true] {
        if trash {
            let current = call(&service, &secret, 2, "space.read", json!({"space":sid(1)})).await;
            assert_eq!(
                call(
                    &service,
                    &secret,
                    3,
                    "space.trash",
                    json!({"space":sid(1),"expected":current["result"]["version"]})
                )
                .await["status"],
                "ok"
            );
        }
        assert_eq!(
            call(
                &service,
                &secret,
                4,
                "space.recovery.inspect_missing",
                json!({"archive":archive})
            )
            .await["error"]["code"],
            "stale_state"
        );
        assert_eq!(
            call(
                &service,
                &secret,
                5,
                "space.recovery.restore_missing",
                json!({"space":sid(1),"archive":archive,"expected":vec![0;32]})
            )
            .await["error"]["code"],
            "stale_state"
        );
    }
    for malformed in [
        json!("ff"),
        json!("BAD"),
        json!("00".repeat(1024 * 1024 + 1)),
    ] {
        assert_eq!(
            call(
                &service,
                &secret,
                6,
                "space.recovery.inspect_missing",
                json!({"archive":malformed})
            )
            .await["error"]["code"],
            "invalid_request"
        );
    }
    drop(service);
    storage.shutdown().await.unwrap();
}
