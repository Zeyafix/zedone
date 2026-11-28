#![forbid(unsafe_code)]
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use serde_json::{Value, json};
use std::{io::Write, path::PathBuf, sync::Arc};
use tower::ServiceExt;
use zedone_adapters::{Service, maintenance::Maintenance};
use zedone_core::{spaces::SpaceStore, wire::CommandResponse};
use zedone_storage::{
    Storage, access::PendingAccess, migration_backup::BackupLimits, snapshot::SnapshotLimits,
};

struct Fixture {
    root: tempfile::TempDir,
    path: PathBuf,
    old_key: String,
    key: String,
    controller: Arc<Maintenance>,
}
async fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("zedone.sqlite");
    let db = Storage::open(&path).await.unwrap();
    let owner = Arc::new(PendingAccess::generate().unwrap());
    let old_key = owner
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    db.transact(move |tx| {
        tx.bootstrap_access(&owner)?;
        tx.create_space(&"1".repeat(32), "Восстановлено", owner.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    let folder = root.path().join("zedone.sqlite.backups");
    std::fs::create_dir(&folder).unwrap();
    let backup = db
        .prepare_migration_backup(
            folder,
            123,
            BackupLimits {
                max_database_bytes: 1024 * 1024,
                max_spaces: 10,
                max_total_snapshot_bytes: 1024 * 1024,
                snapshot: SnapshotLimits {
                    max_bytes: 1024 * 1024,
                    max_changes: 1000,
                },
            },
        )
        .await
        .unwrap();
    drop(backup);
    db.shutdown().await.unwrap();
    std::fs::write(&path, b"damaged").unwrap();
    let controller = Maintenance::open(path.clone(), None, Some("InvalidDatabase".into()))
        .await
        .unwrap();
    let key = controller.local_session().unwrap().key;
    Fixture {
        root,
        path,
        old_key,
        key,
        controller,
    }
}
fn body(id: u8, parameters: Value) -> Vec<u8> {
    serde_json::to_vec(
        &json!({"id":vec![id;16],"operation":"maintenance.execute","parameters":parameters}),
    )
    .unwrap()
}
async fn call(f: &Fixture, id: u8, parameters: Value) -> Value {
    let response = f
        .controller
        .dispatch("maintenance.execute", &body(id, parameters), &f.key)
        .await;
    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value["status"], "ok", "{value}");
    value["result"].clone()
}
async fn http(f: &Fixture, key: &str, body: Vec<u8>, origin: Option<&str>) -> Value {
    let mut request = Request::post("/v1/commands/maintenance.execute")
        .header("authorization", format!("Bearer {key}"))
        .header("content-type", "application/json");
    if let Some(origin) = origin {
        request = request.header("origin", origin);
    }
    let response = zedone_adapters::http::maintenance_router(f.controller.clone(), None)
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    serde_json::from_slice(
        &to_bytes(response.into_body(), 4 * 1024 * 1024)
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn separate_authority_full_restore_http_ipc_and_real_cli_share_the_same_saved_command() {
    let f = fixture().await;
    for key in [&f.old_key, &"f".repeat(64)] {
        assert_eq!(
            http(&f, key, body(1, json!({"action":"inspect"})), None).await["error"]["code"],
            "access_denied"
        );
    }
    assert_eq!(
        http(
            &f,
            &f.key,
            body(1, json!({"action":"inspect"})),
            Some("https://evil.test")
        )
        .await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        http(
            &f,
            &f.key,
            body(1, json!({"action":"inspect","extra":true})),
            None
        )
        .await["error"]["code"],
        "invalid_request"
    );
    let state = call(&f, 1, json!({"action":"inspect"})).await;
    let prepare = body(
        2,
        json!({"action":"prepare","copy":state["copies"][0]["id"],"expected":state["version"]}),
    );
    let result = http(&f, &f.key, prepare.clone(), None).await;
    assert_eq!(result["status"], "ok");
    let replay = f
        .controller
        .dispatch("maintenance.execute", &prepare, &f.key)
        .await;
    assert!(matches!(replay,CommandResponse::Ok(v) if v.replayed));
    let listener = f.controller.start_loopback().await.unwrap();
    let endpoint = listener.endpoint.clone();
    let key_path = f.root.path().join("recovery-key");
    std::fs::write(&key_path, &f.key).unwrap();
    let apply = body(
        3,
        json!({"action":"apply","plan":result["result"]["plan"]["id"],"expected":result["result"]["version"]}),
    );
    let command = std::env::var_os("ZEDONE_TEST_CLI")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_BIN_EXE_zedone")));
    let value = tokio::task::spawn_blocking(move || {
        let mut child = std::process::Command::new(command)
            .args([
                "--maintenance-endpoint",
                &endpoint,
                "--key-file",
                key_path.to_str().unwrap(),
                "--stdin",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        child.stdin.take().unwrap().write_all(&apply).unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        serde_json::from_slice::<Value>(&output.stdout).unwrap()
    })
    .await
    .unwrap();
    assert_eq!(value["result"]["plan"]["phase"], "restored");
    let restored = call(&f, 4, json!({"action":"inspect"})).await;
    let ready = call(
        &f,
        5,
        json!({"action":"retry","expected":restored["version"]}),
    )
    .await;
    assert_eq!(ready["normal_ready"], true);
    let service = f.controller.service().unwrap();
    let old=service.dispatch("space.list",&serde_json::to_vec(&json!({"id":vec![6;16],"operation":"space.list","parameters":{"include_trashed":false}})).unwrap(),&f.old_key).await;
    assert!(
        matches!(old,CommandResponse::Error(v) if v.error.code==zedone_core::error::ErrorCode::AccessDenied)
    );
    let session: zedone_storage::emergency::LocalSession =
        serde_json::from_slice(&std::fs::read(f.root.path().join("local-session.json")).unwrap())
            .unwrap();
    let response=service.dispatch("space.list",&serde_json::to_vec(&json!({"id":vec![7;16],"operation":"space.list","parameters":{"include_trashed":false}})).unwrap(),&session.key).await;
    let response = serde_json::to_value(response).unwrap();
    assert_eq!(response["status"], "ok");
    assert!(response.to_string().contains("Восстановлено"));
    service.storage_shutdown().await.unwrap();
    listener.shutdown().await;
}

#[tokio::test]
async fn read_permission_cannot_mutate_and_revocation_applies_before_receipt_replay() {
    let f = fixture().await;
    let initial = call(&f, 1, json!({"action":"inspect"})).await;
    let key = "a1".repeat(32);
    let verifier = zedone_storage::emergency::key_verifier(&key).unwrap();
    let issued=call(&f,2,json!({"action":"issue_access","label":"Read-only agent","verifier":verifier,"writable":false,"expires_ms":null,"expected":initial["version"]})).await;
    let read = http(&f, &key, body(3, json!({"action":"inspect"})), None).await;
    assert_eq!(read["result"]["writable"], false);
    assert_eq!(read["result"]["owner"], false);
    let denied=http(&f,&key,body(4,json!({"action":"prepare","copy":initial["copies"][0]["id"],"expected":issued["version"]})),None).await;
    assert_eq!(denied["error"]["code"], "access_denied");
    call(&f,5,json!({"action":"revoke_access","subject":issued["issued_subject"],"expected":issued["version"]})).await;
    assert_eq!(
        http(&f, &key, body(3, json!({"action":"inspect"})), None).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(std::fs::read(&f.path).unwrap(), b"damaged");
}

#[tokio::test]
async fn manual_enter_shuts_the_shared_worker_and_explicit_retry_reopens_without_a_restore() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("db");
    let db = Storage::open(&path).await.unwrap();
    let old = db.clone();
    let controller = Maintenance::open(path, Some(Service::new(db)), None)
        .await
        .unwrap();
    let key = controller.local_session().unwrap().key;
    let initial = serde_json::to_value(
        controller
            .dispatch(
                "maintenance.execute",
                &body(1, json!({"action":"inspect"})),
                &key,
            )
            .await,
    )
    .unwrap();
    let entered = serde_json::to_value(
        controller
            .dispatch(
                "maintenance.execute",
                &body(
                    2,
                    json!({"action":"enter","expected":initial["result"]["version"]}),
                ),
                &key,
            )
            .await,
    )
    .unwrap();
    assert_eq!(entered["result"]["normal_ready"], false);
    assert!(old.read(|_| Ok(())).await.is_err());
    let ready = serde_json::to_value(
        controller
            .dispatch(
                "maintenance.execute",
                &body(
                    3,
                    json!({"action":"retry","expected":entered["result"]["version"]}),
                ),
                &key,
            )
            .await,
    )
    .unwrap();
    assert_eq!(ready["result"]["normal_ready"], true);
    controller
        .service()
        .unwrap()
        .storage_shutdown()
        .await
        .unwrap();
}
