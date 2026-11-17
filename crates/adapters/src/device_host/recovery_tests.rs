use super::*;
use axum::{http::StatusCode, middleware};
use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

#[derive(Default)]
struct Fault {
    mode: AtomicU8,
    pulls: AtomicUsize,
    waiting: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

async fn fault(
    axum::extract::State(fault): axum::extract::State<Arc<Fault>>,
    request: axum::extract::Request,
    next: middleware::Next,
) -> axum::response::Response {
    let path = request.uri().path().to_owned();
    let mode = fault.mode.load(Ordering::SeqCst);
    let pull = path.ends_with("/sync.pull");
    let fail_pull = pull && fault.pulls.fetch_add(1, Ordering::SeqCst) == 1;
    if mode == 1 && fail_pull {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    let response = next.run(request).await;
    if (mode == 2 && fail_pull) || (mode == 3 && path.ends_with("/sync.push")) {
        return StatusCode::BAD_GATEWAY.into_response();
    }
    if mode == 8 && pull {
        fault.waiting.notify_one();
        fault.release.notified().await;
    }
    if ((4..=5).contains(&mode) && path.ends_with("/sync.catalog"))
        || ((6..=7).contains(&mode) && path.ends_with("/sync.push"))
    {
        let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
            .await
            .unwrap();
        let mut value: Value = serde_json::from_slice(&bytes).unwrap();
        match mode {
            4 => value["result"]["protocol"] = json!(999),
            5 => value["result"]["format"] = json!(999),
            6 => value["result"]["base"] = json!([]),
            7 => value["result"]["complete"] = json!(false),
            _ => unreachable!(),
        }
        return axum::Json(value).into_response();
    }
    response
}

use axum::response::IntoResponse;

async fn delivered(pair: &Pair, name: &str) {
    tokio::time::timeout(std::time::Duration::from_secs(8), async {
        loop {
            let value = api(
                &pair.service,
                &pair.admin,
                "space.read",
                json!({"space":SPACE}),
            )
            .await;
            if value["names"] == json!([name]) {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("automatic delivery did not resume");
}

#[tokio::test]
async fn pause_resume_while_page_is_in_flight_cannot_commit_a_stale_delivery() {
    let pair = Pair::new().await;
    pair.fault.mode.store(8, Ordering::SeqCst);
    let client = pair.client.clone();
    let key = pair.owner.clone();
    let delivery = tokio::spawn(async move {
        client
            .dispatch(
                "exchange.run",
                json!({"space":SPACE}).to_string().as_bytes(),
                &key,
            )
            .await
    });
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        pair.fault.waiting.notified(),
    )
    .await
    .unwrap();
    pause(&pair.client.service, &pair.owner, &pair.pairing, true).await;
    pause(&pair.client.service, &pair.owner, &pair.pairing, false).await;
    pair.fault.release.notify_one();
    assert!(
        matches!(delivery.await.unwrap(),CommandResponse::Error(e) if e.error.code == ErrorCode::StaleState)
    );
    assert!(
        !pair
            .client_db
            .transact(|tx| Ok(zedone_core::sync::SyncStore::sync_space_exists(tx, SPACE)?))
            .await
            .unwrap()
    );
    pair.fault.mode.store(0, Ordering::SeqCst);
    pair.client.exchange_automatic(&pair.owner).await.unwrap();
    assert_eq!(
        api(
            &pair.client.service,
            &pair.owner,
            "space.read",
            json!({"space":SPACE})
        )
        .await["names"],
        json!(["Shared"])
    );
    pair.finish().await;
}

#[tokio::test]
async fn actual_listener_outage_and_server_restart_resume_on_the_controller_timer_without_repairing()
 {
    use crate::device_control::Controller;
    let mut pair = Pair::new().await;
    pair.client.exchange_automatic(&pair.owner).await.unwrap();
    let listener = pair.listener.take().unwrap();
    let address = listener.address;
    listener.shutdown().await;
    let before = api(
        &pair.client.service,
        &pair.owner,
        "space.read",
        json!({"space":SPACE}),
    )
    .await;
    api(
        &pair.client.service,
        &pair.owner,
        "space.rename",
        json!({"space":SPACE,"name":"Saved offline","expected":before["version"]}),
    )
    .await;
    pair.run_error(ErrorCode::Unavailable).await;
    pair.restart().await;
    let key_path = pair.directory.path().join("local.key");
    std::fs::write(&key_path, &pair.owner).unwrap();
    let automatic = Controller::new(pair.client.clone()).with_automatic(key_path);
    pair.server_db.clone().shutdown().await.unwrap();
    pair.server_db = Storage::open(pair.directory.path().join("server"))
        .await
        .unwrap();
    pair.service = Service::new(pair.server_db.clone());
    pair.server = Arc::new(DeviceHost::new(
        pair.service.clone(),
        (*pair.server.store).clone(),
    ));
    let (listener, _) = pair.server.start(address, &pair.admin).await.unwrap();
    pair.listener = Some(listener);
    delivered(&pair, "Saved offline").await;
    assert_eq!(
        api(
            &pair.service,
            &pair.admin,
            "pairing.inspect",
            json!({"request":pair.pairing})
        )
        .await["status"],
        "confirmed"
    );
    drop(automatic);
    pair.finish().await;
}

#[tokio::test]
async fn incompatible_protocol_is_distinct_from_invalid_input_and_still_requires_current_access() {
    let pair = Pair::new().await;
    for (op, parameters) in [
        (
            "sync.pull",
            json!({"protocol":999,"space":SPACE,"known":[]}),
        ),
        (
            "sync.push",
            json!({"protocol":999,"space":SPACE,"base":[],"changes":[]}),
        ),
    ] {
        let body = json!({"id":vec![77;16],"operation":op,"parameters":parameters}).to_string();
        assert!(
            matches!(pair.client.peer_command(op,body.as_bytes()).await.unwrap(),CommandResponse::Error(e) if e.error.code == ErrorCode::IncompatibleSync)
        );
        let listener = pair.listener.as_ref().unwrap();
        let identity = channel::load_identity(&pair.server.store)
            .await
            .unwrap()
            .unwrap();
        let denied = network::command(
            listener.address,
            identity.fingerprint(),
            &format!("/v1/commands/{op}"),
            &[0; 32],
            body.as_bytes(),
        )
        .await
        .unwrap();
        assert!(
            matches!(denied,CommandResponse::Error(e) if e.error.code == ErrorCode::AccessDenied)
        );
    }
    for mode in [4, 5] {
        pair.fault.mode.store(mode, Ordering::SeqCst);
        pair.run_error(ErrorCode::IncompatibleSync).await;
        pair.fault.mode.store(0, Ordering::SeqCst);
        pair.client.exchange_automatic(&pair.owner).await.unwrap();
        assert_eq!(
            api(
                &pair.client.service,
                &pair.owner,
                "space.read",
                json!({"space":SPACE})
            )
            .await["names"],
            json!(["Shared"])
        );
    }
    pair.finish().await;
}

struct Pair {
    directory: tempfile::TempDir,
    server_db: Storage,
    client_db: Storage,
    service: Service,
    client: Arc<DeviceHost<Arc<Memory>>>,
    store: Arc<Memory>,
    admin: String,
    owner: String,
    pairing: String,
    listener: Option<network::SecureListener>,
    server: Arc<DeviceHost<Arc<Memory>>>,
    fault: Arc<Fault>,
}

impl Pair {
    async fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let (server_db, admin) = setup(&directory.path().join("server")).await;
        let (client_db, owner) = setup(&directory.path().join("client")).await;
        let service = Service::new(server_db.clone());
        api(
            &service,
            &admin,
            "space.create",
            json!({"space":SPACE,"name":"Shared"}),
        )
        .await;
        let server = Arc::new(DeviceHost::new(
            service.clone(),
            Arc::new(Memory::default()),
        ));
        let store = Arc::new(Memory::default());
        let client = Arc::new(DeviceHost::new(
            Service::new(client_db.clone()),
            store.clone(),
        ));
        let (initial, _) = server
            .start("127.0.0.1:0".parse().unwrap(), &admin)
            .await
            .unwrap();
        let address = initial.address;
        initial.shutdown().await;
        let identity = channel::load_identity(&server.store)
            .await
            .unwrap()
            .unwrap();
        let fault_state = Arc::new(Fault::default());
        let router = server
            .router()
            .layer(middleware::from_fn_with_state(fault_state.clone(), fault));
        let listener = network::SecureListener::start(address, Arc::new(identity), router)
            .await
            .unwrap();
        let offer = host(&server, &admin, "offer.create", json!({})).await;
        host(
            &client,
            &owner,
            "session.prepare",
            json!({"address":address,"offer":offer["offer"],"label":"Recovery"}),
        )
        .await;
        let pending = host(&client, &owner, "session.submit", json!({})).await;
        let pairing = pending["request"].as_str().unwrap().to_owned();
        api(&service,&admin,"pairing.confirm",json!({"request":pairing,"expected":pending["version"],"scope":{"kind":"spaces","spaces":{SPACE:"read_write"}},"expires_at_unix_ms":null})).await;
        host(&client, &owner, "session.refresh", json!({})).await;
        selection(&service, &admin, &pairing, true).await;
        selection(&client.service, &owner, &pairing, true).await;
        Self {
            directory,
            server_db,
            client_db,
            service,
            client,
            store,
            admin,
            owner,
            pairing,
            listener: Some(listener),
            server,
            fault: fault_state,
        }
    }
    async fn restart(&mut self) {
        self.client_db.clone().shutdown().await.unwrap();
        self.client_db = Storage::open(self.directory.path().join("client"))
            .await
            .unwrap();
        self.client = Arc::new(DeviceHost::new(
            Service::new(self.client_db.clone()),
            self.store.clone(),
        ));
        let status = host(&self.client, &self.owner, "session.status", json!({})).await;
        assert_eq!(status["status"], "connected");
        assert_eq!(status["request"], self.pairing);
    }
    async fn run_error(&self, expected: ErrorCode) {
        assert!(
            matches!(self.client.dispatch("exchange.run",json!({"space":SPACE}).to_string().as_bytes(),&self.owner).await,
            CommandResponse::Error(e) if e.error.code == expected)
        );
    }
    async fn finish(self) {
        self.listener.unwrap().shutdown().await;
        self.client_db.shutdown().await.unwrap();
        self.server_db.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn interrupted_pages_and_lost_push_ack_resume_after_database_restart_without_duplicates() {
    for mode in [1, 2, 3, 6, 7] {
        let mut pair = Pair::new().await;
        pair.server_db
            .transact(|tx| {
                use zedone_core::spaces::SpaceStore;
                for i in 0..270 {
                    tx.rename_space(SPACE, &format!("Source {i}"))?;
                }
                Ok(())
            })
            .await
            .unwrap();
        pair.fault.mode.store(mode, Ordering::SeqCst);
        pair.run_error(ErrorCode::OutcomeUnknown).await;
        if mode <= 2 {
            assert!(
                !pair
                    .client_db
                    .transact(|tx| Ok(zedone_core::sync::SyncStore::sync_space_exists(tx, SPACE)?))
                    .await
                    .unwrap()
            );
        }
        pair.restart().await;
        pair.fault.mode.store(0, Ordering::SeqCst);
        pair.client.exchange_automatic(&pair.owner).await.unwrap();
        let imported = api(
            &pair.client.service,
            &pair.owner,
            "space.read",
            json!({"space":SPACE}),
        )
        .await;
        assert_eq!(imported["names"], json!(["Source 269"]));
        api(
            &pair.client.service,
            &pair.owner,
            "space.rename",
            json!({"space":SPACE,"name":"Offline edit","expected":imported["version"]}),
        )
        .await;
        pair.fault.mode.store(3, Ordering::SeqCst);
        pair.run_error(ErrorCode::OutcomeUnknown).await;
        let delivered = api(
            &pair.service,
            &pair.admin,
            "space.read",
            json!({"space":SPACE}),
        )
        .await;
        assert_eq!(delivered["names"], json!(["Offline edit"]));
        api(
            &pair.service,
            &pair.admin,
            "space.rename",
            json!({"space":SPACE,"name":"Later edit","expected":delivered["version"]}),
        )
        .await;
        pair.restart().await;
        pair.fault.mode.store(0, Ordering::SeqCst);
        pair.client.exchange_automatic(&pair.owner).await.unwrap();
        let count = |tx: &mut zedone_storage::WriteContext<'_>| {
            Ok(tx.read().document(SPACE)?.get_changes(&[]).len())
        };
        let expected = pair.server_db.transact(count).await.unwrap();
        for _ in 0..3 {
            pair.client.exchange_automatic(&pair.owner).await.unwrap();
        }
        assert_eq!(pair.server_db.transact(count).await.unwrap(), expected);
        assert_eq!(pair.client_db.transact(count).await.unwrap(), expected);
        assert_eq!(
            api(
                &pair.client.service,
                &pair.owner,
                "space.read",
                json!({"space":SPACE})
            )
            .await["names"],
            json!(["Later edit"])
        );
        pair.finish().await;
    }
}

#[tokio::test]
async fn incompatible_peer_retains_offline_edits_and_pairing_and_automatically_resumes_except_explicit_pause()
 {
    let mut pair = Pair::new().await;
    pair.client.exchange_automatic(&pair.owner).await.unwrap();
    for mode in [4, 5] {
        pair.fault.mode.store(mode, Ordering::SeqCst);
        let before = api(
            &pair.client.service,
            &pair.owner,
            "space.read",
            json!({"space":SPACE}),
        )
        .await;
        api(
            &pair.client.service,
            &pair.owner,
            "space.rename",
            json!({"space":SPACE,"name":format!("Offline {mode}"),"expected":before["version"]}),
        )
        .await;
        pair.run_error(ErrorCode::IncompatibleSync).await;
        pair.restart().await;
        pair.client.exchange_automatic(&pair.owner).await.unwrap();
        assert_ne!(
            api(
                &pair.service,
                &pair.admin,
                "space.read",
                json!({"space":SPACE})
            )
            .await["names"],
            json!([format!("Offline {mode}")])
        );
        pause(&pair.client.service, &pair.owner, &pair.pairing, true).await;
        pair.restart().await;
        pair.fault.mode.store(0, Ordering::SeqCst);
        pair.client.exchange_automatic(&pair.owner).await.unwrap();
        pair.run_error(ErrorCode::Conflict).await;
        let control = api(
            &pair.client.service,
            &pair.owner,
            "sync.inspect",
            json!({"space":SPACE,"pairing":pair.pairing}),
        )
        .await;
        assert_eq!(control["selected"], true);
        assert_eq!(control["paused"], true);
        assert_ne!(
            api(
                &pair.service,
                &pair.admin,
                "space.read",
                json!({"space":SPACE})
            )
            .await["names"],
            json!([format!("Offline {mode}")])
        );
        pause(&pair.client.service, &pair.owner, &pair.pairing, false).await;
        pair.client.exchange_automatic(&pair.owner).await.unwrap();
        assert_eq!(
            api(
                &pair.service,
                &pair.admin,
                "space.read",
                json!({"space":SPACE})
            )
            .await["names"],
            json!([format!("Offline {mode}")])
        );
        pair.client.exchange_automatic(&pair.owner).await.unwrap();
    }
    pair.finish().await;
}
