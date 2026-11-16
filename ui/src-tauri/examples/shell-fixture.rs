#![forbid(unsafe_code)]
use std::sync::Arc;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    #[cfg(windows)]
    if args.len() == 2 && args[0] == "--cleanup-devices" {
        use zedone_adapters::secrets::{SecretStore, Slot};
        let directory = std::path::Path::new(&args[1]);
        assert!(directory.join("host-namespace-v1").is_file());
        let namespace = zedone_adapters::device_host::namespace(directory).unwrap();
        let store = zedone_adapters::secrets::windows::WindowsSecrets::new(
            namespace,
            &directory.join("host-locks"),
        );
        for slot in [
            Slot::ChannelIdentity,
            Slot::PendingPairing,
            Slot::ConnectedPairing,
            Slot::OutgoingInvitation,
        ] {
            store.remove(slot).await.unwrap();
        }
        return;
    }
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
    #[allow(unused_mut)]
    let mut shell = zedone_shell::Shell::new(
        Some(zedone_adapters::Service::new(storage)),
        directory.join("local-session.json"),
    );
    #[cfg(not(windows))]
    {
        shell.devices = Some(Arc::new(zedone_adapters::device_control::Controller::new(
            Arc::new(zedone_adapters::device_host::DeviceHost::new(
                shell.service.clone().unwrap(),
                Memory::default(),
            )),
        )));
    }
    axum::serve(listener, zedone_shell::web::router(shell, address))
        .await
        .unwrap();
}

#[cfg(not(windows))]
#[derive(Default)]
struct Memory(
    std::sync::Mutex<std::collections::HashMap<&'static str, zedone_adapters::secrets::Secret>>,
);
#[cfg(not(windows))]
impl zedone_adapters::secrets::SecretStore for Memory {
    async fn load(
        &self,
        slot: zedone_adapters::secrets::Slot,
    ) -> Result<Option<zedone_adapters::secrets::Secret>, zedone_adapters::secrets::SecretError>
    {
        Ok(self
            .0
            .lock()
            .unwrap()
            .get(slot.name())
            .map(|bytes| zedone_adapters::secrets::Secret::new(bytes.to_vec())))
    }
    async fn create(
        &self,
        slot: zedone_adapters::secrets::Slot,
        bytes: &[u8],
    ) -> Result<(), zedone_adapters::secrets::SecretError> {
        let mut map = self.0.lock().unwrap();
        if map.contains_key(slot.name()) {
            return Err(zedone_adapters::secrets::SecretError::AlreadyExists);
        }
        map.insert(
            slot.name(),
            zedone_adapters::secrets::Secret::new(bytes.to_vec()),
        );
        Ok(())
    }
    async fn remove(
        &self,
        slot: zedone_adapters::secrets::Slot,
    ) -> Result<(), zedone_adapters::secrets::SecretError> {
        self.0.lock().unwrap().remove(slot.name());
        Ok(())
    }
}
