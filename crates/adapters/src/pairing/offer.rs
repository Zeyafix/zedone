use super::*;

/// Trusted out-of-band invitation, never Debug or part of a command receipt.
pub struct Offer {
    pub invitation: String,
    pub fingerprint: Fingerprint,
    code: Secret,
}
impl Offer {
    pub fn new(
        invitation: String,
        fingerprint: Fingerprint,
        code: &[u8],
    ) -> Result<Self, ErrorCode> {
        if !zedone_core::invitations::valid_id(&invitation) || code.len() != 32 || code == [0; 32] {
            return Err(ErrorCode::InvalidRequest);
        }
        Ok(Self {
            invitation,
            fingerprint,
            code: Secret::new(code.to_vec()),
        })
    }
    /// Same payload for manual entry and future QR rendering; address is separate.
    pub fn encode(&self) -> Secret {
        Secret::new(
            format!(
                "zd1:{}:{}:{}",
                self.invitation,
                hex(&self.fingerprint.0),
                hex(&self.code)
            )
            .into_bytes(),
        )
    }
    pub fn decode(bytes: &[u8]) -> Result<Self, ErrorCode> {
        if bytes.len() != 166 {
            return Err(ErrorCode::InvalidRequest);
        }
        let text = std::str::from_utf8(bytes).map_err(|_| ErrorCode::InvalidRequest)?;
        let fields: Vec<_> = text.split(':').collect();
        if fields.len() != 4 || fields[0] != "zd1" {
            return Err(ErrorCode::InvalidRequest);
        }
        Self::new(
            fields[1].to_owned(),
            Fingerprint(unhex(fields[2])?),
            &unhex(fields[3])?,
        )
    }
    pub fn verifier(&self) -> InvitationVerifier {
        InvitationVerifier(digest(b"zedone/invitation-code/v1\0", &self.code))
    }
}

/// One host namespace holds one pending connection, explicitly removed only
/// after acknowledged cancellation or later completion. No replacement on error.
pub struct PendingSession {
    pub request: CommandRequest<RequestPairing>,
    offer: Offer,
    pending_key: Secret,
    access_key: Secret,
}
impl PendingSession {
    pub async fn create(
        store: &impl SecretStore,
        offer: Offer,
        label: String,
    ) -> Result<Self, ErrorCode> {
        let mut id = [0; 16];
        let mut pending_key = Secret::new(vec![0; 32]);
        let mut access_key = Secret::new(vec![0; 32]);
        getrandom::fill(&mut id).map_err(|_| ErrorCode::Unavailable)?;
        getrandom::fill(&mut pending_key).map_err(|_| ErrorCode::Unavailable)?;
        getrandom::fill(&mut access_key).map_err(|_| ErrorCode::Unavailable)?;
        if label.trim().is_empty()
            || label.chars().count() > 80
            || label.chars().any(char::is_control)
        {
            return Err(ErrorCode::InvalidRequest);
        }
        let session = Self {
            request: CommandRequest {
                id,
                operation: RequestPairing::NAME.to_owned(),
                parameters: RequestPairing {
                    request: hex(&id),
                    invitation: offer.invitation.clone(),
                    label,
                    access_verifier: KeyVerifier(digest(b"zedone/access-key/v1\0", &access_key)),
                },
            },
            offer,
            pending_key,
            access_key,
        };
        let encoded = session.encode()?;
        store
            .create(Slot::PendingPairing, &encoded)
            .await
            .map_err(|_| ErrorCode::Unavailable)?;
        let saved = Self::load(store).await?.ok_or(ErrorCode::Unavailable)?;
        // Do not publish until protected persistence has acknowledged exact bytes.
        if *saved.encode()? != *encoded {
            return Err(ErrorCode::Unavailable);
        }
        Ok(saved)
    }
    fn encode(&self) -> Result<Secret, ErrorCode> {
        let mut bytes = Secret::new(b"ZDPN\x01".to_vec());
        bytes.extend_from_slice(&self.offer.encode());
        bytes.extend_from_slice(&self.pending_key);
        bytes.extend_from_slice(&self.access_key);
        serde_json::to_writer(&mut *bytes, &self.request).map_err(|_| ErrorCode::Unavailable)?;
        Ok(bytes)
    }
    pub async fn load(store: &impl SecretStore) -> Result<Option<Self>, ErrorCode> {
        let Some(bytes) = store
            .load(Slot::PendingPairing)
            .await
            .map_err(|_| ErrorCode::Unavailable)?
        else {
            return Ok(None);
        };
        if bytes.len() < 236
            || bytes.len() > crate::secrets::MAX_SECRET_BYTES
            || &bytes[..5] != b"ZDPN\x01"
        {
            return Err(ErrorCode::Unavailable);
        }
        let offer = Offer::decode(&bytes[5..171]).map_err(|_| ErrorCode::Unavailable)?;
        let request =
            decode_request::<RequestPairing>(&bytes[235..]).map_err(|_| ErrorCode::Unavailable)?;
        let session = Self {
            offer,
            request,
            pending_key: Secret::new(bytes[171..203].to_vec()),
            access_key: Secret::new(bytes[203..235].to_vec()),
        };
        if session.request.parameters.invitation != session.offer.invitation
            || session.request.parameters.access_verifier.0
                != digest(b"zedone/access-key/v1\0", &session.access_key)
            || session.request.parameters.request != hex(&session.request.id)
        {
            return Err(ErrorCode::Unavailable);
        }
        Ok(Some(session))
    }
    pub async fn submit<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: S,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let payload = Submit {
            command: &self.request,
            code: hex(&self.offer.code),
        };
        // The temporary JSON containing the invitation code never enters receipts.
        let body =
            Secret::new(serde_json::to_vec(&payload).map_err(|_| ErrorCode::InvalidRequest)?);
        self.call(stream, "request", body).await
    }
    pub async fn inspect<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: S,
        id: [u8; 16],
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let request = CommandRequest {
            id,
            operation: InspectPairing::NAME.to_owned(),
            parameters: InspectPairing {
                request: self.request.parameters.request.clone(),
            },
        };
        self.call(
            stream,
            "inspect",
            Secret::new(serde_json::to_vec(&request).map_err(|_| ErrorCode::InvalidRequest)?),
        )
        .await
    }
    pub async fn cancel<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: S,
        id: [u8; 16],
        expected: StateVersion,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let request = CommandRequest {
            id,
            operation: CancelPairing::NAME.to_owned(),
            parameters: CancelPairing {
                request: self.request.parameters.request.clone(),
                expected,
            },
        };
        self.call(
            stream,
            "cancel",
            Secret::new(serde_json::to_vec(&request).map_err(|_| ErrorCode::InvalidRequest)?),
        )
        .await
    }
    async fn call<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: S,
        operation: &str,
        bytes: Secret,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let response = exchange(
            stream,
            self.offer.fingerprint,
            operation,
            &self.pending_key,
            bytes,
        )
        .await?;
        if let CommandResponse::Ok(success) = &response {
            let p = &self.request.parameters;
            if success.result.request != p.request
                || success.result.invitation != p.invitation
                || success.result.label != p.label
            {
                return Err(ErrorCode::OutcomeUnknown);
            }
        }
        Ok(response)
    }
}
#[derive(serde::Serialize)]
#[serde(deny_unknown_fields)]
struct Submit<'a> {
    command: &'a CommandRequest<RequestPairing>,
    code: String,
}
