//! Current Windows user, this machine, explicit installation namespace.
use super::{MAX_SECRET_BYTES, Secret, SecretError, SecretStore, Slot};
use keyring_core::{Entry, api::CredentialStoreApi};
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    sync::Mutex,
};

// Credential Manager does not reliably order operations on separate threads.
// No global default store is installed, and tests never enumerate user entries.
static OPERATIONS: Mutex<()> = Mutex::new(());

pub struct WindowsSecrets {
    namespace: String,
    lock_directory: PathBuf,
}
impl WindowsSecrets {
    pub fn new(namespace: [u8; 32], lock_directory: &Path) -> Self {
        Self {
            namespace: namespace.iter().map(|b| format!("{b:02x}")).collect(),
            lock_directory: lock_directory.to_owned(),
        }
    }
    fn lock(&self) -> Result<File, SecretError> {
        std::fs::create_dir_all(&self.lock_directory).map_err(|_| SecretError::Unavailable)?;
        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(self.lock_directory.join(format!("{}.lock", self.namespace)))
            .map_err(|_| SecretError::Unavailable)?;
        file.lock().map_err(|_| SecretError::Unavailable)?;
        Ok(file)
    }
    fn entry(&self, slot: Slot) -> Result<Entry, SecretError> {
        windows_native_keyring_store::Store::new()
            .and_then(|store| {
                store.build(
                    "app.zedone.local",
                    &self.namespace,
                    Some(&HashMap::from([
                        ("persistence", "Local"),
                        (
                            "target",
                            format!("ZeDone/{}/{}", self.namespace, slot.name()).as_str(),
                        ),
                    ])),
                )
            })
            .map_err(|_| SecretError::Unavailable)
    }
    fn read(entry: &Entry) -> Result<Option<Secret>, SecretError> {
        match entry.get_secret() {
            Ok(value) if !value.is_empty() && value.len() <= MAX_SECRET_BYTES => {
                Ok(Some(Secret::new(value)))
            }
            Ok(value) => {
                drop(Secret::new(value));
                Err(SecretError::InvalidSecret)
            }
            Err(keyring_core::Error::NoEntry) => Ok(None),
            Err(_) => Err(SecretError::Unavailable),
        }
    }
}
impl SecretStore for WindowsSecrets {
    async fn load(&self, slot: Slot) -> Result<Option<Secret>, SecretError> {
        let _guard = OPERATIONS.lock().map_err(|_| SecretError::Unavailable)?;
        let _file = self.lock()?;
        Self::read(&self.entry(slot)?)
    }
    async fn create(&self, slot: Slot, secret: &[u8]) -> Result<(), SecretError> {
        if secret.is_empty() || secret.len() > MAX_SECRET_BYTES {
            return Err(SecretError::InvalidSecret);
        }
        let _guard = OPERATIONS.lock().map_err(|_| SecretError::Unavailable)?;
        let _file = self.lock()?;
        let entry = self.entry(slot)?;
        if Self::read(&entry)?.is_some() {
            return Err(SecretError::AlreadyExists);
        }
        entry
            .set_secret(secret)
            .map_err(|_| SecretError::Unavailable)
    }
    async fn remove(&self, slot: Slot) -> Result<(), SecretError> {
        let _guard = OPERATIONS.lock().map_err(|_| SecretError::Unavailable)?;
        let _file = self.lock()?;
        match self.entry(slot)?.delete_credential() {
            Ok(()) | Err(keyring_core::Error::NoEntry) => Ok(()),
            Err(_) => Err(SecretError::Unavailable),
        }
    }
}
