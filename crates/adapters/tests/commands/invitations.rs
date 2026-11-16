use super::{
    spaces::{cli, sid},
    *,
};

#[tokio::test]
async fn invitations_share_http_ipc_cli_replay_without_admitting_a_device_or_leaking_its_code() {
    let (dir, storage, service, admin, member) = setup().await;
    let secret = key(&admin);
    let code = "11".repeat(32);
    let verifier: Vec<u8> = vec![
        160, 43, 110, 0, 83, 115, 85, 137, 83, 155, 78, 186, 49, 97, 211, 197, 156, 4, 80, 67, 192,
        21, 58, 56, 239, 144, 188, 212, 105, 92, 114, 175,
    ];
    let create = request(
        1,
        "invitation.create",
        json!({"invitation":sid(1),"verifier":verifier}),
    );
    let created = post(&service, "invitation.create", &create, &secret).await;
    assert_eq!(created["result"]["status"], "open", "{created}");
    assert_eq!(
        created["result"]["expires_at_unix_ms"].as_i64().unwrap()
            - created["result"]["created_at_unix_ms"].as_i64().unwrap(),
        600000
    );
    let replay: Value =
        serde_json::from_str(&ipc::invoke(&service, "invitation.create", &create, &secret).await)
            .unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], created["result"]);
    assert_eq!(cli(&dir, &secret, "invitation.create", &create), replay);
    let inspect = request(2, "invitation.inspect", json!({"invitation":sid(1)}));
    let cancel = request(
        3,
        "invitation.cancel",
        json!({"invitation":sid(1),"expected":created["result"]["version"]}),
    );
    let cancelled = cli(&dir, &secret, "invitation.cancel", &cancel);
    assert_eq!(cancelled["result"]["status"], "cancelled");
    let current = post(&service, "invitation.inspect", &inspect, &secret).await;
    assert_eq!(current["result"], cancelled["result"]);
    let list = request(4, "invitation.list", json!({"after":null}));
    let entries = post(&service, "invitation.list", &list, &secret).await;
    assert_eq!(entries["result"]["entries"], json!([cancelled["result"]]));
    for (operation, body) in [
        ("invitation.create", &create),
        ("invitation.inspect", &inspect),
        ("invitation.cancel", &cancel),
        ("invitation.list", &list),
    ] {
        for denied in [
            key(&member),
            code.clone(),
            verifier.iter().map(|b| format!("{b:02x}")).collect(),
        ] {
            assert_eq!(
                post(&service, operation, body, &denied).await["error"]["code"],
                "access_denied"
            );
        }
    }
    for forbidden in ["invitation.request", "invitation.confirm"] {
        assert_eq!(
            post(
                &service,
                forbidden,
                &request(5, forbidden, json!({})),
                &code
            )
            .await["status"],
            "error"
        );
    }
    for parameters in [
        json!({"invitation":"bad","verifier":verifier}),
        json!({"invitation":sid(2),"verifier":vec![0;32]}),
        json!({"invitation":sid(2),"verifier":verifier,"code":code}),
        json!({"invitation":sid(2),"verifier":verifier,"expires_at_unix_ms":0}),
    ] {
        assert_eq!(
            post(
                &service,
                "invitation.create",
                &request(6, "invitation.create", parameters),
                &secret
            )
            .await["error"]["code"],
            "invalid_request"
        );
    }
    let stale = request(
        7,
        "invitation.cancel",
        json!({"invitation":sid(1),"expected":created["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "invitation.cancel", &stale, &secret).await["error"]["code"],
        "stale_state"
    );
    let mut mismatch: Value = serde_json::from_str(&create).unwrap();
    mismatch["parameters"]["invitation"] = json!(sid(2));
    assert_eq!(
        post(
            &service,
            "invitation.create",
            &mismatch.to_string(),
            &secret
        )
        .await["error"]["code"],
        "command_mismatch"
    );
    for response in [&created, &cancelled, &current, &entries] {
        let text = response.to_string();
        assert!(!text.contains("verifier"));
        assert!(!text.contains(&code));
    }
    drop(service);
    storage.shutdown().await.unwrap();
    let bytes = std::fs::read(dir.path().join("device.db")).unwrap();
    assert!(!bytes.windows(32).any(|window| window == [0x11; 32]));
    assert!(
        !bytes
            .windows(code.len())
            .any(|window| window == code.as_bytes())
    );
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage.clone());
    assert_eq!(
        post(&service, "invitation.create", &create, &secret).await,
        replay
    );
    assert_eq!(
        post(
            &service,
            "invitation.inspect",
            &request(8, "invitation.inspect", json!({"invitation":sid(1)})),
            &secret
        )
        .await["result"]["status"],
        "cancelled"
    );
    assert_eq!(
        post(
            &service,
            "access.list",
            &request(9, "access.list", json!({"after":null})),
            &secret
        )
        .await["result"]["entries"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    storage.shutdown().await.unwrap();
}
