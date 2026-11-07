use super::cards::{creation, setup_cards};
use super::spaces::{cli, sid};
use super::*;

async fn all_entries(
    service: &Service,
    dir: &TempDir,
    secret: &str,
    operation: &str,
    body: &str,
) -> Value {
    let first = post(service, operation, body, secret).await;
    let repeated: Value =
        serde_json::from_str(&ipc::invoke(service, operation, body, secret).await).unwrap();
    if first["status"] == "ok" {
        assert_eq!(repeated["replayed"], true);
        assert_eq!(first["result"], repeated["result"]);
    } else {
        assert_eq!(first, repeated);
    }
    assert_eq!(cli(dir, secret, operation, body), repeated);
    first
}

#[tokio::test]
async fn editing_sessions_and_restore_share_original_receipts_on_http_ipc_cli_and_restart() {
    let (dir, storage, service, admin) = setup_cards().await;
    let secret = key(&admin);
    let created = post(&service, "card.create", &creation(), &secret).await;
    let target = |version: &Value| json!({"space":sid(1),"card":sid(3),"expected":version});
    let begin = request(
        2,
        "card.begin_editing",
        target(&created["result"]["version"]),
    );
    all_entries(&service, &dir, &secret, "card.begin_editing", &begin).await;
    let session = "02".repeat(16);
    let save = request(
        3,
        "card.autosave",
        json!({"space":sid(1),"card":sid(3),"session":session,"title":"Saved title","description":"Saved description","expected":created["result"]["version"]}),
    );
    let saved = all_entries(&service, &dir, &secret, "card.autosave", &save).await;
    let history = request(
        4,
        "card.history",
        json!({"space":sid(1),"card":sid(3),"after":null,"limit":100,"expected":null}),
    );
    let pending = all_entries(&service, &dir, &secret, "card.history", &history).await;
    // One-shot CLI opens never close a live host's editing session.
    assert_eq!(pending["result"]["versions"].as_array().unwrap().len(), 1);
    let close = request(
        5,
        "card.finish_editing",
        json!({"space":sid(1),"card":sid(3),"session":session,"expected":saved["result"]["version"]}),
    );
    all_entries(&service, &dir, &secret, "card.finish_editing", &close).await;
    let restore = request(
        6,
        "card.restore_text",
        json!({"space":sid(1),"card":sid(3),"text_version":pending["result"]["versions"][0]["id"],"expected":saved["result"]["version"]}),
    );
    let restored = all_entries(&service, &dir, &secret, "card.restore_text", &restore).await;
    assert_eq!(restored["result"]["titles"], created["result"]["titles"]);
    let history = request(
        7,
        "card.history",
        json!({"space":sid(1),"card":sid(3),"after":null,"limit":100,"expected":null}),
    );
    let versions = all_entries(&service, &dir, &secret, "card.history", &history).await;
    assert_eq!(versions["result"]["versions"].as_array().unwrap().len(), 3);
    storage.shutdown().await.unwrap();
    let reopened = Storage::open(dir.path().join("device.db")).await.unwrap();
    reopened.recover_editing_sessions().await.unwrap();
    let service = Service::new(reopened.clone());
    for (operation, body) in [
        ("card.begin_editing", begin),
        ("card.autosave", save),
        ("card.finish_editing", close),
        ("card.restore_text", restore),
    ] {
        assert_eq!(
            all_entries(&service, &dir, &secret, operation, &body).await["replayed"],
            true
        );
    }
    assert_eq!(
        post(
            &service,
            "card.read",
            &request(8, "card.read", json!({"space":sid(1),"card":sid(3)})),
            &secret
        )
        .await["result"],
        restored["result"]
    );
    reopened.shutdown().await.unwrap();
}

#[tokio::test]
async fn history_commands_reject_forged_context_and_readonly_or_revoked_access_on_every_entry() {
    let (dir, storage, service, admin) = setup_cards().await;
    let secret = key(&admin);
    let created = post(&service, "card.create", &creation(), &secret).await;
    let history = post(
        &service,
        "card.history",
        &request(
            2,
            "card.history",
            json!({"space":sid(1),"card":sid(3),"after":null,"limit":100,"expected":null}),
        ),
        &secret,
    )
    .await;
    let params = json!({"space":sid(1),"card":sid(3),"expected":created["result"]["version"]});
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Spaces([(sid(1), SpacePermission::Read)].into()),
                None,
            )
        })
        .await
        .unwrap();
    for (index, operation) in [
        "card.begin_editing",
        "card.autosave",
        "card.finish_editing",
        "card.restore_text",
    ]
    .into_iter()
    .enumerate()
    {
        let mut p = params.clone();
        if operation == "card.autosave" || operation == "card.finish_editing" {
            p["session"] = json!("03".repeat(16));
        }
        if operation == "card.autosave" {
            p["title"] = json!("Title");
            p["description"] = json!("Description");
        }
        if operation == "card.restore_text" {
            p["text_version"] = history["result"]["versions"][0]["id"].clone();
        }
        let id = 10 + index as u8;
        assert_eq!(
            all_entries(
                &service,
                &dir,
                &key(&member),
                operation,
                &request(id, operation, p.clone())
            )
            .await["error"]["code"],
            "access_denied"
        );
        for field in ["subject", "clock", "changed_ms", "sequence"] {
            let mut forged = p.clone();
            forged[field] = json!("forged");
            assert_eq!(
                all_entries(
                    &service,
                    &dir,
                    &secret,
                    operation,
                    &request(id, operation, forged)
                )
                .await["error"]["code"],
                "invalid_request"
            );
        }
        let good = request(id, operation, p);
        let duplicate = good.replace("\"card\":", "\"card\":\"duplicate\",\"card\":");
        assert_eq!(
            all_entries(&service, &dir, &secret, operation, &duplicate).await["error"]["code"],
            "invalid_request"
        );
    }
    let writer = Arc::new(PendingAccess::generate().unwrap());
    let (a, w) = (admin.clone(), writer.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &w,
                &GrantScope::Spaces([(sid(1), SpacePermission::ReadWrite)].into()),
                None,
            )
        })
        .await
        .unwrap();
    let writer_key = key(&writer);
    let begin = request(20, "card.begin_editing", params);
    assert_eq!(
        post(&service, "card.begin_editing", &begin, &writer_key).await["status"],
        "ok"
    );
    let access = post(
        &service,
        "access.inspect",
        &request(21, "access.inspect", json!({"subject":writer.subject()})),
        &secret,
    )
    .await;
    let revoke = request(
        22,
        "access.revoke",
        json!({"subject":writer.subject(),"expected":access["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "access.revoke", &revoke, &secret).await["status"],
        "ok"
    );
    assert_eq!(
        all_entries(&service, &dir, &writer_key, "card.begin_editing", &begin).await["error"]["code"],
        "access_denied"
    );
}
