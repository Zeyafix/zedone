use super::*;
use serde_json::{Value, json};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

async fn start(session: &HttpSession, storage: &Storage, access: TestAccess) -> HttpSessionStatus {
    session
        .execute(
            HttpSessionRequest::Start {
                address: "127.0.0.1".into(),
                port: 0,
                access,
            },
            storage.clone(),
        )
        .await
        .unwrap()
}

async fn post(
    session: &HttpSessionStatus,
    path: &str,
    body: Value,
    auth: Option<&str>,
    origin: bool,
) -> (u16, Value) {
    let address = session
        .endpoint
        .as_ref()
        .unwrap()
        .strip_prefix("http://")
        .unwrap();
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    let body = body.to_string();
    let auth = auth
        .map(|value| format!("Authorization: Bearer {value}\r\n"))
        .unwrap_or_default();
    let origin = if origin {
        "Origin: http://untrusted.invalid\r\n"
    } else {
        ""
    };
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Type: application/json\r\n{auth}{origin}Content-Length: {}\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).await.unwrap();
    let mut response = String::new();
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        stream.read_to_string(&mut response),
    )
    .await
    .unwrap()
    .unwrap();
    let (headers, body) = response.split_once("\r\n\r\n").unwrap();
    let status = headers.split_whitespace().nth(1).unwrap().parse().unwrap();
    (status, serde_json::from_str(body).unwrap_or(Value::Null))
}

#[tokio::test]
async fn actual_http_limits_each_operation_and_shares_the_local_database() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::new(directory.path().to_path_buf());
    let a = storage
        .execute(StorageRequest::Seed { space: Space::A })
        .await
        .unwrap()
        .snapshot
        .unwrap();
    let b = storage
        .execute(StorageRequest::Seed { space: Space::B })
        .await
        .unwrap()
        .snapshot
        .unwrap();
    let manager = HttpSession::default();
    // Missing access explicitly defaults to read, also for non-UI callers.
    let request =
        serde_json::from_value(json!({"action":"start","address":"127.0.0.1","port":0})).unwrap();
    let session = manager.execute(request, storage.clone()).await.unwrap();
    let token = session.token.as_deref();
    assert!(matches!(session.access, Some(TestAccess::Read)));
    assert_eq!(session.token.as_ref().unwrap().len(), 64);
    for key in [None, Some("wrong")] {
        assert_eq!(
            post(
                &session,
                "/prototype/storage",
                json!({"action":"snapshot","space":"A"}),
                key,
                false
            )
            .await
            .0,
            401
        );
        assert_eq!(
            post(
                &session,
                "/prototype/echo",
                json!({"text":"Ёж"}),
                key,
                false
            )
            .await
            .0,
            401
        );
    }
    assert_eq!(
        post(
            &session,
            "/prototype/echo",
            json!({"text":"Ёж"}),
            token,
            true
        )
        .await
        .0,
        403
    );
    assert_eq!(
        post(
            &session,
            "/prototype/echo",
            json!({"text":"Ёж\n😀"}),
            token,
            false
        )
        .await,
        (200, json!({"text":"Ёж\n😀"}))
    );
    assert_eq!(
        post(
            &session,
            "/prototype/echo",
            json!({"text":42}),
            token,
            false
        )
        .await
        .0,
        422
    );
    for space in ["A", "B"] {
        for action in ["read", "snapshot"] {
            let body = if action == "read" {
                json!({"action":action,"space":space,"query":""})
            } else {
                json!({"action":action,"space":space})
            };
            assert_eq!(
                post(&session, "/prototype/storage", body, token, false)
                    .await
                    .0,
                if space == "A" { 200 } else { 403 }
            );
        }
    }
    for snapshot in [&a, &b] {
        for body in [
            json!({"action":"seed","space":snapshot.space}),
            json!({"action":"delete","space":snapshot.space}),
            json!({"action":"restore","snapshot":snapshot}),
            json!({"action":"move_card","expected":snapshot,"card":1,"column":1,"position":0}),
            json!({"action":"move_column","expected":snapshot,"column":0,"position":1}),
        ] {
            assert_eq!(
                post(&session, "/prototype/storage", body, token, false)
                    .await
                    .0,
                403
            );
        }
    }
    assert_eq!(
        post(
            &session,
            "/prototype/http-session",
            json!({"action":"stop"}),
            token,
            false
        )
        .await
        .0,
        404
    );
    manager
        .execute(HttpSessionRequest::Stop, storage.clone())
        .await
        .unwrap();
    // Stop has released the old listener before returning, including fixed ports.
    let port = session
        .endpoint
        .as_ref()
        .unwrap()
        .rsplit(':')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    let write = manager
        .execute(
            HttpSessionRequest::Start {
                address: "127.0.0.1".into(),
                port,
                access: TestAccess::Write,
            },
            storage.clone(),
        )
        .await
        .unwrap();
    let token = write.token.as_deref();
    assert_ne!(session.token, write.token);
    assert_eq!(
        post(
            &write,
            "/prototype/echo",
            json!({"text":""}),
            session.token.as_deref(),
            false
        )
        .await
        .0,
        401
    );
    assert_eq!(
        post(
            &write,
            "/prototype/storage",
            json!({"action":"delete","space":"B"}),
            token,
            false
        )
        .await
        .0,
        403
    );
    let moved = post(
        &write,
        "/prototype/storage",
        json!({"action":"move_card","expected":a,"card":1,"column":1,"position":0}),
        token,
        false,
    )
    .await;
    assert_eq!(moved.0, 200);
    assert_eq!(
        serde_json::to_value(
            storage
                .execute(StorageRequest::Snapshot { space: Space::A })
                .await
                .unwrap()
        )
        .unwrap(),
        moved.1
    );
    assert_eq!(
        post(
            &write,
            "/prototype/storage",
            json!({"action":"delete","space":"A"}),
            token,
            false
        )
        .await
        .0,
        200
    );
    assert!(
        storage
            .execute(StorageRequest::Snapshot { space: Space::A })
            .await
            .unwrap()
            .snapshot
            .is_none()
    );
    assert_eq!(
        post(
            &write,
            "/prototype/storage",
            json!({"action":"restore","snapshot":a}),
            token,
            false
        )
        .await,
        (200, json!({"snapshot":a}))
    );
    assert_eq!(
        storage
            .execute(StorageRequest::Snapshot { space: Space::B })
            .await
            .unwrap()
            .snapshot
            .unwrap(),
        b
    );
}

