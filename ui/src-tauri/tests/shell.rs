#![forbid(unsafe_code)]
use axum::{
    body::{Body, to_bytes},
    http::Request,
};
use serde_json::{Value, json};
use tower::ServiceExt;
use zedone_shell::{Session, Shell};

#[tokio::test]
async fn new_application_host_finishes_saved_editing_series_before_exposing_service() {
    let dir = tempfile::tempdir().unwrap();
    let storage = zedone_storage::Storage::open(dir.path().join("zedone.sqlite"))
        .await
        .unwrap();
    let access = zedone_storage::access::PendingAccess::generate().unwrap();
    let key: String = access
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    storage
        .transact(move |ctx| ctx.bootstrap_access(&access))
        .await
        .unwrap();
    let service = zedone_adapters::Service::new(storage.clone());
    let space = "1".repeat(32);
    let column = "2".repeat(32);
    let card = "3".repeat(32);
    async fn call(
        service: &zedone_adapters::Service,
        key: &str,
        id: u8,
        operation: &str,
        parameters: Value,
    ) -> Value {
        let result = serde_json::to_value(
            service
                .dispatch(
                    operation,
                    json!({"id":vec![id;16],"operation":operation,"parameters":parameters})
                        .to_string()
                        .as_bytes(),
                    key,
                )
                .await,
        )
        .unwrap();
        assert_eq!(result["status"], "ok");
        result["result"].clone()
    }
    call(
        &service,
        &key,
        1,
        "space.create",
        json!({"space":space,"name":"Board"}),
    )
    .await;
    call(
        &service,
        &key,
        2,
        "column.create",
        json!({"space":space,"column":column,"name":"Column"}),
    )
    .await;
    let first = call(
        &service,
        &key,
        3,
        "card.create",
        json!({"space":space,"column":column,"card":card,"title":"Initial","description":""}),
    )
    .await;
    call(
        &service,
        &key,
        4,
        "card.begin_editing",
        json!({"space":space,"card":card,"expected":first["version"]}),
    )
    .await;
    let saved = call(&service,&key,5,"card.autosave",json!({"space":space,"card":card,"session":"04".repeat(16),"title":"Last saved title","description":"Last saved description","expected":first["version"]})).await;
    storage.shutdown().await.unwrap();
    drop(service);
    let shell = zedone_shell::open_shell(dir.path()).await;
    let service = shell.service.as_ref().unwrap();
    assert_eq!(
        call(
            service,
            &key,
            6,
            "card.read",
            json!({"space":space,"card":card})
        )
        .await,
        saved
    );
    let history = call(
        service,
        &key,
        7,
        "card.history",
        json!({"space":space,"card":card,"after":null,"limit":100,"expected":null}),
    )
    .await;
    assert_eq!(history["versions"].as_array().unwrap().len(), 2);
    assert_eq!(history["versions"][1]["title"], "Last saved title");
    assert_eq!(
        history["versions"][1]["description"],
        "Last saved description"
    );
}

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
