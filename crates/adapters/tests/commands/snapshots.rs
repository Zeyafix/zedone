use super::spaces::{cli, sid};
use super::*;
use zedone_adapters::{
    channel, network,
    secrets::{Secret, SecretError, SecretStore, Slot},
};

#[derive(Default)]
struct MemoryIdentity(std::sync::Mutex<Option<Secret>>);
impl SecretStore for MemoryIdentity {
    async fn load(&self, _: Slot) -> Result<Option<Secret>, SecretError> {
        Ok(self.0.lock().unwrap().clone())
    }
    async fn create(&self, _: Slot, secret: &[u8]) -> Result<(), SecretError> {
        let mut value = self.0.lock().unwrap();
        if value.is_some() {
            return Err(SecretError::AlreadyExists);
        }
        *value = Some(Secret::new(secret.to_vec()));
        Ok(())
    }
    async fn remove(&self, _: Slot) -> Result<(), SecretError> {
        *self.0.lock().unwrap() = None;
        Ok(())
    }
}

async fn call(service: &Service, secret: &str, id: u8, op: &str, p: Value) -> Value {
    post(service, op, &request(id, op, p), secret).await
}

#[tokio::test]
async fn transfer_keeps_one_archive_and_one_import_across_transports_and_restart() {
    let (dir, storage, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    let text = "0123456789abcdef".repeat(2500);
    let created = call(
        &service,
        &secret,
        1,
        "card.create",
        json!({
            "space":sid(1), "column":sid(2), "card":sid(3), "title":"Ёж 空間 😀", "description":text
        }),
    )
    .await;
    assert_eq!(created["status"], "ok");
    let export = request(2, "space.export", json!({"space":sid(1)}));
    let original = post(&service, "space.export", &export, &secret).await;
    assert_eq!(original["status"], "ok");
    assert_eq!(original["replayed"], false);
    let archive = original["result"]["archive"].as_str().unwrap();
    assert!(archive.len() > zedone_adapters::MAX_REQUEST_BYTES);
    let import = request(3, "space.import", json!({"archive":archive}));
    let copied = cli(&dir, &secret, "space.import", &import);
    assert_eq!(copied["status"], "ok");
    assert_eq!(copied["replayed"], false);
    let identity = channel::create_identity(&MemoryIdentity::default())
        .await
        .unwrap();
    let pin = identity.fingerprint();
    let listener = network::SecureListener::start(
        "127.0.0.1:0".parse().unwrap(),
        Arc::new(identity),
        http::router(service.clone()),
    )
    .await
    .unwrap();
    let remote = network::command(
        listener.address,
        pin,
        "/v1/commands/space.import",
        admin.credential().0,
        import.as_bytes(),
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::to_value(remote).unwrap(),
        json!({"status":"ok","replayed":true,"result":copied["result"]})
    );
    let remote = network::command(
        listener.address,
        pin,
        "/v1/commands/space.export",
        admin.credential().0,
        export.as_bytes(),
    )
    .await
    .unwrap();
    assert_eq!(
        serde_json::to_value(remote).unwrap(),
        json!({"status":"ok","replayed":true,"result":original["result"]})
    );
    listener.shutdown().await;
    let copy = copied["result"]["space"].as_str().unwrap();
    assert_ne!(copy, sid(1));
    let read = call(&service, &secret, 4, "space.read", json!({"space":copy})).await;
    let card = &read["result"]["contents"]["cards"][0];
    assert_ne!(card["card"], sid(3));
    assert_eq!(card["descriptions"], json!([text]));
    assert_eq!(card["titles"], json!(["Ёж 空間 😀"]));
    let renamed = call(&service, &secret, 5, "space.rename", json!({
        "space":sid(1), "name":"Changed", "expected":created_space_version(&service, &secret, 6, &sid(1)).await
    })).await;
    assert_eq!(renamed["status"], "ok");
    let repeated: Value =
        serde_json::from_str(&ipc::invoke(&service, "space.export", &export, &secret).await)
            .unwrap();
    assert_eq!(repeated["result"], original["result"]);
    assert_eq!(repeated["replayed"], true);
    let repeated = post(&service, "space.import", &import, &secret).await;
    assert_eq!(repeated["result"], copied["result"]);
    assert_eq!(repeated["replayed"], true);
    let second = call(
        &service,
        &secret,
        7,
        "space.import",
        json!({"archive":archive}),
    )
    .await;
    assert_ne!(second["result"]["space"], copy);
    let trashed = call(
        &service,
        &secret,
        8,
        "space.trash",
        json!({"space":copy,"expected":read["result"]["version"]}),
    )
    .await;
    assert_eq!(trashed["result"]["state"], "trashed");
    let restored = call(
        &service,
        &secret,
        9,
        "space.restore",
        json!({"space":copy,"expected":trashed["result"]["version"]}),
    )
    .await;
    assert_eq!(restored["result"]["state"], "active");
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage.clone());
    assert_eq!(
        cli(&dir, &secret, "space.export", &export),
        json!({"status":"ok","result":original["result"],"replayed":true})
    );
    let repeated: Value =
        serde_json::from_str(&ipc::invoke(&service, "space.import", &import, &secret).await)
            .unwrap();
    assert_eq!(
        repeated,
        json!({"status":"ok","result":copied["result"],"replayed":true})
    );
    let all = call(
        &service,
        &secret,
        10,
        "space.list",
        json!({"include_trashed":true}),
    )
    .await;
    assert_eq!(all["result"]["spaces"].as_array().unwrap().len(), 3);
    storage.shutdown().await.unwrap();
}

