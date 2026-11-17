use super::*;
use serde_json::{Value, json};
const SPACE: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
#[path = "recovery_tests.rs"]
mod recovery_tests;
async fn api(service: &Service, key: &str, op: &str, p: Value) -> Value {
    let mut id = [0; 16];
    getrandom::fill(&mut id).unwrap();
    let reply = service
        .dispatch(
            op,
            json!({"id":id,"operation":op,"parameters":p})
                .to_string()
                .as_bytes(),
            key,
        )
        .await;
    match reply {
        CommandResponse::Ok(v) => v.result,
        CommandResponse::Error(e) => panic!("{op}: {:?}", e.error.code),
    }
}
async fn host(host: &DeviceHost<Arc<Memory>>, key: &str, op: &str, p: Value) -> Value {
    match host.dispatch(op, p.to_string().as_bytes(), key).await {
        CommandResponse::Ok(v) => v.result,
        CommandResponse::Error(e) => panic!("{op}: {:?}", e.error.code),
    }
}
async fn selection(service: &Service, key: &str, pairing: &str, selected: bool) {
    let c = api(
        service,
        key,
        "sync.inspect",
        json!({"space":SPACE,"pairing":pairing}),
    )
    .await;
    api(service,key,"sync.select",json!({"space":SPACE,"pairing":pairing,"selected":selected,"expected":c["selection_version"]})).await;
}
async fn pause(service: &Service, key: &str, pairing: &str, paused: bool) {
    let c = api(
        service,
        key,
        "sync.inspect",
        json!({"space":SPACE,"pairing":pairing}),
    )
    .await;
    api(
        service,
        key,
        "sync.pause",
        json!({"space":SPACE,"pairing":pairing,"paused":paused,"expected":c["pause_version"]}),
    )
    .await;
}

#[tokio::test]
async fn paired_exchange_obeys_readonly_pause_reverse_restart_and_preserves_local_data() {
    for writable in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let (server_db, admin) = setup(&dir.path().join("server")).await;
        let (client_db, owner) = setup(&dir.path().join("client")).await;
        let service = Service::new(server_db.clone());
        let client_service = Service::new(client_db.clone());
        api(
            &service,
            &admin,
            "space.create",
            json!({"space":SPACE,"name":"Shared"}),
        )
        .await;
        api(
            &client_service,
            &owner,
            "space.create",
            json!({"space":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","name":"Local preserved"}),
        )
        .await;
        let server = Arc::new(DeviceHost::new(
            service.clone(),
            Arc::new(Memory::default()),
        ));
        let store = Arc::new(Memory::default());
        let client = DeviceHost::new(client_service.clone(), store.clone());
        let (listener, _) = server
            .start("127.0.0.1:0".parse().unwrap(), &admin)
            .await
            .unwrap();
        let offer = host(&server, &admin, "offer.create", json!({})).await;
        host(
            &client,
            &owner,
            "session.prepare",
            json!({"address":listener.address,"offer":offer["offer"],"label":"Peer"}),
        )
        .await;
        let pending = host(&client, &owner, "session.submit", json!({})).await;
        let pairing = pending["request"].as_str().unwrap().to_owned();
        api(&service,&admin,"pairing.confirm",json!({"request":pairing,"expected":pending["version"],"scope":{"kind":"spaces","spaces":{SPACE:if writable{"read_write"}else{"read"}}},"expires_at_unix_ms":null})).await;
        host(&client, &owner, "session.refresh", json!({})).await;
        selection(&client_service, &owner, &pairing, true).await;
        assert_eq!(
            host(&client, &owner, "exchange.run", json!({"space":SPACE})).await["state"],
            "remote_unselected"
        );
        assert_eq!(
            api(
                &client_service,
                &owner,
                "space.list",
                json!({"include_trashed":false})
            )
            .await["spaces"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        selection(&service, &admin, &pairing, true).await;
        let first = host(&client, &owner, "exchange.run", json!({"space":SPACE})).await;
        assert_eq!(
            first["state"],
            if writable { "up_to_date" } else { "read_only" }
        );
        let imported = api(
            &client_service,
            &owner,
            "space.read",
            json!({"space":SPACE}),
        )
        .await;
        assert_eq!(imported["names"], json!(["Shared"]));
        api(
            &client_service,
            &owner,
            "space.rename",
            json!({"space":SPACE,"name":"Client edit","expected":imported["version"]}),
        )
        .await;
        host(&client, &owner, "exchange.run", json!({"space":SPACE})).await;
        let remote = api(&service, &admin, "space.read", json!({"space":SPACE})).await;
        assert_eq!(
            remote["names"],
            json!([if writable { "Client edit" } else { "Shared" }])
        );
        pause(&client_service, &owner, &pairing, true).await;
        assert!(
            matches!(client.dispatch("exchange.run",json!({"space":SPACE}).to_string().as_bytes(),&owner).await,CommandResponse::Error(e) if e.error.code==ErrorCode::Conflict)
        );
        selection(&client_service, &owner, &pairing, false).await;
        selection(&client_service, &owner, &pairing, true).await;
        assert_eq!(
            api(
                &client_service,
                &owner,
                "sync.inspect",
                json!({"space":SPACE,"pairing":pairing})
            )
            .await["paused"],
            true
        );
        drop(client);
        let client = DeviceHost::new(client_service.clone(), store);
        assert!(
            matches!(client.dispatch("exchange.run",json!({"space":SPACE}).to_string().as_bytes(),&owner).await,CommandResponse::Error(e) if e.error.code==ErrorCode::Conflict)
        );
        pause(&client_service, &owner, &pairing, false).await;
        pause(&service, &admin, &pairing, true).await;
        assert_eq!(
            host(&client, &owner, "exchange.run", json!({"space":SPACE})).await["state"],
            "remote_paused"
        );
        pause(&service, &admin, &pairing, false).await;
        host(&client, &owner, "exchange.run", json!({"space":SPACE})).await;
        assert_eq!(
            api(
                &client_service,
                &owner,
                "space.read",
                json!({"space":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"})
            )
            .await["names"],
            json!(["Local preserved"])
        );
        let request = api(
            &service,
            &admin,
            "pairing.inspect",
            json!({"request":pairing}),
        )
        .await;
        api(
            &service,
            &admin,
            "pairing.unlink",
            json!({"request":pairing,"expected":request["version"]}),
        )
        .await;
        assert!(
            matches!(client.dispatch("exchange.run",json!({"space":SPACE}).to_string().as_bytes(),&owner).await,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
        );
        assert!(
            api(
                &client_service,
                &owner,
                "space.read",
                json!({"space":SPACE})
            )
            .await["contents"]
                .is_object()
        );
        listener.shutdown().await;
        client_db.shutdown().await.unwrap();
        server_db.shutdown().await.unwrap();
    }
}
