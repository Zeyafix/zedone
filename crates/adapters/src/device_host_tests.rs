use super::*;
use crate::secrets::{SecretError, Slot};
use std::{collections::HashMap, sync::Mutex};
use zedone_storage::{Storage, access::PendingAccess};

#[derive(Default)]
struct Memory(Mutex<HashMap<&'static str, Secret>>);
impl SecretStore for Memory {
    async fn load(&self, slot: Slot) -> Result<Option<Secret>, SecretError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .get(slot.name())
            .map(|v| Secret::new(v.to_vec())))
    }
    async fn create(&self, slot: Slot, value: &[u8]) -> Result<(), SecretError> {
        let mut map = self.0.lock().unwrap();
        if map.contains_key(slot.name()) {
            return Err(SecretError::AlreadyExists);
        }
        map.insert(slot.name(), Secret::new(value.to_vec()));
        Ok(())
    }
    async fn remove(&self, slot: Slot) -> Result<(), SecretError> {
        self.0.lock().unwrap().remove(slot.name());
        Ok(())
    }
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
async fn setup(path: &FilePath) -> (Storage, String) {
    let storage = Storage::open(path).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    storage
        .transact(move |tx| tx.bootstrap_access(&a))
        .await
        .unwrap();
    (storage, hex(admin.credential().0))
}
#[tokio::test]
async fn secure_listener_authorization_shutdown_restart_and_saved_client() {
    let directory = tempfile::tempdir().unwrap();
    let (db, key) = setup(&directory.path().join("db")).await;
    let store = Arc::new(Memory::default());
    let host = Arc::new(DeviceHost::new(Service::new(db.clone()), store.clone()));
    assert!(
        host.start("127.0.0.1:0".parse().unwrap(), &hex(&[4; 32]))
            .await
            .is_err()
    );
    assert!(store.load(Slot::ChannelIdentity).await.unwrap().is_none());
    let (listener, pin) = host
        .start("127.0.0.1:0".parse().unwrap(), &key)
        .await
        .unwrap();
    let address = listener.address;
    let local = network::command(
        address,
        pin,
        "/v1/host/session.status",
        &crate::decode_key(&key).unwrap(),
        b"{}",
    )
    .await
    .unwrap();
    assert!(matches!(local,CommandResponse::Ok(v) if v.result["status"]=="unconfigured"));
    assert!(
        matches!(host.dispatch("session.command",b"{}",&key).await,CommandResponse::Error(e) if e.error.code==ErrorCode::InvalidRequest),
        "administrative host API exposed a remote-data proxy"
    );
    assert!(
        matches!(network::command(address,pin,"/v1/host/session.status",&[4;32],b"{}").await.unwrap(),CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
    );
    let mut wrong = pin;
    wrong.0[0] ^= 1;
    assert!(
        network::command(address, wrong, "/v1/host/session.status", &[4; 32], b"{}")
            .await
            .is_err()
    );
    listener.shutdown().await;
    assert!(network::connect(address).await.is_err());
    let (listener, reopened) = host.start(address, &key).await.unwrap();
    assert_eq!(pin, reopened);
    listener.shutdown().await;
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn host_client_uses_saved_address_and_key_with_ordinary_service_permissions() {
    let directory = tempfile::tempdir().unwrap();
    let (server_db, admin) = setup(&directory.path().join("server")).await;
    let server = Arc::new(DeviceHost::new(
        Service::new(server_db.clone()),
        Memory::default(),
    ));
    let (listener, pin) = server
        .start("127.0.0.1:0".parse().unwrap(), &admin)
        .await
        .unwrap();
    let (client_db, owner) = setup(&directory.path().join("client")).await;
    let store = Arc::new(Memory::default());
    let client = DeviceHost::new(Service::new(client_db.clone()), store.clone());
    let space = format!("{:032x}", 77);
    let service = Service::new(server_db.clone());
    async fn api(
        service: &Service,
        key: &str,
        id: u8,
        op: &str,
        p: serde_json::Value,
    ) -> serde_json::Value {
        let reply = service
            .dispatch(
                op,
                &serde_json::to_vec(
                    &serde_json::json!({"id":vec![id;16],"operation":op,"parameters":p}),
                )
                .unwrap(),
                key,
            )
            .await;
        let value = serde_json::to_value(reply).unwrap();
        assert_eq!(value["status"], "ok", "{value}");
        value["result"].clone()
    }
    api(
        &service,
        &admin,
        1,
        "space.create",
        serde_json::json!({"space":space,"name":"Preserved"}),
    )
    .await;
    let offer = Offer::new(format!("{:032x}", 2), pin, &[11; 32]).unwrap();
    api(
        &service,
        &admin,
        2,
        "invitation.create",
        serde_json::json!({"invitation":offer.invitation,"verifier":offer.verifier().0}),
    )
    .await;
    let prepared = serde_json::json!({"address":listener.address,"offer":std::str::from_utf8(&offer.encode()).unwrap(),"label":"Native device"});
    assert!(matches!(
        client
            .dispatch(
                "session.prepare",
                &serde_json::to_vec(&prepared).unwrap(),
                &owner
            )
            .await,
        CommandResponse::Ok(_)
    ));
    let pending = match client.dispatch("session.submit", b"{}", &owner).await {
        CommandResponse::Ok(v) => v.result,
        _ => panic!("submit failed"),
    };
    assert!(matches!(
        client.peer_command("space.read", b"{}").await,
        Err(ErrorCode::AccessDenied)
    ));
    api(&service,&admin,3,"pairing.confirm",serde_json::json!({"request":pending["request"],"expected":pending["version"],"scope":{"kind":"spaces","spaces":{space.clone():"read"}},"expires_at_unix_ms":null})).await;
    assert!(matches!(
        client.dispatch("session.refresh", b"{}", &owner).await,
        CommandResponse::Ok(_)
    ));
    drop(client);
    let client = DeviceHost::new(Service::new(client_db.clone()), store);
    let read =
        serde_json::json!({"id":vec![4;16],"operation":"space.read","parameters":{"space":space}});
    let reply = client
        .peer_command("space.read", &serde_json::to_vec(&read).unwrap())
        .await
        .unwrap();
    let saved = match reply {
        CommandResponse::Ok(v) => v.result,
        _ => panic!("read failed"),
    };
    assert_eq!(saved["names"], serde_json::json!(["Preserved"]));
    let rename = serde_json::json!({"id":vec![5;16],"operation":"space.rename","parameters":{"space":space,"name":"Forbidden","expected":saved["version"]}});
    assert!(
        matches!(client.peer_command("space.rename",&serde_json::to_vec(&rename).unwrap()).await.unwrap(),CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied)
    );
    listener.shutdown().await;
    client_db.shutdown().await.unwrap();
    server_db.shutdown().await.unwrap();
}

#[tokio::test]
#[ignore = "paired Windows host and Android APK acceptance fixture"]
async fn acceptance_server() {
    let directory = std::path::PathBuf::from(std::env::var_os("ZEDONE_PAIRING_FIXTURE").unwrap());
    assert!(directory.is_dir());
    let (db, key) = setup(&directory.join("host.sqlite")).await;
    std::fs::write(directory.join("admin.key"), &key).unwrap();
    #[cfg(windows)]
    let store = Arc::new(crate::secrets::windows::WindowsSecrets::new(
        namespace(&directory).unwrap(),
        &directory,
    ));
    #[cfg(not(windows))]
    let store = Arc::new(Memory::default());
    let host = Arc::new(DeviceHost::new(Service::new(db.clone()), store.clone()));
    let (listener, pin) = host
        .start("127.0.0.1:0".parse().unwrap(), &key)
        .await
        .unwrap();
    std::fs::write(
        directory.join("public.json"),
        serde_json::to_vec(&serde_json::json!({"address":listener.address,"pin":pin.0})).unwrap(),
    )
    .unwrap();
    let stopped = tokio::time::timeout(std::time::Duration::from_secs(240), async {
        while !directory.join("stop").exists() {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
        }
    })
    .await;
    listener.shutdown().await;
    for slot in [
        Slot::PendingPairing,
        Slot::ConnectedPairing,
        Slot::ChannelIdentity,
    ] {
        store.remove(slot).await.unwrap();
    }
    db.shutdown().await.unwrap();
    assert!(stopped.is_ok(), "fixture timeout");
}
