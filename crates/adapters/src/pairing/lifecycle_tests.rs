use super::*;

#[derive(Clone, Copy)]
enum Failure {
    CreateBefore,
    CreateAfter,
    ReadBack,
    RemoveBefore,
    RemoveAfter,
    RemoveNoop,
}
#[derive(Clone, Default)]
struct FaultStore {
    memory: Arc<MemoryStore>,
    failure: Arc<Mutex<Option<Failure>>>,
}
impl FaultStore {
    fn take(&self, predicate: impl FnOnce(Failure) -> bool) -> bool {
        let mut fault = self.failure.lock().unwrap();
        if fault.is_some_and(predicate) {
            *fault = None;
            return true;
        }
        false
    }
    fn fail(&self, failure: Failure) {
        *self.failure.lock().unwrap() = Some(failure);
    }
    async fn bytes(&self, slot: Slot) -> Option<Secret> {
        self.memory.load(slot).await.unwrap()
    }
}
impl SecretStore for FaultStore {
    async fn load(&self, slot: Slot) -> Result<Option<Secret>, SecretError> {
        if matches!(slot, Slot::ConnectedPairing)
            && self.bytes(slot).await.is_some()
            && self.take(|f| matches!(f, Failure::ReadBack))
        {
            return Err(SecretError::Unavailable);
        }
        self.memory.load(slot).await
    }
    async fn create(&self, slot: Slot, secret: &[u8]) -> Result<(), SecretError> {
        if matches!(slot, Slot::ConnectedPairing)
            && self.take(|f| matches!(f, Failure::CreateBefore))
        {
            return Err(SecretError::Unavailable);
        }
        self.memory.create(slot, secret).await?;
        if matches!(slot, Slot::ConnectedPairing)
            && self.take(|f| matches!(f, Failure::CreateAfter))
        {
            return Err(SecretError::Unavailable);
        }
        Ok(())
    }
    async fn remove(&self, slot: Slot) -> Result<(), SecretError> {
        if self.take(|f| matches!(f, Failure::RemoveBefore)) {
            return Err(SecretError::Unavailable);
        }
        if self.take(|f| matches!(f, Failure::RemoveNoop)) {
            return Ok(());
        }
        self.memory.remove(slot).await?;
        if self.take(|f| matches!(f, Failure::RemoveAfter)) {
            return Err(SecretError::Unavailable);
        }
        Ok(())
    }
}

async fn refresh(
    lifecycle: &SessionLifecycle<impl SecretStore>,
    host: &PairingHost,
    identity: &Identity,
) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
    let (client, server) = sockets().await;
    let (response, served) = tokio::join!(lifecycle.refresh(client), host.serve(identity, server));
    if response.is_ok() {
        served.unwrap();
    }
    response
}

async fn pending<S: SecretStore>(
    store: S,
    db: &Storage,
    admin: &PendingAccess,
    identity: &Identity,
) -> (SessionLifecycle<S>, PairingSnapshot) {
    invitation(db, admin, &offer(identity)).await;
    let lifecycle = SessionLifecycle::new(store);
    lifecycle
        .create(offer(identity), "Phone".into())
        .await
        .unwrap();
    let (client, server) = sockets().await;
    let host = PairingHost::new(db.clone());
    let (response, served) = tokio::join!(lifecycle.submit(client), host.serve(identity, server));
    served.unwrap();
    (lifecycle, success(response.unwrap()).result)
}

async fn confirm(db: &Storage, admin: &PendingAccess, snapshot: &PairingSnapshot) {
    let result = command(
        &Service::new(db.clone()),
        admin,
        80,
        "pairing.confirm",
        json!({"request":snapshot.request,"expected":snapshot.version,
            "scope":{"kind":"administration"},
            "expires_at_unix_ms":null}),
    )
    .await;
    assert_eq!(result["status"], "ok", "{result}");
}

