//! Explicitly opted-in debug test fixture. Never linked into a production APK.
#[cfg(not(debug_assertions))]
compile_error!("pairing-acceptance must not be enabled in release builds");
use std::sync::Arc;
use tauri::{Manager, Runtime};
use zedone_adapters::{
    device_host::{DeviceHost, namespace},
    secrets::{Secret, SecretStore, Slot},
};
struct Fixture(#[allow(dead_code)] zedone_adapters::network::SecureListener);
pub fn start<R: Runtime>(app: tauri::AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        let Ok(base) = app.path().app_local_data_dir() else {
            return;
        };
        let Ok(run) = std::fs::read_to_string(base.join("pairing-acceptance-run")) else {
            return;
        };
        if run.len() != 32 || !run.bytes().all(|c| c.is_ascii_hexdigit()) {
            return;
        }
        let directory = base.join(format!("pairing-acceptance-{run}"));
        let work = async {
            std::fs::create_dir_all(&directory).map_err(|_| ())?;
            if std::fs::read_to_string(base.join("pairing-acceptance-cleanup"))
                .ok()
                .as_deref()
                == Some(run.as_str())
            {
                let store = crate::secrets::AndroidSecrets::new(
                    &app,
                    namespace(&directory).map_err(|_| ())?,
                )
                .map_err(|_| ())?;
                for slot in [
                    Slot::PendingPairing,
                    Slot::ConnectedPairing,
                    Slot::ChannelIdentity,
                ] {
                    store.remove(slot).await.map_err(|_| ())?;
                    if store.load(slot).await.map_err(|_| ())?.is_some() {
                        return Err(());
                    }
                }
                std::fs::remove_dir_all(&directory).map_err(|_| ())?;
                std::fs::write(base.join("pairing-acceptance-cleaned"), run.as_bytes())
                    .map_err(|_| ())?;
                return Ok(());
            }
            let existed = directory.join("data.sqlite").exists();
            let db = zedone_storage::Storage::open(directory.join("data.sqlite"))
                .await
                .map_err(|_| ())?;
            if !existed {
                let admin =
                    Arc::new(zedone_storage::access::PendingAccess::generate().map_err(|_| ())?);
                let a = admin.clone();
                db.transact(move |tx| tx.bootstrap_access(&a))
                    .await
                    .map_err(|_| ())?;
                let key = Secret::new(
                    admin
                        .credential()
                        .0
                        .iter()
                        .flat_map(|b| format!("{b:02x}").into_bytes())
                        .collect(),
                );
                std::fs::write(directory.join("admin.key"), &key).map_err(|_| ())?;
            }
            let key = std::fs::read_to_string(directory.join("admin.key")).map_err(|_| ())?;
            let store = Arc::new(
                crate::secrets::AndroidSecrets::new(&app, namespace(&directory).map_err(|_| ())?)
                    .map_err(|_| ())?,
            );
            let host = Arc::new(DeviceHost::new(
                zedone_adapters::Service::new(db),
                store.clone(),
            ));
            let (listener, pin) = host
                .start("127.0.0.1:0".parse().map_err(|_| ())?, &key)
                .await
                .map_err(|_| ())?;
            listener.shutdown().await;
            let identity = zedone_adapters::channel::load_identity(&store)
                .await
                .map_err(|_| ())?
                .ok_or(())?;
            // Debug fixture only: exercise the trusted native client method.
            // This proxy is deliberately absent from production host APIs.
            let routes = host.router().merge(
                axum::Router::new()
                    .route(
                        "/v1/commands/fixture.peer_command",
                        axum::routing::post(forward::<R>),
                    )
                    .with_state(host.clone()),
            );
            let listener = zedone_adapters::network::SecureListener::start(
                "127.0.0.1:0".parse().map_err(|_| ())?,
                Arc::new(identity),
                routes,
            )
            .await
            .map_err(|_| ())?;
            let value = serde_json::json!({"address":listener.address,"pin":pin.0});
            std::fs::write(
                directory.join("public.json"),
                serde_json::to_vec(&value).map_err(|_| ())?,
            )
            .map_err(|_| ())?;
            app.manage(Fixture(listener));
            Ok::<_, ()>(())
        }
        .await;
        if work.is_err() {
            let _ = std::fs::write(directory.join("failed"), b"fixture unavailable");
        }
    });
}

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Forward {
    operation: String,
    request: String,
}
async fn forward<R: Runtime>(
    axum::extract::State(host): axum::extract::State<
        Arc<DeviceHost<Arc<crate::secrets::AndroidSecrets<R>>>>,
    >,
    headers: axum::http::HeaderMap,
    axum::Json(input): axum::Json<Forward>,
) -> axum::Json<zedone_core::wire::CommandResponse<serde_json::Value>> {
    let result = async {
        use zedone_core::error::ErrorCode;
        if headers.contains_key("origin") || headers.get_all("authorization").iter().count() != 1 {
            return Err(ErrorCode::AccessDenied);
        }
        let key = headers
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .ok_or(ErrorCode::AccessDenied)?;
        host.authorize(key).await?;
        host.peer_command(&input.operation, input.request.as_bytes())
            .await
    }
    .await;
    axum::Json(result.unwrap_or_else(|code| Err(zedone_core::error::CommandError { code }).into()))
}
