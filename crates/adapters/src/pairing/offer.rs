use super::*;

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

pub struct PendingSession {
    pub request: CommandRequest<RequestPairing>,
    offer: Offer,
    pending_key: Secret,
    access_key: Secret,
    pub address: Option<std::net::SocketAddr>,
}
impl PendingSession {
    pub async fn create(
        store: &impl SecretStore,
        offer: Offer,
        label: String,
    ) -> Result<Self, ErrorCode> {
        Self::create_at(store, offer, label, None).await
    }
    pub async fn create_at(
        store: &impl SecretStore,
        offer: Offer,
        label: String,
        address: Option<std::net::SocketAddr>,
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
            address,
        };
        let encoded = session.encode()?;
        store
            .create(Slot::PendingPairing, &encoded)
            .await
            .map_err(|_| ErrorCode::Unavailable)?;
        let saved = Self::load(store).await?.ok_or(ErrorCode::Unavailable)?;
        if *saved.encode()? != *encoded {
            return Err(ErrorCode::Unavailable);
        }
        Ok(saved)
    }
    pub(super) fn encode(&self) -> Result<Secret, ErrorCode> {
        let mut bytes = Secret::new(b"ZDPN\x01".to_vec());
        bytes.extend_from_slice(&self.offer.encode());
        bytes.extend_from_slice(&self.pending_key);
        bytes.extend_from_slice(&self.access_key);
        serde_json::to_writer(&mut *bytes, &self.request).map_err(|_| ErrorCode::Unavailable)?;
        if let Some(address) = self.address {
            let mut stored = Secret::new(b"ZDPN\x02".to_vec());
            let address = address.to_string();
            stored.push(address.len() as u8);
            stored.extend_from_slice(address.as_bytes());
            stored.extend_from_slice(&bytes);
            return Ok(stored);
        }
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
        Self::decode(&bytes).map(Some)
    }
    pub(super) fn decode(bytes: &[u8]) -> Result<Self, ErrorCode> {
        if bytes.starts_with(b"ZDPN\x02") {
            let length = usize::from(*bytes.get(5).ok_or(ErrorCode::Unavailable)?);
            let address =
                std::str::from_utf8(bytes.get(6..6 + length).ok_or(ErrorCode::Unavailable)?)
                    .ok()
                    .and_then(|v| v.parse::<std::net::SocketAddr>().ok())
                    .ok_or(ErrorCode::Unavailable)?;
            let rest = bytes.get(6 + length..).ok_or(ErrorCode::Unavailable)?;
            if !rest.starts_with(b"ZDPN\x01") || bytes.len() > crate::secrets::MAX_SECRET_BYTES {
                return Err(ErrorCode::Unavailable);
            }
            let mut session = Self::decode(rest)?;
            session.address = Some(address);
            return Ok(session);
        }
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
            address: None,
        };
        if session.request.parameters.invitation != session.offer.invitation
            || session.request.parameters.access_verifier.0
                != digest(b"zedone/access-key/v1\0", &session.access_key)
            || session.request.parameters.request != hex(&session.request.id)
        {
            return Err(ErrorCode::Unavailable);
        }
        Ok(session)
    }
    pub async fn submit<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: S,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let payload = Submit {
            command: &self.request,
            code: hex(&self.offer.code),
        };
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
    pub async fn unlink<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
        &self,
        stream: S,
        id: [u8; 16],
        expected: StateVersion,
    ) -> Result<CommandResponse<PairingSnapshot>, ErrorCode> {
        let request = CommandRequest {
            id,
            operation: UnlinkPairing::NAME.to_owned(),
            parameters: UnlinkPairing {
                request: self.request.parameters.request.clone(),
                expected,
            },
        };
        self.call(
            stream,
            "unlink",
            Secret::new(serde_json::to_vec(&request).map_err(|_| ErrorCode::InvalidRequest)?),
        )
        .await
    }
    pub fn credential(&self) -> zedone_core::access::Credential<'_> {
        zedone_core::access::Credential(&self.access_key)
    }
    pub fn fingerprint(&self) -> Fingerprint {
        self.offer.fingerprint
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