#[tokio::test]
async fn pending_confirmation_reopen_unlink_cleanup_and_new_creation() {
    let (_dir, db, admin, identity) = setup().await;
    let store = FaultStore::default();
    store
        .create(Slot::ChannelIdentity, &[91; 32])
        .await
        .unwrap();
    let (lifecycle, snapshot) = pending(store.clone(), &db, &admin, &identity).await;
    let host = PairingHost::new(db.clone());
    assert!(lifecycle.connected().await.unwrap().is_none());
    assert_eq!(
        success(refresh(&lifecycle, &host, &identity).await.unwrap())
            .result
            .status,
        PairingStatus::Pending
    );
    let bytes = store.bytes(Slot::PendingPairing).await.unwrap();
    assert!(store.bytes(Slot::ConnectedPairing).await.is_none());
    confirm(&db, &admin, &snapshot).await;
    let confirmed = success(refresh(&lifecycle, &host, &identity).await.unwrap()).result;
    assert_eq!(confirmed.status, PairingStatus::Confirmed);
    assert!(store.bytes(Slot::PendingPairing).await.is_none());
    assert_eq!(
        store
            .bytes(Slot::ConnectedPairing)
            .await
            .unwrap()
            .as_slice(),
        bytes.as_slice()
    );
    drop(lifecycle);
    let lifecycle = SessionLifecycle::new(store.clone());
    let session = lifecycle.connected().await.unwrap().unwrap();
    let original = PendingSession::decode(&bytes).unwrap();
    assert_eq!(session.credential().0, original.credential().0);
    assert!(
        lifecycle
            .create(offer(&identity), "Replacement".into())
            .await
            .is_err()
    );
    for replayed in [false, true] {
        let (client, server) = sockets().await;
        let (response, served) = tokio::join!(
            lifecycle.unlink(client, [81; 16], confirmed.version),
            host.serve(&identity, server)
        );
        served.unwrap();
        assert_eq!(success(response.unwrap()).replayed, replayed);
        assert!(lifecycle.connected().await.unwrap().is_some());
    }
    assert_eq!(
        success(refresh(&lifecycle, &host, &identity).await.unwrap())
            .result
            .status,
        PairingStatus::Unlinked
    );
    assert!(lifecycle.connected().await.unwrap().is_none());
    assert!(store.bytes(Slot::PendingPairing).await.is_none());
    assert!(store.bytes(Slot::ConnectedPairing).await.is_none());
    assert_eq!(
        store.bytes(Slot::ChannelIdentity).await.unwrap().as_slice(),
        &[91; 32]
    );
    let next = Offer::new(format!("{:032x}", 2), identity.fingerprint(), &[12; 32]).unwrap();
    lifecycle
        .create(next, "Fresh request".into())
        .await
        .unwrap();
    let next = PendingSession::load(&store).await.unwrap().unwrap();
    assert_ne!(next.request.id, original.request.id);
    assert_ne!(next.credential().0, original.credential().0);
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn every_promotion_boundary_recovers_without_replacing_a_key() {
    for fault in [
        Failure::CreateBefore,
        Failure::CreateAfter,
        Failure::ReadBack,
        Failure::RemoveBefore,
        Failure::RemoveAfter,
        Failure::RemoveNoop,
    ] {
        let (_dir, db, admin, identity) = setup().await;
        let store = FaultStore::default();
        let (lifecycle, snapshot) = pending(store.clone(), &db, &admin, &identity).await;
        let original = store.bytes(Slot::PendingPairing).await.unwrap();
        confirm(&db, &admin, &snapshot).await;
        store.fail(fault);
        let host = PairingHost::new(db.clone());
        assert_eq!(
            refresh(&lifecycle, &host, &identity).await.unwrap_err(),
            ErrorCode::Unavailable
        );
        let remaining = store
            .bytes(Slot::PendingPairing)
            .await
            .or(store.bytes(Slot::ConnectedPairing).await)
            .unwrap();
        assert_eq!(remaining.as_slice(), original.as_slice());
        drop(lifecycle);
        let reopened = SessionLifecycle::new(store.clone());
        assert_eq!(
            success(refresh(&reopened, &host, &identity).await.unwrap())
                .result
                .status,
            PairingStatus::Confirmed
        );
        assert_eq!(
            store
                .bytes(Slot::ConnectedPairing)
                .await
                .unwrap()
                .as_slice(),
            original.as_slice()
        );
        assert!(store.bytes(Slot::PendingPairing).await.is_none());
        db.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn fresh_terminal_status_cleans_pending_after_cancel_or_reject() {
    for rejected in [false, true] {
        let (_dir, db, admin, identity) = setup().await;
        let store = FaultStore::default();
        let (lifecycle, snapshot) = pending(store.clone(), &db, &admin, &identity).await;
        let host = PairingHost::new(db.clone());
        let terminal = if rejected {
            let result = command(
                &Service::new(db.clone()),
                &admin,
                82,
                "pairing.reject",
                json!({"request":snapshot.request,"expected":snapshot.version}),
            )
            .await;
            assert_eq!(result["status"], "ok", "{result}");
            PairingStatus::Rejected
        } else {
            for replayed in [false, true] {
                let (client, server) = sockets().await;
                let (response, served) = tokio::join!(
                    lifecycle.cancel(client, [82; 16], snapshot.version),
                    host.serve(&identity, server)
                );
                served.unwrap();
                assert_eq!(success(response.unwrap()).replayed, replayed);
            }
            PairingStatus::Cancelled
        };
        assert!(store.bytes(Slot::PendingPairing).await.is_some());
        store.fail(Failure::RemoveBefore);
        assert_eq!(
            refresh(&lifecycle, &host, &identity).await.unwrap_err(),
            ErrorCode::Unavailable
        );
        assert_eq!(
            success(refresh(&lifecycle, &host, &identity).await.unwrap())
                .result
                .status,
            terminal
        );
        assert!(store.bytes(Slot::PendingPairing).await.is_none());
        assert!(store.bytes(Slot::ConnectedPairing).await.is_none());
        db.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn conflicting_or_corrupt_slots_are_preserved_without_selection_or_replacement() {
    let (_dir, db, _admin, identity) = setup().await;
    let store = FaultStore::default();
    let original = PendingSession::create(&store, offer(&identity), "First".into())
        .await
        .unwrap();
    let bytes = store.bytes(Slot::PendingPairing).await.unwrap();
    let other = MemoryStore::default();
    let different = PendingSession::create(&other, offer(&identity), "Other".into())
        .await
        .unwrap();
    for invalid in [Secret::new(vec![1, 2, 3]), different.encode().unwrap()] {
        store
            .create(Slot::ConnectedPairing, &invalid)
            .await
            .unwrap();
        let lifecycle = SessionLifecycle::new(store.clone());
        assert!(lifecycle.connected().await.is_err());
        assert!(
            lifecycle
                .create(offer(&identity), "Replacement".into())
                .await
                .is_err()
        );
        assert_eq!(
            store.bytes(Slot::PendingPairing).await.unwrap().as_slice(),
            bytes.as_slice()
        );
        assert_eq!(
            store
                .bytes(Slot::ConnectedPairing)
                .await
                .unwrap()
                .as_slice(),
            invalid.as_slice()
        );
        store.remove(Slot::ConnectedPairing).await.unwrap();
    }
    assert_eq!(
        PendingSession::load(&store)
            .await
            .unwrap()
            .unwrap()
            .request
            .id,
        original.request.id
    );
    store.remove(Slot::PendingPairing).await.unwrap();
    store
        .create(Slot::ConnectedPairing, &[1, 2, 3])
        .await
        .unwrap();
    let lifecycle = SessionLifecycle::new(store.clone());
    assert!(lifecycle.connected().await.is_err());
    assert!(
        lifecycle
            .create(offer(&identity), "Replacement".into())
            .await
            .is_err()
    );
    assert_eq!(
        store
            .bytes(Slot::ConnectedPairing)
            .await
            .unwrap()
            .as_slice(),
        &[1, 2, 3]
    );
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn wrong_pin_or_missing_reply_preserves_both_protected_copies() {
    let (_dir, db, admin, identity) = setup().await;
    let store = FaultStore::default();
    let (lifecycle, snapshot) = pending(store.clone(), &db, &admin, &identity).await;
    confirm(&db, &admin, &snapshot).await;
    let host = PairingHost::new(db.clone());
    store.fail(Failure::RemoveBefore);
    assert_eq!(
        refresh(&lifecycle, &host, &identity).await.unwrap_err(),
        ErrorCode::Unavailable
    );
    let pending = store.bytes(Slot::PendingPairing).await.unwrap();
    let connected = store.bytes(Slot::ConnectedPairing).await.unwrap();
    let wrong = channel::create_identity(&MemoryStore::default())
        .await
        .unwrap();
    assert_eq!(
        refresh(&lifecycle, &host, &wrong).await.unwrap_err(),
        ErrorCode::AccessDenied
    );
    let (client, server) = sockets().await;
    drop(server);
    assert!(lifecycle.refresh(client).await.is_err());
    assert_eq!(
        store.bytes(Slot::PendingPairing).await.unwrap().as_slice(),
        pending.as_slice()
    );
    assert_eq!(
        store
            .bytes(Slot::ConnectedPairing)
            .await
            .unwrap()
            .as_slice(),
        connected.as_slice()
    );
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn revocation_after_interrupted_promotion_cleans_both_slots_without_reviving_access() {
    let (_dir, db, admin, identity) = setup().await;
    let store = FaultStore::default();
    let (lifecycle, snapshot) = pending(store.clone(), &db, &admin, &identity).await;
    confirm(&db, &admin, &snapshot).await;
    store.fail(Failure::RemoveBefore);
    let host = PairingHost::new(db.clone());
    assert_eq!(
        refresh(&lifecycle, &host, &identity).await.unwrap_err(),
        ErrorCode::Unavailable
    );
    let session = lifecycle.connected().await.unwrap().unwrap();
    let credential = hex(session.credential().0);
    let service = Service::new(db.clone());
    let body = serde_json::to_vec(
        &json!({"id":vec![83;16],"operation":"access.list","parameters":{"after":null}}),
    )
    .unwrap();
    assert!(matches!(
        service.dispatch("access.list", &body, &credential).await,
        CommandResponse::Ok(_)
    ));
    let grant = command(
        &service,
        &admin,
        84,
        "access.inspect",
        json!({"subject":snapshot.request}),
    )
    .await;
    assert_eq!(grant["status"], "ok", "{grant}");
    let revoked = command(
        &service,
        &admin,
        85,
        "access.revoke",
        json!({"subject":snapshot.request,"expected":grant["result"]["version"]}),
    )
    .await;
    assert_eq!(revoked["status"], "ok", "{revoked}");
    assert!(
        matches!(service.dispatch("access.list", &body, &credential).await,
        CommandResponse::Error(e) if e.error.code == ErrorCode::AccessDenied)
    );
    store.fail(Failure::RemoveBefore);
    assert_eq!(
        refresh(&lifecycle, &host, &identity).await.unwrap_err(),
        ErrorCode::Unavailable
    );
    let current = success(refresh(&lifecycle, &host, &identity).await.unwrap()).result;
    assert_eq!(current.status, PairingStatus::Unlinked);
    assert!(store.bytes(Slot::PendingPairing).await.is_none());
    assert!(store.bytes(Slot::ConnectedPairing).await.is_none());
    db.shutdown().await.unwrap();
}

async fn synthetic_reply(
    lifecycle: &SessionLifecycle<FaultStore>,
    identity: &Identity,
    snapshot: PairingSnapshot,
    replayed: bool,
) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
    let (client, server) = sockets().await;
    let serve = async {
        let tls = channel::accept(&identity.acceptor().unwrap(), server)
            .await
            .unwrap();
        let bytes = Bytes::from(
            serde_json::to_vec(&CommandResponse::Ok(zedone_core::wire::CommandSuccess {
                status: zedone_core::wire::SuccessStatus::Ok,
                result: snapshot,
                replayed,
            }))
            .unwrap(),
        );
        let service = hyper::service::service_fn(move |_| {
            let bytes = bytes.clone();
            async move { Ok::<_, std::convert::Infallible>(hyper::Response::new(Full::new(bytes))) }
        });
        hyper::server::conn::http1::Builder::new()
            .serve_connection(TokioIo::new(tls), service)
            .await
            .unwrap();
    };
    let (response, ()) = tokio::join!(lifecycle.refresh(client), serve);
    response
}

#[tokio::test]
async fn replayed_or_inconsistent_observations_never_promote_or_delete_a_session() {
    let (_dir, db, admin, identity) = setup().await;
    let store = FaultStore::default();
    let (lifecycle, snapshot) = pending(store.clone(), &db, &admin, &identity).await;
    let bytes = store.bytes(Slot::PendingPairing).await.unwrap();
    for status in [
        PairingStatus::Confirmed,
        PairingStatus::Cancelled,
        PairingStatus::Rejected,
        PairingStatus::Unlinked,
    ] {
        let mut reply = snapshot.clone();
        reply.status = status;
        reply.subject = matches!(
            reply.status,
            PairingStatus::Confirmed | PairingStatus::Unlinked
        )
        .then(|| reply.request.clone());
        assert_eq!(
            synthetic_reply(&lifecycle, &identity, reply, true)
                .await
                .unwrap_err(),
            ErrorCode::OutcomeUnknown
        );
    }
    let mut reply = snapshot;
    reply.status = PairingStatus::Confirmed;
    reply.subject = Some(format!("{:032x}", 999));
    assert_eq!(
        synthetic_reply(&lifecycle, &identity, reply, false)
            .await
            .unwrap_err(),
        ErrorCode::OutcomeUnknown
    );
    assert_eq!(
        store.bytes(Slot::PendingPairing).await.unwrap().as_slice(),
        bytes.as_slice()
    );
    assert!(store.bytes(Slot::ConnectedPairing).await.is_none());
    db.shutdown().await.unwrap();
}

#[cfg(windows)]
#[tokio::test]
async fn windows_credential_manager_reopens_connected_session_and_removes_only_its_slot() {
    use crate::secrets::windows::WindowsSecrets;
    let (dir, db, admin, identity) = setup().await;
    let mut namespace = [0; 32];
    getrandom::fill(&mut namespace).unwrap();
    let store = WindowsSecrets::new(namespace, dir.path());
    store
        .create(Slot::ChannelIdentity, &[45; 32])
        .await
        .unwrap();
    let (lifecycle, snapshot) = pending(store, &db, &admin, &identity).await;
    confirm(&db, &admin, &snapshot).await;
    let host = PairingHost::new(db.clone());
    let confirmed = success(refresh(&lifecycle, &host, &identity).await.unwrap()).result;
    let key = Secret::new(
        lifecycle
            .connected()
            .await
            .unwrap()
            .unwrap()
            .credential()
            .0
            .to_vec(),
    );
    drop(lifecycle);
    let reopened = SessionLifecycle::new(WindowsSecrets::new(namespace, dir.path()));
    assert_eq!(
        reopened.connected().await.unwrap().unwrap().credential().0,
        key.as_slice()
    );
    let (client, server) = sockets().await;
    let (response, served) = tokio::join!(
        reopened.unlink(client, [86; 16], confirmed.version),
        host.serve(&identity, server)
    );
    served.unwrap();
    assert_eq!(
        success(response.unwrap()).result.status,
        PairingStatus::Unlinked
    );
    refresh(&reopened, &host, &identity).await.unwrap();
    let store = WindowsSecrets::new(namespace, dir.path());
    assert!(store.load(Slot::PendingPairing).await.unwrap().is_none());
    assert!(store.load(Slot::ConnectedPairing).await.unwrap().is_none());
    assert_eq!(
        store
            .load(Slot::ChannelIdentity)
            .await
            .unwrap()
            .unwrap()
            .as_slice(),
        &[45; 32]
    );
    store.remove(Slot::ChannelIdentity).await.unwrap();
    db.shutdown().await.unwrap();
}
