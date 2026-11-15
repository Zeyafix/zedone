//! Crash-safe host composition of the two protected slots. Neither a local slot
//! nor a historical confirmation receipt grants access to the ordinary Service.
use super::*;
use zedone_core::pairing::PairingStatus;

/// One instance per host namespace; the host must not run another lifecycle
/// owner concurrently. The mutex serializes the complete read/network/write
/// operation, rather than just individual OS-store calls.
pub struct SessionLifecycle<S> {
    store: S,
    operations: tokio::sync::Mutex<()>,
}

struct Saved {
    session: PendingSession,
    bytes: Secret,
    pending: bool,
    connected: bool,
}

impl<S: SecretStore> SessionLifecycle<S> {
    pub fn new(store: S) -> Self {
        Self {
            store,
            operations: tokio::sync::Mutex::new(()),
        }
    }

    /// Never replace either a pending or connected session, including an
    /// unreadable record or a partially completed promotion.
    pub async fn create(&self, offer: Offer, label: String) -> Result<(), ErrorCode> {
        self.create_at(offer, label, None).await
    }
    pub async fn create_at(
        &self,
        offer: Offer,
        label: String,
        address: Option<std::net::SocketAddr>,
    ) -> Result<(), ErrorCode> {
        let _guard = self.operations.lock().await;
        if self.saved().await?.is_some() {
            return Err(ErrorCode::Unavailable);
        }
        PendingSession::create_at(&self.store, offer, label, address).await?;
        Ok(())
    }

    pub async fn address(&self) -> Result<std::net::SocketAddr, ErrorCode> {
        let _guard = self.operations.lock().await;
        self.saved()
            .await?
            .and_then(|s| s.session.address)
            .ok_or(ErrorCode::Unavailable)
    }

    pub async fn state(&self) -> Result<serde_json::Value, ErrorCode> {
        let _guard = self.operations.lock().await;
        Ok(match self.saved().await? {
            None => serde_json::json!({"status":"unconfigured"}),
            Some(saved) => {
                serde_json::json!({"status":if saved.connected {"connected"} else {"pending"},
                "request":saved.session.request.parameters.request,"address":saved.session.address})
            }
        })
    }