#[tokio::test]
async fn lifecycle_revokes_live_connections_and_never_restarts_access_implicitly() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::new(directory.path().to_path_buf());
    let manager = HttpSession::default();
    let first = start(&manager, &storage, TestAccess::Read).await;
    let address = first
        .endpoint
        .as_ref()
        .unwrap()
        .strip_prefix("http://")
        .unwrap();
    let mut connection = tokio::net::TcpStream::connect(address).await.unwrap();
    tokio::task::yield_now().await;
    manager.suspend(true);
    let request = format!(
        "POST /prototype/echo HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: 12\r\n\r\n{{\"text\":\"x\"}}",
        first.token.unwrap()
    );
    let _ = connection.write_all(request.as_bytes()).await;
    let mut response = String::new();
    let _ = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        connection.read_to_string(&mut response),
    )
    .await
    .unwrap();
    assert!(
        !response.contains("200 OK"),
        "pre-existing connections cannot retain a grant"
    );
    assert!(
        manager
            .execute(
                HttpSessionRequest::Start {
                    address: "127.0.0.1".into(),
                    port: 0,
                    access: TestAccess::Read
                },
                storage.clone()
            )
            .await
            .is_err()
    );
    manager.suspend(false);
    assert!(
        manager
            .execute(HttpSessionRequest::Status, storage.clone())
            .await
            .unwrap()
            .endpoint
            .is_none()
    );
    let second = start(&manager, &storage, TestAccess::Read).await;
    assert!(second.endpoint.is_some());
    manager
        .execute(HttpSessionRequest::Stop, storage.clone())
        .await
        .unwrap();
    manager
        .execute(HttpSessionRequest::Stop, storage.clone())
        .await
        .unwrap();
    assert!(
        manager
            .execute(HttpSessionRequest::Status, storage)
            .await
            .unwrap()
            .token
            .is_none()
    );
}

#[tokio::test]
async fn bind_failures_leave_access_off_and_a_failed_second_start_preserves_the_session() {
    let directory = tempfile::tempdir().unwrap();
    let storage = Storage::new(directory.path().to_path_buf());
    let manager = HttpSession::default();
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    for (address, port) in [
        ("0.0.0.0", 0),
        ("::", 0),
        ("192.168.1.1", 0),
        ("8.8.8.8", 0),
        ("invalid", 0),
        ("127.0.0.1", occupied.local_addr().unwrap().port()),
    ] {
        assert!(
            manager
                .execute(
                    HttpSessionRequest::Start {
                        address: address.into(),
                        port,
                        access: TestAccess::Write
                    },
                    storage.clone()
                )
                .await
                .is_err()
        );
        assert!(
            manager
                .execute(HttpSessionRequest::Status, storage.clone())
                .await
                .unwrap()
                .endpoint
                .is_none()
        );
    }
    let session = start(&manager, &storage, TestAccess::Read).await;
    assert!(
        manager
            .execute(
                HttpSessionRequest::Start {
                    address: "127.0.0.1".into(),
                    port: 0,
                    access: TestAccess::Write
                },
                storage.clone()
            )
            .await
            .is_err()
    );
    assert_eq!(
        manager
            .execute(HttpSessionRequest::Status, storage)
            .await
            .unwrap()
            .token,
        session.token
    );
}
