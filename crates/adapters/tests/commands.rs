use std::{
    collections::BTreeMap,
    io::Write,
    process::{Command, Stdio},
    sync::Arc,
    time::{Duration, SystemTime},
};

use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use serde_json::{Value, json};
use tempfile::TempDir;
use tower::ServiceExt;
use zedone_adapters::{Service, http, ipc};
use zedone_core::{
    access::{GrantScope, SpacePermission},
    wire::CommandResponse,
};
use zedone_storage::{
    Storage,
    access::{AccessKey, PendingAccess},
};

#[path = "commands/spaces.rs"]
mod spaces;

#[path = "commands/columns.rs"]
mod columns;

#[path = "commands/cards.rs"]
mod cards;
#[path = "commands/ordering.rs"]
mod ordering;
#[path = "commands/tags.rs"]
mod tags;

fn key(access: &PendingAccess) -> String {
    access
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn request(id: u8, operation: &str, parameters: Value) -> String {
    serde_json::to_string(
        &json!({"id": vec![id;16], "operation": operation, "parameters": parameters}),
    )
    .unwrap()
}
fn inspect(id: u8, subject: &str) -> String {
    request(id, "access.inspect", json!({"subject":subject}))
}

async fn setup() -> (
    TempDir,
    Storage,
    Service,
    Arc<PendingAccess>,
    Arc<PendingAccess>,
) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.create_document("a")?;
            ctx.bootstrap_access(&a)?;
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Spaces(BTreeMap::from([("a".into(), SpacePermission::Read)])),
                None,
            )
        })
        .await
        .unwrap();
    (dir, storage.clone(), Service::new(storage), admin, member)
}