    pub async fn submit<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: T,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let _guard = self.operations.lock().await;
        let saved = self.saved().await?.ok_or(ErrorCode::InvalidRequest)?;
        if saved.connected {
            return Err(ErrorCode::InvalidRequest);
        }
        let response = saved.session.submit(stream).await?;
        // Authenticated application rejection is durably terminal for this
        // exact attempt. Transport failure is the outer Err and preserves it.
        if matches!(&response,CommandResponse::Error(e) if e.error.code==ErrorCode::AccessDenied) {
            self.remove(Slot::PendingPairing, &saved.bytes).await?;
        }
        Ok(response)
    }

    /// A local readiness hint, never an authorization decision. Every ordinary
    /// command must still authenticate this credential against current access_*.
    pub async fn connected(&self) -> Result<Option<PendingSession>, ErrorCode> {
        let _guard = self.operations.lock().await;
        Ok(self
            .saved()
            .await?
            .filter(|saved| saved.connected)
            .map(|saved| saved.session))
    }

    /// The host retains id/expected as the original intention for retries.
    /// Cleanup is a separate fresh refresh, never based on a replayed receipt.
    pub async fn cancel<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: T,
        id: [u8; 16],
        expected: StateVersion,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let _guard = self.operations.lock().await;
        let saved = self.saved().await?.ok_or(ErrorCode::InvalidRequest)?;
        saved.session.cancel(stream, id, expected).await
    }

    pub async fn unlink<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: T,
        id: [u8; 16],
        expected: StateVersion,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let _guard = self.operations.lock().await;
        let saved = self.saved().await?.ok_or(ErrorCode::InvalidRequest)?;
        saved.session.unlink(stream, id, expected).await
    }

    /// Inspect with a fresh internal ID: accepting a caller-supplied or replayed
    /// observation could promote a historical confirmation after its revocation.
    /// A failure leaves every surviving record intact for the next refresh.
    pub async fn refresh<T: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: T,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let _guard = self.operations.lock().await;
        let saved = self.saved().await?.ok_or(ErrorCode::InvalidRequest)?;
        let mut id = [0; 16];
        getrandom::fill(&mut id).map_err(|_| ErrorCode::Unavailable)?;
        let response = saved.session.inspect(stream, id).await?;
        if let CommandResponse::Ok(success) = &response {
            if success.replayed {
                return Err(ErrorCode::OutcomeUnknown);
            }
            let snapshot = &success.result;
            let decided = matches!(
                snapshot.status,
                PairingStatus::Confirmed | PairingStatus::Unlinked
            );
            if (decided && snapshot.subject.as_deref() != Some(snapshot.request.as_str()))
                || (!decided && snapshot.subject.is_some())
            {
                return Err(ErrorCode::OutcomeUnknown);
            }
            match snapshot.status {
                PairingStatus::Pending if saved.connected => return Err(ErrorCode::OutcomeUnknown),
                PairingStatus::Pending => (),
                PairingStatus::Confirmed => {
                    if !saved.connected {
                        self.store
                            .create(Slot::ConnectedPairing, &saved.bytes)
                            .await
                            .map_err(|_| ErrorCode::Unavailable)?;
                    }
                    // Copy, acknowledge exact protected bytes, then remove the
                    // temporary copy. Any interrupted boundary is recoverable.
                    self.require(Slot::ConnectedPairing, &saved.bytes).await?;
                    if saved.pending {
                        self.remove(Slot::PendingPairing, &saved.bytes).await?;
                    }
                }
                PairingStatus::Cancelled | PairingStatus::Rejected if saved.connected => {
                    return Err(ErrorCode::OutcomeUnknown);
                }
                PairingStatus::Cancelled | PairingStatus::Rejected | PairingStatus::Unlinked => {
                    // Remove the temporary copy first. If interrupted, the
                    // connected copy still permits a fresh terminal inspection.
                    if saved.pending {
                        self.remove(Slot::PendingPairing, &saved.bytes).await?;
                    }
                    if saved.connected {
                        self.remove(Slot::ConnectedPairing, &saved.bytes).await?;
                    }
                }
            }
        }
        Ok(response)
    }

    async fn read(&self, slot: Slot) -> Result<Option<Secret>, ErrorCode> {
        self.store
            .load(slot)
            .await
            .map_err(|_| ErrorCode::Unavailable)
    }
    async fn require(&self, slot: Slot, expected: &[u8]) -> Result<(), ErrorCode> {
        if self.read(slot).await?.as_deref().map(|v| v.as_slice()) != Some(expected) {
            return Err(ErrorCode::Unavailable);
        }
        Ok(())
    }
    async fn remove(&self, slot: Slot, expected: &[u8]) -> Result<(), ErrorCode> {
        self.require(slot, expected).await?;
        self.store
            .remove(slot)
            .await
            .map_err(|_| ErrorCode::Unavailable)?;
        if self.read(slot).await?.is_some() {
            return Err(ErrorCode::Unavailable);
        }
        Ok(())
    }
    async fn saved(&self) -> Result<Option<Saved>, ErrorCode> {
        let pending = self.read(Slot::PendingPairing).await?;
        let connected = self.read(Slot::ConnectedPairing).await?;
        if let (Some(a), Some(b)) = (&pending, &connected)
            && a.as_slice() != b.as_slice()
        {
            // Two different requests are not an interrupted copy. Never choose
            // one, overwrite it or erase either automatically.
            return Err(ErrorCode::Unavailable);
        }
        let Some(bytes) = connected.as_ref().or(pending.as_ref()) else {
            return Ok(None);
        };
        Ok(Some(Saved {
            session: PendingSession::decode(bytes)?,
            bytes: Secret::new(bytes.to_vec()),
            pending: pending.is_some(),
            connected: connected.is_some(),
        }))
    }
}
