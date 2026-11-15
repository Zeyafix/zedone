use super::*;
use crate::{Service, secrets::SecretError};
use serde_json::json;
use std::{collections::HashMap, sync::Mutex};
use tokio::net::{TcpListener, TcpStream};
use zedone_core::pairing::PairingStatus;
use zedone_storage::access::PendingAccess;

#[derive(Default)]
struct MemoryStore(Mutex<HashMap<&'static str, Secret>>);
impl SecretStore for MemoryStore {
    async fn load(&self, slot: Slot) -> Result<Option<Secret>, SecretError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .get(slot.name())
            .map(|v| Secret::new(v.to_vec())))
    }
    async fn create(&self, slot: Slot, secret: &[u8]) -> Result<(), SecretError> {
        let mut values = self.0.lock().unwrap();
        if values.contains_key(slot.name()) {
            return Err(SecretError::AlreadyExists);
        }
        values.insert(slot.name(), Secret::new(secret.to_vec()));
        Ok(())
    }
    async fn remove(&self, slot: Slot) -> Result<(), SecretError> {
        self.0.lock().unwrap().remove(slot.name());
        Ok(())
    }
}
async fn sockets() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (client, server) = tokio::join!(
        TcpStream::connect(listener.local_addr().unwrap()),
        listener.accept()
    );
    (client.unwrap(), server.unwrap().0)
}
fn success(
    response: CommandResponse<PairingSnapshot>,
) -> zedone_core::wire::CommandSuccess<PairingSnapshot> {
    match response {
        CommandResponse::Ok(v) => v,
        other => panic!("{other:?}"),
    }
}
async fn command(
    service: &Service,
    admin: &PendingAccess,
    id: u8,
    op: &str,
    p: serde_json::Value,
) -> serde_json::Value {
    serde_json::to_value(
        service
            .dispatch(
                op,
                &serde_json::to_vec(&json!({"id":vec![id;16],"operation":op,"parameters":p}))
                    .unwrap(),
                &hex(admin.credential().0),
            )
            .await,
    )
    .unwrap()
}
async fn setup() -> (tempfile::TempDir, Storage, PendingAccess, Identity) {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    db.transact(move |tx| tx.bootstrap_access(&a))
        .await
        .unwrap();
    let identity = channel::create_identity(&MemoryStore::default())
        .await
        .unwrap();
    (dir, db, Arc::try_unwrap(admin).ok().unwrap(), identity)
}
fn offer(identity: &Identity) -> Offer {
    Offer::new(format!("{:032x}", 1), identity.fingerprint(), &[0x11; 32]).unwrap()
}
async fn invitation(db: &Storage, admin: &PendingAccess, offer: &Offer) {
    let response = command(
        &Service::new(db.clone()),
        admin,
        1,
        "invitation.create",
        json!({"invitation":offer.invitation,"verifier":offer.verifier().0.to_vec()}),
    )
    .await;
    assert_eq!(response["status"], "ok", "{response}");
}
#[tokio::test]
async fn real_tls_request_restart_cancel_and_replay_keep_secrets_and_data_separate() {
    let (dir, db, admin, identity) = setup().await;
    let offered = offer(&identity);
    invitation(&db, &admin, &offered).await;
    let store = MemoryStore::default();
    let encoded = offered.encode();
    let session = PendingSession::create(
        &store,
        Offer::decode(&encoded).unwrap(),
        "Native device".into(),
    )
    .await
    .unwrap();
    let host = PairingHost::new(db.clone());
    let (client, server) = sockets().await;
    let (reply, served) = tokio::join!(session.submit(client), host.serve(&identity, server));
    served.unwrap();
    let accepted = success(reply.unwrap());
    assert_eq!(accepted.result.status, PairingStatus::Pending);
    let saved_id = session.request.parameters.request.clone();
    drop(session);
    drop(host);
    db.shutdown().await.unwrap();
    let bytes = std::fs::read(dir.path().join("pair.db")).unwrap();
    assert!(!bytes.windows(32).any(|b| b == [0x11; 32]));
    assert!(!bytes.windows(64).any(|b| b == "11".repeat(32).as_bytes()));
    let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
    let host = PairingHost::new(db.clone());
    let session = PendingSession::load(&store).await.unwrap().unwrap();
    let (client, server) = sockets().await;
    let (reply, served) = tokio::join!(session.submit(client), host.serve(&identity, server));
    served.unwrap();
    let replay = success(reply.unwrap());
    assert!(replay.replayed);
    assert_eq!(replay.result, accepted.result);
    let (client, server) = sockets().await;
    let (reply, served) = tokio::join!(
        session.inspect(client, [2; 16]),
        host.serve(&identity, server)
    );
    served.unwrap();
    assert_eq!(success(reply.unwrap()).result, accepted.result);
    let (client, server) = sockets().await;
    let (reply, served) = tokio::join!(
        session.cancel(client, [3; 16], accepted.result.version),
        host.serve(&identity, server)
    );
    served.unwrap();
    let cancelled = success(reply.unwrap());
    assert_eq!(cancelled.result.status, PairingStatus::Cancelled);
    let (client, server) = sockets().await;
    let (reply, served) = tokio::join!(
        session.cancel(client, [3; 16], accepted.result.version),
        host.serve(&identity, server)
    );
    served.unwrap();
    assert!(success(reply.unwrap()).replayed);
    let service = Service::new(db.clone());
    let list = command(&service, &admin, 4, "pairing.list", json!({"after":null})).await;
    assert_eq!(list["result"]["entries"][0]["request"], saved_id);
    assert_eq!(list["result"]["entries"][0]["status"], "cancelled");
    // Neither the invitation code nor the pending token can call data/admin APIs.
    let pending = store.load(Slot::PendingPairing).await.unwrap().unwrap();
    for key in [&pending[171..203], &pending[203..235], &[0x11; 32]] {
        let response = service
            .dispatch(
                "access.list",
                &serde_json::to_vec(
                    &json!({"id":vec![5;16],"operation":"access.list","parameters":{"after":null}}),
                )
                .unwrap(),
                &hex(key),
            )
            .await;
        assert!(
            matches!(response,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
        );
    }
    let list = command(&service, &admin, 6, "access.list", json!({"after":null})).await;
    assert_eq!(list["result"]["entries"].as_array().unwrap().len(), 1);
    store.remove(Slot::PendingPairing).await.unwrap();
    assert!(PendingSession::load(&store).await.unwrap().is_none());
    drop(service);
    drop(host);
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn wrong_pin_or_corrupt_protected_record_never_sends_or_replaces_a_request() {
    let (_dir, db, admin, identity) = setup().await;
    let mut offered = offer(&identity);
    invitation(&db, &admin, &offered).await;
    offered.fingerprint = Fingerprint([9; 32]);
    let store = MemoryStore::default();
    let session = PendingSession::create(&store, offered, "Device".into())
        .await
        .unwrap();
    let host = PairingHost::new(db.clone());
    let (client, server) = sockets().await;
    let (reply, served) = tokio::join!(session.submit(client), host.serve(&identity, server));
    assert_eq!(reply.unwrap_err(), ErrorCode::AccessDenied);
    assert!(served.is_err());
    let service = Service::new(db.clone());
    assert_eq!(
        command(&service, &admin, 2, "pairing.list", json!({"after":null})).await["result"]["entries"],
        json!([])
    );
    let saved = store.load(Slot::PendingPairing).await.unwrap().unwrap();
    assert!(
        PendingSession::create(&store, offer(&identity), "Other".into())
            .await
            .is_err()
    );
    assert_eq!(
        *store.load(Slot::PendingPairing).await.unwrap().unwrap(),
        *saved
    );
    store
        .0
        .lock()
        .unwrap()
        .insert(Slot::PendingPairing.name(), Secret::new(vec![1; 240]));
    assert!(PendingSession::load(&store).await.is_err());
    assert!(
        PendingSession::create(&store, offer(&identity), "Other".into())
            .await
            .is_err()
    );
    for invalid in [b"http://untrusted/".as_slice(), b"zd2:", b""] {
        assert!(Offer::decode(invalid).is_err());
    }
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn tls_authentication_failures_commit_the_attempt_limit_and_reject_browser_origins() {
    let (_dir, db, admin, identity) = setup().await;
    invitation(&db, &admin, &offer(&identity)).await;
    let store = MemoryStore::default();
    let wrong = Offer::new(format!("{:032x}", 1), identity.fingerprint(), &[0x22; 32]).unwrap();
    let session = PendingSession::create(&store, wrong, "Wrong code".into())
        .await
        .unwrap();
    let host = PairingHost::new(db.clone());
    for _ in 0..5 {
        let (client, server) = sockets().await;
        let (reply, served) = tokio::join!(session.submit(client), host.serve(&identity, server));
        served.unwrap();
        assert!(
            matches!(reply.unwrap(),CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
        );
    }
    let service = Service::new(db.clone());
    let current = command(
        &service,
        &admin,
        2,
        "invitation.inspect",
        json!({"invitation":format!("{:032x}",1)}),
    )
    .await;
    assert_eq!(current["result"]["status"], "locked");
    assert_eq!(
        command(&service, &admin, 3, "pairing.list", json!({"after":null})).await["result"]["entries"],
        json!([])
    );
    let mut headers = HeaderMap::new();
    headers.insert("origin", "https://example.invalid".parse().unwrap());
    assert!(
        matches!(host.dispatch("request",headers,Ok(Bytes::new())).await,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
    );
    db.shutdown().await.unwrap();
}
#[cfg(windows)]
#[tokio::test]
async fn pending_session_survives_windows_store_reopen_and_removal_keeps_identity() {
    let dir = tempfile::tempdir().unwrap();
    let mut namespace = [0; 32];
    getrandom::fill(&mut namespace).unwrap();
    let store = crate::secrets::windows::WindowsSecrets::new(namespace, dir.path());
    let identity = channel::create_identity(&store).await.unwrap();
    let session = PendingSession::create(&store, offer(&identity), "Windows".into())
        .await
        .unwrap();
    let reopened = crate::secrets::windows::WindowsSecrets::new(namespace, dir.path());
    assert_eq!(
        PendingSession::load(&reopened)
            .await
            .unwrap()
            .unwrap()
            .request
            .id,
        session.request.id
    );
    reopened.remove(Slot::PendingPairing).await.unwrap();
    assert_eq!(
        channel::load_identity(&reopened)
            .await
            .unwrap()
            .unwrap()
            .fingerprint(),
        identity.fingerprint()
    );
    reopened.remove(Slot::ChannelIdentity).await.unwrap();
}

#[tokio::test]
#[ignore = "requires paired host and Device-owned native Android process"]
async fn cross_platform_probe() {
    let role = std::env::var("ZEDONE_CHANNEL_PROBE_ROLE").unwrap();
    let metadata = std::env::var("ZEDONE_CHANNEL_PROBE_METADATA").unwrap();
    if role == "client" {
        let (port, pin): (u16, [u8; 32]) = serde_json::from_str(&metadata).unwrap();
        let address = ("127.0.0.1", port);
        let mut wrong = pin;
        wrong[0] ^= 1;
        assert!(
            channel::connect(
                TcpStream::connect(address).await.unwrap(),
                Fingerprint(wrong)
            )
            .await
            .is_err()
        );
        let store = MemoryStore::default();
        let session = PendingSession::create(
            &store,
            Offer::new(format!("{:032x}", 1), Fingerprint(pin), &[0x11; 32]).unwrap(),
            "Native Android".into(),
        )
        .await
        .unwrap();
        let accepted = success(
            session
                .submit(TcpStream::connect(address).await.unwrap())
                .await
                .unwrap(),
        );
        assert_eq!(accepted.result.status, PairingStatus::Pending);
        drop(session);
        let session = PendingSession::load(&store).await.unwrap().unwrap();
        let replay = success(
            session
                .submit(TcpStream::connect(address).await.unwrap())
                .await
                .unwrap(),
        );
        assert!(replay.replayed);
        assert_eq!(replay.result, accepted.result);
        let inspected = success(
            session
                .inspect(TcpStream::connect(address).await.unwrap(), [2; 16])
                .await
                .unwrap(),
        );
        assert_eq!(inspected.result, accepted.result);
        let cancelled = success(
            session
                .cancel(
                    TcpStream::connect(address).await.unwrap(),
                    [3; 16],
                    inspected.result.version,
                )
                .await
                .unwrap(),
        );
        assert_eq!(cancelled.result.status, PairingStatus::Cancelled);
        store.remove(Slot::PendingPairing).await.unwrap();
        return;
    }
    assert_eq!(role, "server");
    let work = async {
        let (dir, db, admin, identity) = setup().await;
        invitation(&db, &admin, &offer(&identity)).await;
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        std::fs::write(
            &metadata,
            serde_json::to_vec(&(
                listener.local_addr().unwrap().port(),
                identity.fingerprint().0,
            ))
            .unwrap(),
        )
        .unwrap();
        let host = PairingHost::new(db.clone());
        assert!(
            host.serve(&identity, listener.accept().await.unwrap().0)
                .await
                .is_err()
        );
        host.serve(&identity, listener.accept().await.unwrap().0)
            .await
            .unwrap();
        drop(host);
        db.shutdown().await.unwrap();
        let db = Storage::open(dir.path().join("pair.db")).await.unwrap();
        let host = PairingHost::new(db.clone());
        for _ in 0..3 {
            host.serve(&identity, listener.accept().await.unwrap().0)
                .await
                .unwrap();
        }
        let service = Service::new(db.clone());
        let list = command(&service, &admin, 4, "pairing.list", json!({"after":null})).await;
        assert_eq!(list["result"]["entries"][0]["status"], "cancelled");
        assert_eq!(command(&service,&admin,5,"access.list",json!({"after":null})).await["result"]["entries"].as_array().unwrap().len(),1);
        drop(host);
        drop(service);
        db.shutdown().await.unwrap();
    };
    tokio::time::timeout(Duration::from_secs(30), work)
        .await
        .unwrap();
}

#[tokio::test]
async fn tls_confirmation_enforces_scopes_own_unlink_and_fresh_reconnection() {
    let (_dir, db, admin, identity) = setup().await;
    let service = Service::new(db.clone());
    let space = format!("{:032x}", 90);
    let created = command(
        &service,
        &admin,
        20,
        "space.create",
        json!({"space":space,"name":"Preserved"}),
    )
    .await;
    assert_eq!(created["status"], "ok", "{created}");
    let store = MemoryStore::default();
    let mut old_key: Option<String> = None;
    for round in 0..2u8 {
        let offered = Offer::new(
            format!("{:032x}", round + 1),
            identity.fingerprint(),
            &[0x11 + round; 32],
        )
        .unwrap();
        let invite = command(
            &service,
            &admin,
            21 + round,
            "invitation.create",
            json!({"invitation":offered.invitation,"verifier":offered.verifier().0.to_vec()}),
        )
        .await;
        assert_eq!(invite["status"], "ok");
        let session = PendingSession::create(&store, offered, "Phone".into())
            .await
            .unwrap();
        let host = PairingHost::new(db.clone());
        let (client, server) = sockets().await;
        let (reply, served) = tokio::join!(session.submit(client), host.serve(&identity, server));
        served.unwrap();
        let pending = success(reply.unwrap()).result;
        let bytes=serde_json::to_vec(&json!({"id":vec![30+round;16],"operation":"pairing.confirm","parameters":{
            "request":pending.request,"expected":pending.version,"scope":{"kind":"spaces","spaces":{space.clone():"read"}},"expires_at_unix_ms":null
        }})).unwrap();
        let access_key = hex(session.credential().0);
        let denied = service
            .dispatch("pairing.confirm", &bytes, &access_key)
            .await;
        assert!(
            matches!(denied,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
        );
        let denied = service
            .dispatch(
                "pairing.confirm",
                &bytes,
                &hex(&store.load(Slot::PendingPairing).await.unwrap().unwrap()[171..203]),
            )
            .await;
        assert!(
            matches!(denied,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
        );
        // The TLS-only router cannot accept a confirmation, even with the
        // pending bearer and a well-formed administrative command.
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            format!(
                "Pending {}",
                hex(&store.load(Slot::PendingPairing).await.unwrap().unwrap()[171..203])
            )
            .parse()
            .unwrap(),
        );
        headers.insert("content-type", "application/json".parse().unwrap());
        assert!(
            matches!(host.dispatch("confirm",headers,Ok(Bytes::from(bytes.clone()))).await,CommandResponse::Error(e) if e.error.code==ErrorCode::InvalidRequest)
        );
        let approved = service
            .dispatch("pairing.confirm", &bytes, &hex(admin.credential().0))
            .await;
        assert!(matches!(approved, CommandResponse::Ok(_)), "{approved:?}");
        let (client, server) = sockets().await;
        let (reply, served) = tokio::join!(
            session.inspect(client, [31; 16]),
            host.serve(&identity, server)
        );
        served.unwrap();
        let linked = success(reply.unwrap()).result;
        assert_eq!(linked.status, PairingStatus::Confirmed);
        let read = serde_json::to_vec(
            &json!({"id":vec![32;16],"operation":"space.read","parameters":{"space":space}}),
        )
        .unwrap();
        assert!(matches!(
            service.dispatch("space.read", &read, &access_key).await,
            CommandResponse::Ok(_)
        ));
        if let Some(key) = &old_key {
            assert!(
                matches!(service.dispatch("space.read",&read,key).await,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
            );
        }
        let write=serde_json::to_vec(&json!({"id":vec![33;16],"operation":"space.rename","parameters":{"space":space,"name":"Unauthorized","expected":created["result"]["version"]}})).unwrap();
        assert!(
            matches!(service.dispatch("space.rename",&write,&access_key).await,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
        );
        let (client, server) = sockets().await;
        let (reply, served) = tokio::join!(
            session.unlink(client, [34; 16], linked.version),
            host.serve(&identity, server)
        );
        served.unwrap();
        let unlinked = success(reply.unwrap());
        assert_eq!(unlinked.result.status, PairingStatus::Unlinked);
        let (client, server) = sockets().await;
        let (reply, served) = tokio::join!(
            session.unlink(client, [34; 16], linked.version),
            host.serve(&identity, server)
        );
        served.unwrap();
        assert!(success(reply.unwrap()).replayed);
        assert!(
            matches!(service.dispatch("space.read",&read,&access_key).await,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
        );
        old_key = Some(access_key);
        store.remove(Slot::PendingPairing).await.unwrap();
    }
    let read = command(&service, &admin, 40, "space.read", json!({"space":space})).await;
    assert_eq!(read["result"]["names"], json!(["Preserved"]));
    db.shutdown().await.unwrap();
}
