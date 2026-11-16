use std::{path::PathBuf, time::Duration};

use rusqlite::{Connection, TransactionBehavior};

use crate::{
    Storage, StorageError, StorageResult,
    migration_backup::{BackupLimits, publication},
    schema::{self, Registry},
};

#[derive(Clone, Debug)]
pub struct MigrationOptions {
    pub folder: PathBuf,
    pub created_at_unix_ms: u64,
    pub limits: BackupLimits,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StartupState {
    New,
    Running,
    Failed(StorageError),
    Ready,
}

pub struct Startup {
    path: PathBuf,
    state: StartupState,
    pending: Option<crate::Opening>,
    pub(crate) registry: Registry,
}

impl Startup {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            state: StartupState::New,
            pending: None,
            registry: schema::REGISTRY,
        }
    }

    pub fn state(&self) -> StartupState {
        self.state
    }

    pub async fn automatic(&mut self, options: Option<MigrationOptions>) -> StorageResult<Storage> {
        if self.state != StartupState::New {
            return Err(StorageError::RetryRequired);
        }
        self.begin(options)?;
        self.finish().await
    }

    pub async fn retry(&mut self, options: Option<MigrationOptions>) -> StorageResult<Storage> {
        if self.state == StartupState::Ready {
            return Err(StorageError::RetryRequired);
        }
        if self.pending.is_none() {
            self.begin(options)?;
        }
        self.finish().await
    }

    fn begin(&mut self, options: Option<MigrationOptions>) -> StorageResult<()> {
        self.state = StartupState::Running;
        match Storage::begin_open(self.path.clone(), options, self.registry) {
            Ok(opening) => {
                self.pending = Some(opening);
                Ok(())
            }
            Err(error) => {
                self.state = StartupState::Failed(error);
                Err(error)
            }
        }
    }

    async fn finish(&mut self) -> StorageResult<Storage> {
        let pending = self.pending.as_mut().ok_or(StorageError::Unavailable)?;
        let result = (&mut pending.ready)
            .await
            .map_err(|_| StorageError::OutcomeUnknown)
            .and_then(|r| r);
        let opening = self.pending.take().ok_or(StorageError::Unavailable)?;
        self.state = match result {
            Ok(()) => StartupState::Ready,
            Err(error) => StartupState::Failed(error),
        };
        result?;
        Ok(Storage {
            sender: opening.sender,
            registry: self.registry,
            clock_anchor: Default::default(),
        })
    }
}

pub(crate) fn initialize(
    connection: &mut Connection,
    options: Option<&MigrationOptions>,
    registry: Registry,
    checkpoint: impl Fn(publication::Point) -> StorageResult<()>,
) -> StorageResult<()> {
    connection.busy_timeout(Duration::from_secs(1))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    {
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if schema::empty(&tx)? {
            schema::create(&tx, registry)?;
        }
        registry.validate(&tx)?;
        tx.commit()?;
    }
    let needs_migration = registry.known(connection)?.version != registry.current().version;
    let has_witness = publication::has_commit_witness(connection)?;
    if needs_migration || has_witness {
        let options = options.ok_or(StorageError::Unavailable)?;
        publication::migrate_startup(connection, options, registry, checkpoint)?;
    }
    registry.target(connection)?;
    schema::configure(connection)
}

#[cfg(test)]
mod tests;
