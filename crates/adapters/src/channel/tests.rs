use super::*;
use std::sync::Mutex;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[derive(Default)]
struct MemoryStore(Mutex<Option<Secret>>);
impl SecretStore for MemoryStore {
    async fn load(&self, _: Slot) -> Result<Option<Secret>, SecretError> {
        Ok(self
            .0
            .lock()
            .unwrap()
            .as_ref()
            .map(|s| Secret::new(s.to_vec())))
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

async fn sockets() -> (TcpStream, TcpStream) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let (client, server) = tokio::join!(
        TcpStream::connect(listener.local_addr().unwrap()),
        listener.accept()
    );
    (client.unwrap(), server.unwrap().0)
}

async fn exchange(identity: &Identity, pin: Fingerprint) -> Result<(), ChannelError> {
    let (client, server) = sockets().await;
    let acceptor = identity.acceptor()?;
    let server = async {
        let mut tls = accept(&acceptor, server).await?;
        assert_eq!(
            tls.get_ref().1.protocol_version(),
            Some(rustls::ProtocolVersion::TLSv1_3)
        );
        let mut message = [0; 7];
        tls.read_exact(&mut message)
            .await
            .map_err(|_| ChannelError::Unavailable)?;
        assert_eq!(&message, b"private");
        tls.write_all(b"reply")
            .await
            .map_err(|_| ChannelError::Unavailable)?;
        tls.flush().await.map_err(|_| ChannelError::Unavailable)?;
        Ok::<_, ChannelError>(())
    };
    let client = async {
        let mut tls = connect(client, pin).await?;
        assert_eq!(
            tls.get_ref().1.protocol_version(),
            Some(rustls::ProtocolVersion::TLSv1_3)
        );
        tls.write_all(b"private")
            .await
            .map_err(|_| ChannelError::Unavailable)?;
        tls.flush().await.map_err(|_| ChannelError::Unavailable)?;
        let mut reply = [0; 5];
        tls.read_exact(&mut reply)
            .await
            .map_err(|_| ChannelError::Unavailable)?;
        assert_eq!(&reply, b"reply");
        Ok::<_, ChannelError>(())
    };
    let (client, server) = tokio::join!(client, server);
    client?;
    server
}

#[tokio::test]
async fn identity_persists_and_real_tls_survives_reload_then_rejects_replacement() {
    let store = MemoryStore::default();
    assert!(load_identity(&store).await.unwrap().is_none());
    let first = create_identity(&store).await.unwrap();
    let pin = first.fingerprint();
    exchange(&first, pin).await.unwrap();
    assert!(matches!(
        create_identity(&store).await,
        Err(ChannelError::Storage(SecretError::AlreadyExists))
    ));
    drop(first);
    let restored = load_identity(&store).await.unwrap().unwrap();
    assert_eq!(restored.fingerprint(), pin);
    exchange(&restored, pin).await.unwrap();
    store.remove(Slot::ChannelIdentity).await.unwrap();
    store.remove(Slot::ChannelIdentity).await.unwrap();
    assert!(load_identity(&store).await.unwrap().is_none());
    let next = create_identity(&store).await.unwrap();
    assert_ne!(next.fingerprint(), pin);
    assert_eq!(exchange(&next, pin).await, Err(ChannelError::UntrustedPeer));
    exchange(&next, next.fingerprint()).await.unwrap();
}

#[tokio::test]
async fn malformed_or_mismatched_saved_identity_is_not_replaced() {
    for bytes in [
        vec![],
        b"ZDCH\x01\xff\xffbad".to_vec(),
        vec![42; MAX_SECRET_BYTES + 1],
    ] {
        let store = MemoryStore(Mutex::new(Some(Secret::new(bytes.clone()))));
        assert!(matches!(
            load_identity(&store).await,
            Err(ChannelError::InvalidIdentity)
        ));
        assert!(create_identity(&store).await.is_err());
        assert_eq!(store.0.lock().unwrap().as_ref().unwrap().as_slice(), bytes);
    }
    let first = Identity::generate().unwrap();
    let mut second = Identity::generate().unwrap();
    second.certificate = first.certificate.clone();
    assert!(matches!(
        Identity::decode(&second.encode().unwrap()),
        Err(ChannelError::InvalidIdentity)
    ));
}

#[tokio::test]
async fn pinned_certificate_still_requires_name_time_usage_and_signature() {
    let identity = Identity::generate().unwrap();
    let pin = PinnedServer(identity.fingerprint());
    let certificate = CertificateDer::from(identity.certificate.clone());
    let now = UnixTime::now();
    assert!(
        pin.verify_server_cert(
            &certificate,
            &[],
            &ServerName::try_from(SERVER_NAME).unwrap(),
            &[],
            now
        )
        .is_ok()
    );
    assert!(
        pin.verify_server_cert(
            &certificate,
            &[],
            &ServerName::try_from("attacker.invalid").unwrap(),
            &[],
            now
        )
        .is_err()
    );
    assert!(
        pin.verify_server_cert(
            &certificate,
            &[],
            &ServerName::try_from(SERVER_NAME).unwrap(),
            &[],
            UnixTime::since_unix_epoch(Duration::ZERO)
        )
        .is_err()
    );
    // rcgen's fixed default validity ends in year 4096.
    assert!(
        pin.verify_server_cert(
            &certificate,
            &[],
            &ServerName::try_from(SERVER_NAME).unwrap(),
            &[],
            UnixTime::since_unix_epoch(Duration::from_secs(100_000_000_000))
        )
        .is_err()
    );
    assert!(
        pin.verify_server_cert(
            &certificate,
            std::slice::from_ref(&certificate),
            &ServerName::try_from(SERVER_NAME).unwrap(),
            &[],
            now
        )
        .is_err()
    );
    let other = Identity::generate().unwrap();
    assert_eq!(
        exchange(&other, identity.fingerprint()).await,
        Err(ChannelError::UntrustedPeer)
    );
}

#[tokio::test]
async fn stalled_handshake_and_plaintext_fail_without_application_data() {
    let identity = Identity::generate().unwrap();
    let acceptor = identity.acceptor().unwrap();
    let (mut client, server) = sockets().await;
    client
        .write_all(b"POST /v1/commands/space.list HTTP/1.1\r\n\r\n")
        .await
        .unwrap();
    assert!(matches!(
        accept(&acceptor, server).await,
        Err(ChannelError::UntrustedPeer)
    ));
    let (client, _held_server) = sockets().await;
    assert!(matches!(
        connect(client, identity.fingerprint()).await,
        Err(ChannelError::TimedOut)
    ));
    let (_held_client, server) = sockets().await;
    assert!(matches!(
        accept(&acceptor, server).await,
        Err(ChannelError::TimedOut)
    ));
}

struct FailedStore;
impl SecretStore for FailedStore {
    async fn load(&self, _: Slot) -> Result<Option<Secret>, SecretError> {
        Err(SecretError::Unavailable)
    }
    async fn create(&self, _: Slot, _: &[u8]) -> Result<(), SecretError> {
        Err(SecretError::Unavailable)
    }
    async fn remove(&self, _: Slot) -> Result<(), SecretError> {
        Err(SecretError::Unavailable)
    }
}
#[tokio::test]
async fn unavailable_store_never_yields_a_usable_identity() {
    assert!(matches!(
        create_identity(&FailedStore).await,
        Err(ChannelError::Storage(SecretError::Unavailable))
    ));
    assert!(matches!(
        load_identity(&FailedStore).await,
        Err(ChannelError::Storage(SecretError::Unavailable))
    ));
}

#[derive(Debug)]
struct WrongKey(Arc<rustls::sign::CertifiedKey>);
impl rustls::server::ResolvesServerCert for WrongKey {
    fn resolve(
        &self,
        _: rustls::server::ClientHello<'_>,
    ) -> Option<Arc<rustls::sign::CertifiedKey>> {
        Some(self.0.clone())
    }
}

#[tokio::test]
async fn copying_the_pinned_certificate_without_its_private_key_is_rejected() {
    let original = Identity::generate().unwrap();
    let attacker = Identity::generate().unwrap();
    let provider = Arc::new(rustls::crypto::ring::default_provider());
    let key = provider
        .key_provider
        .load_private_key(PrivatePkcs8KeyDer::from(attacker.key.to_vec()).into())
        .unwrap();
    let forged = rustls::sign::CertifiedKey::new(
        vec![CertificateDer::from(original.certificate.clone())],
        key,
    );
    let config = ServerConfig::builder_with_provider(provider)
        .with_protocol_versions(&[&rustls::version::TLS13])
        .unwrap()
        .with_no_client_auth()
        .with_cert_resolver(Arc::new(WrongKey(Arc::new(forged))));
    let acceptor = TlsAcceptor::from(Arc::new(config));
    let (client, server) = sockets().await;
    let (client, server) = tokio::join!(
        connect(client, original.fingerprint()),
        accept(&acceptor, server)
    );
    assert!(matches!(client, Err(ChannelError::UntrustedPeer)));
    assert!(server.is_err());
}

// Invoked explicitly by product-check while its own Android sandbox is alive.
// Only public port/fingerprint travel through the environment/metadata file.
#[tokio::test]
#[ignore = "requires paired host and Device-owned native Android process"]
async fn cross_platform_probe() {
    let role = std::env::var("ZEDONE_CHANNEL_PROBE_ROLE").unwrap();
    let metadata = std::env::var("ZEDONE_CHANNEL_PROBE_METADATA").unwrap();
    if role == "client" {
        let data: (u16, [u8; 32]) = serde_json::from_str(&metadata).unwrap();
        let address = ("127.0.0.1", data.0);
        let mut wrong = data.1;
        wrong[0] ^= 1;
        assert!(matches!(
            connect(
                TcpStream::connect(address).await.unwrap(),
                Fingerprint(wrong)
            )
            .await,
            Err(ChannelError::UntrustedPeer)
        ));
        let mut tls = connect(
            TcpStream::connect(address).await.unwrap(),
            Fingerprint(data.1),
        )
        .await
        .unwrap();
        tls.write_all(b"private").await.unwrap();
        tls.flush().await.unwrap();
        let mut reply = [0; 5];
        tls.read_exact(&mut reply).await.unwrap();
        assert_eq!(&reply, b"reply");
        return;
    }
    assert_eq!(role, "server");
    #[cfg(windows)]
    let directory = tempfile::tempdir().unwrap();
    #[cfg(windows)]
    let store = crate::secrets::windows::WindowsSecrets::new(
        Sha256::digest(directory.path().as_os_str().as_encoded_bytes()).into(),
        directory.path(),
    );
    #[cfg(not(windows))]
    let store = MemoryStore::default();
    let identity = create_identity(&store).await.unwrap();
    drop(identity);
    let identity = load_identity(&store).await.unwrap().unwrap();
    let result = async move {
        let acceptor = identity.acceptor().unwrap();
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        std::fs::write(
            &metadata,
            serde_json::to_vec(&(
                listener.local_addr().unwrap().port(),
                identity.fingerprint().0,
            ))
            .unwrap(),
        )
        .unwrap();
        let first = listener.accept().await.unwrap().0;
        assert!(accept(&acceptor, first).await.is_err());
        let second = listener.accept().await.unwrap().0;
        let mut tls = accept(&acceptor, second).await.unwrap();
        let mut message = [0; 7];
        tls.read_exact(&mut message).await.unwrap();
        assert_eq!(&message, b"private");
        tls.write_all(b"reply").await.unwrap();
        tls.flush().await.unwrap();
    };
    let mut task = tokio::spawn(result);
    let outcome = tokio::time::timeout(Duration::from_secs(30), &mut task).await;
    if outcome.is_err() {
        task.abort();
        let _ = task.await;
    }
    store.remove(Slot::ChannelIdentity).await.unwrap();
    assert!(matches!(outcome, Ok(Ok(()))));
}

#[cfg(windows)]
#[tokio::test]
async fn windows_native_store_persistence_isolation_no_overwrite_and_removal() {
    use crate::secrets::windows::WindowsSecrets;
    let directory = tempfile::tempdir().unwrap();
    let mut namespace = [0; 32];
    // Random test namespace: never touch, enumerate or log existing user entries.
    namespace.copy_from_slice(&Sha256::digest(
        directory.path().as_os_str().as_encoded_bytes(),
    ));
    let store = WindowsSecrets::new(namespace, directory.path());
    let first = create_identity(&store).await.unwrap();
    let pin = first.fingerprint();
    let reloaded = WindowsSecrets::new(namespace, directory.path());
    let lock_directory = directory.path().to_owned();
    let task = tokio::spawn(async move {
        assert_eq!(
            load_identity(&reloaded)
                .await
                .unwrap()
                .unwrap()
                .fingerprint(),
            pin
        );
        assert!(matches!(
            create_identity(&reloaded).await,
            Err(ChannelError::Storage(SecretError::AlreadyExists))
        ));
        exchange(&load_identity(&reloaded).await.unwrap().unwrap(), pin)
            .await
            .unwrap();
        let mut other_namespace = namespace;
        other_namespace[0] ^= 1;
        let other = WindowsSecrets::new(other_namespace, &lock_directory);
        assert!(other.load(Slot::ChannelIdentity).await.unwrap().is_none());
        assert_eq!(
            reloaded
                .create(Slot::ChannelIdentity, &[0; MAX_SECRET_BYTES + 1])
                .await,
            Err(SecretError::InvalidSecret)
        );
    });
    let outcome = task.await;
    store.remove(Slot::ChannelIdentity).await.unwrap();
    store.remove(Slot::ChannelIdentity).await.unwrap();
    assert!(load_identity(&store).await.unwrap().is_none());
    assert!(outcome.is_ok());
}
