#![forbid(unsafe_code)]
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use serde_json::{Value, json};
use tower::ServiceExt;
use zedone_shell::{Session, Shell};

#[tokio::test]
async fn session_never_grants_access_or_swallows_corrupt_configuration() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("local-session.json");
    let shell = zedone_shell::open_shell(dir.path()).await;
    assert!(matches!(shell.session(), Session::Unconfigured));
    for bytes in [
        "{}".to_string(),
        "{".to_string(),
        "x".repeat(4097),
        json!({"subject":"a".repeat(32),"key":"b".repeat(64),"extra":true}).to_string(),
    ] {
        std::fs::write(&path, bytes).unwrap();
        assert!(matches!(shell.session(), Session::Unavailable));
    }
    std::fs::write(
        &path,
        json!({"subject":"a".repeat(32), "key":"b".repeat(64)}).to_string(),
    )
    .unwrap();
    assert!(matches!(shell.session(), Session::Configured { .. }));
    let response = shell
        .service
        .as_ref()
        .unwrap()
        .dispatch(
            "space.list",
            json!({
                "id":vec![1;16],"operation":"space.list","parameters":{"include_trashed":false}
            })
            .to_string()
            .as_bytes(),
            &"b".repeat(64),
        )
        .await;
    let value = serde_json::to_value(response).unwrap();
    assert_eq!(value["error"]["code"], "access_denied");
    std::fs::remove_file(path).unwrap();
    assert!(matches!(shell.session(), Session::Unconfigured));
    assert!(matches!(
        Shell::new(None, dir.path().join("missing")).session(),
        Session::Unavailable
    ));
}

#[tokio::test]
async fn embedded_host_rejects_foreign_pages_hosts_and_arbitrary_files() {
    let dir = tempfile::tempdir().unwrap();
    let app = zedone_shell::web::router(
        zedone_shell::open_shell(dir.path()).await,
        "127.0.0.1:12345".parse().unwrap(),
    );
    for (uri, host, origin, site) in [
        ("/host/session", "evil.test", None, None),
        (
            "/host/session",
            "127.0.0.1:12345",
            Some("https://evil.test"),
            None,
        ),
        ("/host/session", "127.0.0.1:12345", None, Some("cross-site")),
        ("/host/session", "127.0.0.1:12345", None, Some("same-site")),
    ] {
        let mut request = Request::get(uri).header("host", host);
        if let Some(origin) = origin {
            request = request.header("origin", origin);
        }
        if let Some(site) = site {
            request = request.header("sec-fetch-site", site);
        }
        assert_eq!(
            app.clone()
                .oneshot(request.body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            403
        );
    }
    for (uri, content_type) in [
        ("/", "text/html; charset=utf-8"),
        ("/app.js", "text/javascript; charset=utf-8"),
        ("/index.css", "text/css; charset=utf-8"),
        ("/host/session", "application/json"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::get(uri)
                    .header("host", "127.0.0.1:12345")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], content_type);
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(
            response.headers()["cross-origin-resource-policy"],
            "same-origin"
        );
        assert!(
            response.headers()["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("frame-ancestors 'none'")
        );
        if uri == "/host/session" {
            let body: Value =
                serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap())
                    .unwrap();
            assert_eq!(body, json!({"status":"unconfigured"}));
        }
    }
    let response = app
        .oneshot(
            Request::get("/../local-session.json")
                .header("host", "127.0.0.1:12345")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
    assert_eq!(body["error"]["code"], "invalid_request");
}
