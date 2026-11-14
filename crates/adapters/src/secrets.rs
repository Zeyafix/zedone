//! Host-only storage boundary. Never dispatch these operations from renderer JSON.
use std::future::Future;
use zeroize::Zeroizing;

pub const MAX_SECRET_BYTES: usize = 2048;
pub type Secret = Zeroizing<Vec<u8>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SecretError {
    Unavailable,
    AlreadyExists,
    InvalidSecret,
}

#[derive(Clone, Copy)]
pub enum Slot {
    ChannelIdentity,
}
impl Slot {
    pub fn name(self) -> &'static str {
        match self {
            Self::ChannelIdentity => "channel-identity-v1",
        }
    }
}

/// Hosts serialize lifecycle changes. A missing value is distinct from any
/// read/decryption failure. create never replaces an existing value.
pub trait SecretStore: Send + Sync {
    fn load(&self, slot: Slot) -> impl Future<Output = Result<Option<Secret>, SecretError>> + Send;
    fn create(
        &self,
        slot: Slot,
        secret: &[u8],
    ) -> impl Future<Output = Result<(), SecretError>> + Send;
    fn remove(&self, slot: Slot) -> impl Future<Output = Result<(), SecretError>> + Send;
}

#[cfg(windows)]
pub mod windows;
