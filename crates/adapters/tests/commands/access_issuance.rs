use super::spaces::{cli, setup_spaces, sid};
use super::*;

fn parameters(access: &PendingAccess, mode: &str) -> Value {
    json!({"subject":access.subject(),"verifier":access.registration_verifier(),
        "scope":{"kind":"spaces","spaces":{sid(1):mode}},"expires_at_unix_ms":null})
}

async fn create_space(service: &Service, secret: &str) {
    let result = post(
        service,
        "space.create",
        &request(1, "space.create", json!({"space":sid(1),"name":"Private"})),
        secret,
    )
    .await;
    assert_eq!(result["status"], "ok", "{result}");
}

async fn read(service: &Service, secret: &str) -> Value {
    let nonce = PendingAccess::generate().unwrap();
    let id: Vec<u8> = (0..16)
        .map(|i| u8::from_str_radix(&nonce.subject()[i * 2..i * 2 + 2], 16).unwrap())
        .collect();
    post(
        service,
        "space.read",
        &json!({"id":id,"operation":"space.read","parameters":{"space":sid(1)}}).to_string(),
        secret,
    )
    .await
}

fn assert_error(value: &Value, code: &str) {
    assert_eq!(value, &json!({"status":"error","error":{"code":code}}));
}

#[tokio::test]
async fn issuance_rotation_revocation_and_regrant_replay_on_http_ipc_cli_and_restart() {
    let (dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    create_space(&service, &secret).await;
    let member = PendingAccess::generate().unwrap();
    let body = request(2, "access.issue", parameters(&member, "read"));
    let issued = post(&service, "access.issue", &body, &secret).await;
    assert_eq!(issued["status"], "ok", "{issued}");
    assert_eq!(issued["replayed"], false);
    assert_eq!(issued["result"]["scope"]["spaces"][sid(1)], "read");
    assert_eq!(issued["result"]["expires_at_unix_ms"], Value::Null);
    assert_eq!(read(&service, &key(&member)).await["status"], "ok");
    let current = read(&service, &secret).await;
    let rename = request(
        201,
        "space.rename",
        json!({"space":sid(1),"name":"Denied","expected":current["result"]["version"]}),
    );
    assert_error(
        &post(&service, "space.rename", &rename, &key(&member)).await,
        "access_denied",
    );
    assert_error(
        &post(
            &service,
            "access.list",
            &request(3, "access.list", json!({"after":null})),
            &key(&member),
        )
        .await,
        "access_denied",
    );
    let raw_registration: String = member
        .registration_verifier()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_error(&read(&service, &raw_registration).await, "access_denied");

    let rotated = PendingAccess::generate().unwrap();
    let mut replacement = parameters(&rotated, "read_write");
    replacement["subject"] = json!(member.subject());
    replacement["expected"] = issued["result"]["version"].clone();
    let replace = request(4, "access.replace", replacement);
    let changed: Value =
        serde_json::from_str(&ipc::invoke(&service, "access.replace", &replace, &secret).await)
            .unwrap();
    assert_eq!(changed["status"], "ok", "{changed}");
    assert_ne!(changed["result"]["version"], issued["result"]["version"]);
    assert_error(&read(&service, &key(&member)).await, "access_denied");
    let renamed = post(&service, "space.rename", &rename, &key(&rotated)).await;
    assert_eq!(renamed["result"]["names"], json!(["Denied"]));
    let cli_reply = cli(&dir, &secret, "access.replace", &replace);
    assert_eq!(cli_reply["replayed"], true);
    assert_eq!(cli_reply["result"], changed["result"]);

    let revoke = request(
        5,
        "access.revoke",
        json!({"subject":member.subject(),"expected":changed["result"]["version"]}),
    );
    let revoked = post(&service, "access.revoke", &revoke, &secret).await;
    assert_eq!(revoked["result"]["revoked"], true);
    assert_error(&read(&service, &key(&rotated)).await, "access_denied");
    // Lost replies return the ORIGINAL issuance/rotation, without reviving keys.
    assert_eq!(
        post(&service, "access.issue", &body, &secret).await["result"],
        issued["result"]
    );
    assert_eq!(
        post(&service, "access.replace", &replace, &secret).await["result"],
        changed["result"]
    );
    assert_error(&read(&service, &key(&rotated)).await, "access_denied");

    let fresh = PendingAccess::generate().unwrap();
    let mut regrant = parameters(&fresh, "read");
    regrant["subject"] = json!(member.subject());
    regrant["expected"] = revoked["result"]["version"].clone();
    let regrant = request(6, "access.replace", regrant);
    assert_eq!(
        post(&service, "access.replace", &regrant, &secret).await["status"],
        "ok"
    );
    assert_eq!(
        post(&service, "access.revoke", &revoke, &secret).await["result"],
        revoked["result"]
    );
    assert_eq!(
        read(&service, &key(&fresh)).await["result"]["names"],
        json!(["Denied"])
    );

    // Receipts and authoritative SQL contain no recipient bearer secret.
    for access in [&member, &rotated, &fresh] {
        assert!(!body.contains(&key(access)));
        assert!(!replace.contains(&key(access)));
        assert!(!regrant.contains(&key(access)));
    }
    drop(service);
    storage.shutdown().await.unwrap();
    let bytes = std::fs::read(dir.path().join("device.db")).unwrap();
    for access in [&member, &rotated, &fresh] {
        assert!(
            !bytes
                .windows(32)
                .any(|window| window == access.credential().0)
        );
        assert!(
            !bytes
                .windows(64)
                .any(|window| window == key(access).as_bytes())
        );
    }
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage.clone());
    let again = post(&service, "access.issue", &body, &secret).await;
    assert_eq!(again["replayed"], true);
    assert_eq!(again["result"], issued["result"]);
    assert_eq!(read(&service, &key(&fresh)).await["status"], "ok");
    for access in [&member, &rotated] {
        assert_error(&read(&service, &key(access)).await, "access_denied");
    }
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn stale_collisions_retired_keys_and_mismatches_cannot_overwrite_access() {
    let (_dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    create_space(&service, &secret).await;
    let member = PendingAccess::generate().unwrap();
    let issue = request(2, "access.issue", parameters(&member, "read"));
    let issued = post(&service, "access.issue", &issue, &secret).await;
    let mut duplicate: Value = serde_json::from_str(&issue).unwrap();
    duplicate["id"] = json!(vec![3; 16]);
    assert_error(
        &post(&service, "access.issue", &duplicate.to_string(), &secret).await,
        "invalid_request",
    );
    duplicate["id"] = json!(vec![2; 16]);
    duplicate["parameters"]["scope"]["spaces"][sid(1)] = json!("read_write");
    assert_error(
        &post(&service, "access.issue", &duplicate.to_string(), &secret).await,
        "command_mismatch",
    );
    let revoke = request(
        4,
        "access.revoke",
        json!({"subject":member.subject(),"expected":issued["result"]["version"]}),
    );
    let revoked = post(&service, "access.revoke", &revoke, &secret).await;
    let replacement = PendingAccess::generate().unwrap();
    let mut params = parameters(&replacement, "read_write");
    params["subject"] = json!(member.subject());
    params["expected"] = issued["result"]["version"].clone();
    assert_error(
        &post(
            &service,
            "access.replace",
            &request(5, "access.replace", params.clone()),
            &secret,
        )
        .await,
        "stale_state",
    );
    params["expected"] = revoked["result"]["version"].clone();
    params["verifier"] = json!(member.registration_verifier());
    assert_error(
        &post(
            &service,
            "access.replace",
            &request(6, "access.replace", params.clone()),
            &secret,
        )
        .await,
        "invalid_request",
    );
    // No partial un-revoke, key activation or failed-command receipt.
    assert_error(&read(&service, &key(&member)).await, "access_denied");
    params["verifier"] = json!(replacement.registration_verifier());
    let changed = post(
        &service,
        "access.replace",
        &request(6, "access.replace", params),
        &secret,
    )
    .await;
    assert_eq!(changed["status"], "ok", "{changed}");
    assert_eq!(read(&service, &key(&replacement)).await["status"], "ok");
    assert_error(
        &post(
            &service,
            "access.revoke",
            &request(
                7,
                "access.revoke",
                json!({"subject":member.subject(),"expected":revoked["result"]["version"]}),
            ),
            &secret,
        )
        .await,
        "stale_state",
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn explicit_scopes_and_strict_requests_reject_accidental_or_unauthorized_grants() {
    let (_dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    create_space(&service, &secret).await;
    let recipient = PendingAccess::generate().unwrap();
    let original = parameters(&recipient, "read");
    let mut cases = Vec::new();
    let mut p = original.clone();
    p["scope"]["spaces"] = json!({});
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p["scope"]["spaces"] = json!({"not-an-id":"read"});
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p["scope"]["spaces"] = json!({sid(2):"read"});
    cases.push((p, "not_found"));
    let mut p = original.clone();
    p["verifier"] = json!(vec![0; 32]);
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p["verifier"] = json!(vec![1; 31]);
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p["subject"] = json!("A".repeat(32));
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p["expires_at_unix_ms"] = json!(-1);
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p["expires_at_unix_ms"] = json!(1);
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p["key"] = json!(key(&recipient));
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p.as_object_mut().unwrap().remove("scope");
    cases.push((p, "invalid_request"));
    let mut p = original.clone();
    p["scope"]["kind"] = json!("administration");
    cases.push((p, "invalid_request"));
    for (index, (params, code)) in cases.into_iter().enumerate() {
        assert_error(
            &post(
                &service,
                "access.issue",
                &request(10 + index as u8, "access.issue", params),
                &secret,
            )
            .await,
            code,
        );
    }
    let inspect = request(40, "access.inspect", json!({"subject":recipient.subject()}));
    assert_error(
        &post(&service, "access.inspect", &inspect, &secret).await,
        "not_found",
    );
    assert_error(
        &post(
            &service,
            "access.list",
            &request(41, "access.list", json!({"after":""})),
            &secret,
        )
        .await,
        "invalid_request",
    );
    assert_error(
        &post(
            &service,
            "access.issue",
            &request(42, "access.issue", original),
            &key(&recipient),
        )
        .await,
        "access_denied",
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn last_admin_is_preserved_and_administration_is_separate_from_space_permissions() {
    let (_dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    create_space(&service, &secret).await;
    let current = post(
        &service,
        "access.inspect",
        &inspect(2, admin.subject()),
        &secret,
    )
    .await;
    let replacement = PendingAccess::generate().unwrap();
    let mut params = parameters(&replacement, "read_write");
    params["subject"] = json!(admin.subject());
    params["expected"] = current["result"]["version"].clone();
    assert_error(
        &post(
            &service,
            "access.replace",
            &request(3, "access.replace", params.clone()),
            &secret,
        )
        .await,
        "invalid_request",
    );
    assert_eq!(
        post(
            &service,
            "access.inspect",
            &inspect(4, admin.subject()),
            &secret
        )
        .await["result"],
        current["result"]
    );
    let second_admin = PendingAccess::generate().unwrap();
    let second = json!({"subject":second_admin.subject(),"verifier":second_admin.registration_verifier(),"scope":{"kind":"administration"},"expires_at_unix_ms":null});
    assert_eq!(
        post(
            &service,
            "access.issue",
            &request(5, "access.issue", second),
            &secret
        )
        .await["status"],
        "ok"
    );
    assert_error(&read(&service, &key(&second_admin)).await, "access_denied");
    let changed = post(
        &service,
        "access.replace",
        &request(6, "access.replace", params),
        &secret,
    )
    .await;
    assert_eq!(changed["status"], "ok");
    assert_error(
        &post(
            &service,
            "access.list",
            &request(7, "access.list", json!({"after":null})),
            &secret,
        )
        .await,
        "access_denied",
    );
    assert_error(
        &post(
            &service,
            "access.list",
            &request(8, "access.list", json!({"after":null})),
            &key(&replacement),
        )
        .await,
        "access_denied",
    );
    assert_eq!(read(&service, &key(&replacement)).await["status"], "ok");
    assert_eq!(
        post(
            &service,
            "access.list",
            &request(9, "access.list", json!({"after":null})),
            &key(&second_admin)
        )
        .await["status"],
        "ok"
    );
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn access_list_has_bounded_ordered_pages_and_receipts_are_not_current_state() {
    let (_dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    create_space(&service, &secret).await;
    let body = request(2, "access.list", json!({"after":null}));
    let initial = post(&service, "access.list", &body, &secret).await;
    assert_eq!(initial["result"]["entries"].as_array().unwrap().len(), 1);
    for id in 3..58 {
        let recipient = PendingAccess::generate().unwrap();
        let params = parameters(&recipient, "read");
        assert_eq!(
            post(
                &service,
                "access.issue",
                &request(id, "access.issue", params),
                &secret
            )
            .await["status"],
            "ok"
        );
    }
    assert_eq!(
        post(&service, "access.list", &body, &secret).await["result"],
        initial["result"]
    );
    let page = post(
        &service,
        "access.list",
        &request(60, "access.list", json!({"after":null})),
        &secret,
    )
    .await;
    let entries = page["result"]["entries"].as_array().unwrap();
    assert_eq!(entries.len(), 50);
    let cursor = page["result"]["next"].as_str().unwrap();
    assert_eq!(entries[49]["subject"], cursor);
    let next = post(
        &service,
        "access.list",
        &request(61, "access.list", json!({"after":cursor})),
        &secret,
    )
    .await;
    assert_eq!(next["result"]["entries"].as_array().unwrap().len(), 6);
    assert_eq!(next["result"]["next"], Value::Null);
    let subjects = entries
        .iter()
        .chain(next["result"]["entries"].as_array().unwrap())
        .map(|v| v["subject"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert!(subjects.windows(2).all(|pair| pair[0] < pair[1]));
    for entry in entries {
        assert!(entry.get("verifier").is_none());
        assert!(entry.get("key").is_none());
    }
    let empty = post(
        &service,
        "access.list",
        &request(62, "access.list", json!({"after":"f".repeat(32)})),
        &secret,
    )
    .await;
    assert_eq!(empty["result"]["entries"], json!([]));
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn concurrent_replacements_of_one_preview_allow_only_one_new_key() {
    let (_dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    create_space(&service, &secret).await;
    let recipient = PendingAccess::generate().unwrap();
    let issued = post(
        &service,
        "access.issue",
        &request(2, "access.issue", parameters(&recipient, "read")),
        &secret,
    )
    .await;
    let a = PendingAccess::generate().unwrap();
    let b = PendingAccess::generate().unwrap();
    let replacement = |id, pending| {
        let mut p = parameters(pending, "read_write");
        p["subject"] = json!(recipient.subject());
        p["expected"] = issued["result"]["version"].clone();
        request(id, "access.replace", p)
    };
    let ra = replacement(3, &a);
    let rb = replacement(4, &b);
    let (ra, rb) = tokio::join!(
        post(&service, "access.replace", &ra, &secret),
        post(&service, "access.replace", &rb, &secret)
    );
    let (winner, loser, winning_key, losing_key) = if ra["status"] == "ok" {
        (&ra, &rb, &a, &b)
    } else {
        (&rb, &ra, &b, &a)
    };
    assert_eq!(winner["status"], "ok");
    assert_error(loser, "stale_state");
    assert_eq!(read(&service, &key(winning_key)).await["status"], "ok");
    for pending in [&recipient, losing_key] {
        assert_error(&read(&service, &key(pending)).await, "access_denied");
    }
    storage.shutdown().await.unwrap();
}