async fn post(service: &Service, operation: &str, body: &str, key: &str) -> Value {
    let response = http::router(service.clone())
        .oneshot(
            Request::post(format!("/v1/commands/{operation}"))
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {key}"))
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.headers()["cache-control"], "no-store");
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

#[tokio::test]
async fn combined_scope_has_the_same_strict_snapshot_and_replay_on_http_ipc_and_cli() {
    let (dir, storage, service, admin, member) = setup().await;
    let mixed_key = Arc::new(AccessKey::generate().unwrap());
    let (a, m, k) = (admin.clone(), member.clone(), mixed_key.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.replace_access(
                &a.credential(),
                clock,
                m.subject(),
                &k,
                &GrantScope::AdministrationAndSpaces(BTreeMap::from([(
                    "a".into(),
                    SpacePermission::ReadWrite,
                )])),
                None,
            )
        })
        .await
        .unwrap();
    let secret: String = mixed_key
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    let body = inspect(30, member.subject());
    let original = post(&service, "access.inspect", &body, &secret).await;
    assert_eq!(
        original["result"]["scope"],
        json!({"kind":"administration_and_spaces", "spaces":{"a":"read_write"}})
    );
    let replay: Value =
        serde_json::from_str(&ipc::invoke(&service, "access.inspect", &body, &secret).await)
            .unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], original["result"]);
    let key_path = dir.path().join("mixed.key");
    std::fs::write(&key_path, &secret).unwrap();
    let mut child = Command::new(
        std::env::var("ZEDONE_TEST_CLI").unwrap_or_else(|_| env!("CARGO_BIN_EXE_zedone").into()),
    )
    .arg("--database")
    .arg(dir.path().join("device.db"))
    .arg("--key-file")
    .arg(key_path)
    .args(["--operation", "access.inspect"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    assert!(output.stderr.is_empty());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        replay
    );
    let revoke = request(
        31,
        "access.revoke",
        json!({"subject":member.subject(), "expected":original["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "access.revoke", &revoke, &key(&admin)).await["status"],
        "ok"
    );
    assert_eq!(
        post(&service, "access.inspect", &body, &secret).await["error"]["code"],
        "access_denied",
        "revocation precedes the existing receipt"
    );
}

#[tokio::test]
async fn http_ipc_and_cli_share_replay_after_regrant_and_restart_without_repeating_revocation() {
    let (dir, storage, service, admin, member) = setup().await;
    let original = post(
        &service,
        "access.inspect",
        &inspect(1, member.subject()),
        &key(&admin),
    )
    .await;
    assert_eq!(original["status"], "ok");
    let revoke = request(
        2,
        "access.revoke",
        json!({"subject":member.subject(), "expected":original["result"]["version"]}),
    );
    let result = post(&service, "access.revoke", &revoke, &key(&admin)).await;
    assert_eq!(result["result"]["revoked"], true);
    let response: Value =
        serde_json::from_str(&ipc::invoke(&service, "access.revoke", &revoke, &key(&admin)).await)
            .unwrap();
    assert_eq!(response["replayed"], true);
    assert_eq!(result["result"], response["result"]);
    let (a, m) = (admin.clone(), member.clone());
    let replacement = Arc::new(AccessKey::generate().unwrap());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.replace_access(
                &a.credential(),
                clock,
                m.subject(),
                &replacement,
                &GrantScope::Spaces(BTreeMap::from([("a".into(), SpacePermission::ReadWrite)])),
                None,
            )
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage.clone());
    // Invoke the actual CLI child; the original request is preserved on stdin.
    let key_path = dir.path().join("admin.key");
    std::fs::write(&key_path, key(&admin)).unwrap();
    let mut child = Command::new(
        std::env::var("ZEDONE_TEST_CLI").unwrap_or_else(|_| env!("CARGO_BIN_EXE_zedone").into()),
    )
    .arg("--database")
    .arg(dir.path().join("device.db"))
    .arg("--key-file")
    .arg(&key_path)
    .args(["--operation", "access.revoke"])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(revoke.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty());
    let cli: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(cli, response);
    let current = post(
        &service,
        "access.inspect",
        &inspect(3, member.subject()),
        &key(&admin),
    )
    .await;
    assert_eq!(
        current["result"]["revoked"], false,
        "replay must not undo regrant"
    );
    assert_ne!(current["result"]["version"], original["result"]["version"]);
    let stale = request(
        4,
        "access.revoke",
        json!({"subject":member.subject(), "expected":original["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "access.revoke", &stale, &key(&admin)).await["error"]["code"],
        "stale_state"
    );
    let changed = request(
        2,
        "access.revoke",
        json!({"subject":member.subject(), "expected":current["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "access.revoke", &changed, &key(&admin)).await["error"]["code"],
        "command_mismatch"
    );
}

#[tokio::test]
async fn every_delivery_requires_current_administration_before_receipt_or_target_lookup() {
    let (_dir, storage, service, admin, member) = setup().await;
    for subject in [member.subject(), "00000000000000000000000000000000"] {
        assert_eq!(
            post(
                &service,
                "access.inspect",
                &inspect(1, subject),
                &key(&member)
            )
            .await["error"]["code"],
            "access_denied"
        );
    }
    let body = inspect(2, member.subject());
    assert_eq!(
        post(&service, "access.inspect", &body, &key(&admin)).await["status"],
        "ok"
    );
    let second = Arc::new(PendingAccess::generate().unwrap());
    let (a, s) = (admin.clone(), second.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &s,
                &GrantScope::Administration,
                None,
            )?;
            ctx.revoke_access(&s.credential(), clock, a.subject())
        })
        .await
        .unwrap();
    assert_eq!(
        post(&service, "access.inspect", &body, &key(&admin)).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        post(&service, "access.inspect", &body, &key(&second)).await["replayed"],
        false,
        "another subject has its own receipts"
    );
    // Expired administration is also checked against real host time.
    let expiring = Arc::new(PendingAccess::generate().unwrap());
    let (s, e) = (second.clone(), expiring.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.issue_access(
                &s.credential(),
                clock,
                &e,
                &GrantScope::Administration,
                Some(
                    std::time::UNIX_EPOCH
                        + Duration::from_millis(
                            (SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64)
                                + 60_000,
                        ),
                ),
            )
        })
        .await
        .unwrap();
    // Explicit early revocation also applies to limited administration.
    // No client clock or expired key can grant itself an extension.
    let (s, e) = (second.clone(), expiring.clone());
    storage
        .timed_transact(move |ctx, clock| ctx.revoke_access(&s.credential(), clock, e.subject()))
        .await
        .unwrap();
    assert_eq!(
        post(&service, "access.inspect", &body, &key(&expiring)).await["error"]["code"],
        "access_denied"
    );
}

#[tokio::test]
async fn strict_input_cannot_supply_identity_clock_policy_or_a_different_handler() {
    let (_dir, _storage, service, admin, member) = setup().await;
    let body = inspect(1, member.subject());
    let value: Value = serde_json::from_str(&body).unwrap();
    for extra in ["subject", "clock", "grant", "handler", "permission"] {
        let mut bad = value.clone();
        bad[extra] = json!("untrusted");
        assert_eq!(
            post(&service, "access.inspect", &bad.to_string(), &key(&admin)).await["error"]["code"],
            "invalid_request"
        );
    }
    for invalid in [
        body.replacen("\"parameters\":", "\"parameters\":{},\"parameters\":", 1),
        "{}{}".into(),
        "x".repeat(65537),
    ] {
        assert_eq!(
            post(&service, "access.inspect", &invalid, &key(&admin)).await["error"]["code"],
            "invalid_request"
        );
        let response: Value = serde_json::from_str(
            &ipc::invoke(&service, "access.inspect", &invalid, &key(&admin)).await,
        )
        .unwrap();
        assert_eq!(response["error"]["code"], "invalid_request");
    }
    assert_eq!(
        post(&service, "access.revoke", &body, &key(&admin)).await["error"]["code"],
        "invalid_request"
    );
    assert_eq!(
        post(&service, "access.inspect", &body, "invalid").await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        post(&service, "bootstrap", &body, &key(&admin)).await["error"]["code"],
        "invalid_request"
    );
    let last = post(
        &service,
        "access.inspect",
        &inspect(3, admin.subject()),
        &key(&admin),
    )
    .await;
    let revoke = request(
        4,
        "access.revoke",
        json!({"subject":admin.subject(),"expected":last["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "access.revoke", &revoke, &key(&admin)).await["error"]["code"],
        "invalid_request"
    );
    assert_eq!(
        post(
            &service,
            "access.inspect",
            &inspect(5, admin.subject()),
            &key(&admin)
        )
        .await["result"]["revoked"],
        false
    );
}

#[tokio::test]
async fn real_http_socket_delivers_the_same_envelope_and_blocks_browser_origins() {
    let (_dir, _storage, service, admin, member) = setup().await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server =
        tokio::spawn(async move { axum::serve(listener, http::router(service)).await.unwrap() });
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    for origin in ["", "Origin: https://untrusted.example\r\n"] {
        let mut socket = tokio::net::TcpStream::connect(address).await.unwrap();
        let body = inspect(9, member.subject());
        let frame = format!(
            "POST /v1/commands/access.inspect HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n{origin}Connection: close\r\n\r\n{body}",
            key(&admin),
            body.len()
        );
        socket.write_all(frame.as_bytes()).await.unwrap();
        let mut bytes = Vec::new();
        socket.read_to_end(&mut bytes).await.unwrap();
        let response = String::from_utf8(bytes).unwrap();
        let response: CommandResponse<Value> =
            serde_json::from_str(response.split_once("\r\n\r\n").unwrap().1).unwrap();
        assert_eq!(
            matches!(response, CommandResponse::Ok(_)),
            origin.is_empty()
        );
    }
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn only_the_host_configured_browser_origin_is_accepted_and_it_still_needs_credentials() {
    let (_dir, _storage, service, admin, member) = setup().await;
    for (origin, token, allowed) in [
        ("http://127.0.0.1:9000", key(&admin), true),
        ("http://127.0.0.1:9000", "wrong".into(), false),
        ("https://untrusted.example", key(&admin), false),
    ] {
        let response =
            http::router_with_origin(service.clone(), Some("http://127.0.0.1:9000".into()))
                .oneshot(
                    Request::post("/v1/commands/access.inspect")
                        .header("content-type", "application/json")
                        .header("authorization", format!("Bearer {token}"))
                        .header("origin", origin)
                        .body(Body::from(inspect(1, member.subject())))
                        .unwrap(),
                )
                .await
                .unwrap();
        let response: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
                .unwrap();
        assert_eq!(response["status"] == "ok", allowed);
    }
}

#[tokio::test]
async fn concurrent_delivery_and_lost_response_execute_one_effect_and_keep_one_receipt() {
    let (_dir, storage, service, admin, member) = setup().await;
    let original = post(
        &service,
        "access.inspect",
        &inspect(1, member.subject()),
        &key(&admin),
    )
    .await;
    let revoke = request(
        2,
        "access.revoke",
        json!({"subject":member.subject(),"expected":original["result"]["version"]}),
    );
    let (s1, s2) = (service.clone(), service.clone());
    let (b1, b2) = (revoke.clone(), revoke.clone());
    let (k1, k2) = (key(&admin), key(&admin));
    let first = tokio::spawn(async move { post(&s1, "access.revoke", &b1, &k1).await });
    let second = tokio::spawn(async move { ipc::invoke(&s2, "access.revoke", &b2, &k2).await });
    let a = first.await.unwrap();
    let b: Value = serde_json::from_str(&second.await.unwrap()).unwrap();
    assert_eq!(a["result"], b["result"]);
    assert_ne!(a["replayed"], b["replayed"]);
    let final_response = post(&service, "access.revoke", &revoke, &key(&admin)).await;
    assert_eq!(final_response["replayed"], true);
    // Receipt lookup is still denied after administrator revocation (covered
    // separately); no transport has a direct SQLite or bootstrap entry.
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancelled_wait_after_worker_admission_does_not_cancel_the_command() {
    let (_dir, storage, service, admin, member) = setup().await;
    let original = post(
        &service,
        "access.inspect",
        &inspect(1, member.subject()),
        &key(&admin),
    )
    .await;
    let revoke = request(
        2,
        "access.revoke",
        json!({"subject":member.subject(),"expected":original["result"]["version"]}),
    );
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let worker = storage.clone();
    let blocker = tokio::spawn(async move {
        worker
            .transact(move |_| {
                entered.send(()).unwrap();
                held.recv().unwrap();
                Ok(())
            })
            .await
    });
    waiting.await.unwrap();
    let secret = key(&admin);
    let mut delivery = Box::pin(service.dispatch("access.revoke", revoke.as_bytes(), &secret));
    std::future::poll_fn(|context| {
        assert!(delivery.as_mut().poll(context).is_pending());
        std::task::Poll::Ready(())
    })
    .await;
    drop(delivery);
    release.send(()).unwrap();
    blocker.await.unwrap().unwrap();
    let confirmed = post(&service, "access.revoke", &revoke, &key(&admin)).await;
    assert_eq!(confirmed["replayed"], true);
    assert_eq!(confirmed["result"]["revoked"], true);
}
