//! Test-only synthetic installation. Never linked into the application.
#![forbid(unsafe_code)]
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    assert_eq!(
        args.len(),
        2,
        "Expected --prepare or --serve and an empty test directory"
    );
    assert!(args[0] == "--prepare" || args[0] == "--serve");
    let directory = std::path::Path::new(&args[1]);
    std::fs::create_dir_all(directory).unwrap();
    assert_eq!(
        std::fs::read_dir(directory).unwrap().count(),
        0,
        "Fixture refuses nonempty directory"
    );
    let storage = zedone_storage::Storage::open(directory.join("zedone.sqlite"))
        .await
        .unwrap();
    let access = Arc::new(zedone_storage::access::PendingAccess::generate().unwrap());
    let pending = access.clone();
    storage
        .timed_transact(move |ctx, _| ctx.bootstrap_access(&pending))
        .await
        .unwrap();
    let key: String = access
        .credential()
        .0
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    std::fs::write(
        directory.join("local-session.json"),
        serde_json::to_vec(&serde_json::json!({
            "subject": access.subject(), "key": key,
        }))
        .unwrap(),
    )
    .unwrap();
    if args[0] == "--prepare" {
        storage.shutdown().await.unwrap();
        return;
    }
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    println!("http://{address}");
    let shell = zedone_shell::Shell::new(
        Some(zedone_adapters::Service::new(storage)),
        directory.join("local-session.json"),
    );
    axum::serve(listener, zedone_shell::web::router(shell, address))
        .await
        .unwrap();
}
