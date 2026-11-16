use crate::secrets::{MAX_SECRET_BYTES, Secret, SecretError, SecretStore, Slot};
use rustls::{
    ClientConfig, DigitallySignedStruct, RootCertStore, ServerConfig, SignatureScheme,
    client::{
        WebPkiServerVerifier,
        danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier},
    },
    pki_types::{CertificateDer, PrivatePkcs8KeyDer, ServerName, UnixTime},
};
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Duration};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::{TlsAcceptor, TlsConnector};

const SERVER_NAME: &str = "zedone.invalid";
const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ChannelError {
    Unavailable,
    UntrustedPeer,
    TimedOut,
    InvalidIdentity,
    Storage(SecretError),
}
impl From<SecretError> for ChannelError {
    fn from(error: SecretError) -> Self {
        Self::Storage(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Fingerprint(pub [u8; 32]);
impl Fingerprint {
    fn of(certificate: &[u8]) -> Self {
        Self(Sha256::digest(certificate).into())
    }
}

pub struct Identity {
    certificate: Vec<u8>,
    key: Secret,
}
impl Identity {
    fn generate() -> Result<Self, ChannelError> {
        let certified = rcgen::generate_simple_self_signed(vec![SERVER_NAME.to_owned()])
            .map_err(|_| ChannelError::Unavailable)?;
        Ok(Self {
            certificate: certified.cert.der().to_vec(),
            key: Secret::new(certified.signing_key.serialize_der()),
        })
    }
    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of(&self.certificate)
    }
    fn encode(&self) -> Result<Secret, ChannelError> {
        let length =
            u16::try_from(self.certificate.len()).map_err(|_| ChannelError::InvalidIdentity)?;
        let mut bytes = Secret::new(b"ZDCH\x01".to_vec());
        bytes.extend_from_slice(&length.to_be_bytes());
        bytes.extend_from_slice(&self.certificate);
        bytes.extend_from_slice(&self.key);
        if bytes.len() > MAX_SECRET_BYTES {
            return Err(ChannelError::InvalidIdentity);
        }
        Ok(bytes)
    }
    fn decode(bytes: &[u8]) -> Result<Self, ChannelError> {
        if bytes.len() < 8 || bytes.len() > MAX_SECRET_BYTES || &bytes[..5] != b"ZDCH\x01" {
            return Err(ChannelError::InvalidIdentity);
        }
        let length = u16::from_be_bytes([bytes[5], bytes[6]]) as usize;
        if length == 0 || 7 + length >= bytes.len() {
            return Err(ChannelError::InvalidIdentity);
        }
        let identity = Self {
            certificate: bytes[7..7 + length].to_vec(),
            key: Secret::new(bytes[7 + length..].to_vec()),
        };
        identity.acceptor()?;
        Ok(identity)
    }
    pub fn acceptor(&self) -> Result<TlsAcceptor, ChannelError> {
        let mut config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])
                .map_err(|_| ChannelError::InvalidIdentity)?
                .with_no_client_auth()
                .with_single_cert(
                    vec![CertificateDer::from(self.certificate.clone())],
                    PrivatePkcs8KeyDer::from(self.key.to_vec()).into(),
                )
                .map_err(|_| ChannelError::InvalidIdentity)?;
        config.alpn_protocols = vec![b"http/1.1".to_vec()];
        config.send_tls13_tickets = 0;
        config.max_early_data_size = 0;
        Ok(TlsAcceptor::from(Arc::new(config)))
    }
}

pub async fn create_identity(store: &impl SecretStore) -> Result<Identity, ChannelError> {
    let identity = Identity::generate()?;
    store
        .create(Slot::ChannelIdentity, &identity.encode()?)
        .await?;
    let saved = load_identity(store)
        .await?
        .ok_or(ChannelError::Unavailable)?;
    if saved.fingerprint() != identity.fingerprint() {
        return Err(ChannelError::InvalidIdentity);
    }
    Ok(saved)
}
pub async fn load_identity(store: &impl SecretStore) -> Result<Option<Identity>, ChannelError> {
    store
        .load(Slot::ChannelIdentity)
        .await?
        .map(|secret| Identity::decode(&secret))
        .transpose()
}

#[derive(Debug)]
struct PinnedServer(Fingerprint);
impl ServerCertVerifier for PinnedServer {
    fn verify_server_cert(
        &self,
        certificate: &CertificateDer<'_>,
        intermediates: &[CertificateDer<'_>],
        name: &ServerName<'_>,
        ocsp: &[u8],
        now: UnixTime,
    ) -> Result<ServerCertVerified, rustls::Error> {
        if Fingerprint::of(certificate) != self.0 || !intermediates.is_empty() {
            return Err(rustls::Error::InvalidCertificate(
                rustls::CertificateError::UnknownIssuer,
            ));
        }
        let mut roots = RootCertStore::empty();
        roots.add(certificate.clone().into_owned())?;
        WebPkiServerVerifier::builder_with_provider(
            Arc::new(roots),
            Arc::new(rustls::crypto::ring::default_provider()),
        )
        .build()
        .map_err(|_| rustls::Error::InvalidCertificate(rustls::CertificateError::BadEncoding))?
        .verify_server_cert(certificate, intermediates, name, ocsp, now)
    }
    fn verify_tls12_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls12_signature(
            message,
            certificate,
            signature,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn verify_tls13_signature(
        &self,
        message: &[u8],
        certificate: &CertificateDer<'_>,
        signature: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, rustls::Error> {
        rustls::crypto::verify_tls13_signature(
            message,
            certificate,
            signature,
            &rustls::crypto::ring::default_provider().signature_verification_algorithms,
        )
    }
    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        rustls::crypto::ring::default_provider()
            .signature_verification_algorithms
            .supported_schemes()
    }
}

pub async fn connect<S: AsyncRead + AsyncWrite + Unpin>(
    stream: S,
    trusted: Fingerprint,
) -> Result<tokio_rustls::client::TlsStream<S>, ChannelError> {
    let mut config =
        ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|_| ChannelError::Unavailable)?
            .dangerous()
            .with_custom_certificate_verifier(Arc::new(PinnedServer(trusted)))
            .with_no_client_auth();
    config.alpn_protocols = vec![b"http/1.1".to_vec()];
    config.enable_early_data = false;
    config.resumption = rustls::client::Resumption::disabled();
    tokio::time::timeout(
        HANDSHAKE_TIMEOUT,
        TlsConnector::from(Arc::new(config)).connect(
            ServerName::try_from(SERVER_NAME).expect("constant DNS name"),
            stream,
        ),
    )
    .await
    .map_err(|_| ChannelError::TimedOut)?
    .map_err(|_| ChannelError::UntrustedPeer)
}
pub async fn accept<S: AsyncRead + AsyncWrite + Unpin>(
    acceptor: &TlsAcceptor,
    stream: S,
) -> Result<tokio_rustls::server::TlsStream<S>, ChannelError> {
    tokio::time::timeout(HANDSHAKE_TIMEOUT, acceptor.accept(stream))
        .await
        .map_err(|_| ChannelError::TimedOut)?
        .map_err(|_| ChannelError::UntrustedPeer)
}

#[cfg(test)]
mod tests;