async fn created_space_version(service: &Service, secret: &str, id: u8, space: &str) -> Value {
    call(service, secret, id, "space.read", json!({"space":space})).await["result"]["version"]
        .clone()
}

#[tokio::test]
async fn transfer_rejects_invalid_bodies_and_reused_ids_without_creating_copies() {
    let (_dir, storage, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    let original = call(
        &service,
        &secret,
        1,
        "space.export",
        json!({"space":sid(1)}),
    )
    .await;
    for (op, params, code) in [
        ("space.export", json!({"space":sid(2)}), "access_denied"),
        (
            "space.import",
            json!({"archive":original["result"]["archive"]}),
            "command_mismatch",
        ),
    ] {
        assert_eq!(
            call(&service, &secret, 1, op, params).await["error"]["code"],
            code
        );
    }
    for archive in [
        "".to_owned(),
        "0".into(),
        "GG".into(),
        "é".into(),
        "00".into(),
        "00".repeat(1024 * 1024 + 1),
    ] {
        let denied = call(
            &service,
            &secret,
            2,
            "space.import",
            json!({"archive":archive}),
        )
        .await;
        assert_eq!(denied["error"]["code"], "invalid_request");
    }
    for body in [
        request(
            2,
            "space.import",
            json!({"archive":original["result"]["archive"],"subject":admin.subject()}),
        ),
        request(2, "space.import", json!({"archive":[1,2]})),
        request(
            0,
            "space.import",
            json!({"archive":original["result"]["archive"]}),
        ),
        format!(
            "{{\"id\":{:?},\"operation\":\"space.import\",\"parameters\":{{\"archive\":\"00\",\"archive\":\"00\"}}}}",
            [2; 16]
        ),
    ] {
        assert_eq!(
            post(&service, "space.import", &body, &secret).await["error"]["code"],
            "invalid_request"
        );
    }
    let all = call(
        &service,
        &secret,
        3,
        "space.list",
        json!({"include_trashed":true}),
    )
    .await;
    assert_eq!(all["result"]["spaces"].as_array().unwrap().len(), 1);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn read_access_can_export_and_import_but_replays_require_current_access() {
    let (dir, storage, service, admin) = super::cards::setup_cards().await;
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
    let secret = key(&member);
    let export = request(1, "space.export", json!({"space":sid(1)}));
    let original = post(&service, "space.export", &export, &secret).await;
    assert_eq!(original["status"], "ok");
    let import = request(
        2,
        "space.import",
        json!({"archive":original["result"]["archive"]}),
    );
    let copied = post(&service, "space.import", &import, &secret).await;
    assert_eq!(copied["status"], "ok");
    let copy = copied["result"]["space"].as_str().unwrap();
    assert_eq!(
        call(
            &service,
            &key(&admin),
            3,
            "space.read",
            json!({"space":copy})
        )
        .await["error"]["code"],
        "access_denied"
    );
    let denied = call(&service,&secret,3,"space.rename",json!({"space":sid(1),"name":"Denied","expected":created_space_version(&service,&secret,4,&sid(1)).await})).await;
    assert_eq!(denied["error"]["code"], "access_denied");
    let replacement = Arc::new(AccessKey::generate().unwrap());
    let (a, m, k) = (admin.clone(), member.clone(), replacement.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.replace_access(
                &a.credential(),
                clock,
                m.subject(),
                &k,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
                None,
            )
        })
        .await
        .unwrap();
    let secret: String = replacement
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    assert_eq!(
        post(&service, "space.import", &import, &secret).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        cli(&dir, &secret, "space.export", &export)["replayed"],
        true
    );
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_transact(move |ctx, clock| ctx.revoke_access(&a.credential(), clock, m.subject()))
        .await
        .unwrap();
    assert_eq!(
        post(&service, "space.export", &export, &secret).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        post(&service, "space.import", &import, &secret).await["error"]["code"],
        "access_denied"
    );
    storage.shutdown().await.unwrap();
}
