//! Development host. Windows lifecycle/tray/packaging belongs to P08.
#![forbid(unsafe_code)]
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if run().await.is_err() {
        eprintln!("ZeDone shell could not start");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 || args[0] != "--data-directory" {
        return Err("invalid arguments".into());
    }
    let shell = zedone_shell::open_shell(std::path::Path::new(&args[1])).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let address = listener.local_addr()?;
    println!("http://{address}");
    axum::serve(listener, zedone_shell::web::router(shell, address))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;
    Ok(())
}
