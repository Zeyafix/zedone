pub mod access;
mod card_history;
mod cards;
mod cleanup;
mod clock;
mod columns;
mod document;
mod invitations;
pub mod migration_backup;
mod ordering;
mod pairing;
mod schema;
pub mod snapshot;
mod spaces;
pub mod startup;
mod tags;
mod version;

#[cfg(test)]
mod replay_tests;
#[cfg(test)]
mod tests;

use std::{fmt, panic::AssertUnwindSafe, path::PathBuf, thread};

use rusqlite::{Connection, TransactionBehavior};
use serde::Serialize;
use tokio::sync::{mpsc, oneshot};
use zedone_core::error::{CommandError, ErrorCode};

pub use document::{ReadContext, RecordedResult, WriteContext};
pub use version::state_version;

pub type StorageResult<T> = Result<T, StorageError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageError {
    Unavailable,
    OutcomeUnknown,
    UnsupportedSchema,
    InvalidDatabase,
    RetryRequired,
    AlreadyRecorded,
    Rejected(CommandError),
}

impl fmt::Display for StorageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for StorageError {}
impl From<rusqlite::Error> for StorageError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Unavailable
    }
}
impl From<automerge::AutomergeError> for StorageError {
    fn from(_: automerge::AutomergeError) -> Self {
        Self::Unavailable
    }
}
impl From<CommandError> for StorageError {
    fn from(value: CommandError) -> Self {
        Self::Rejected(value)
    }
}
impl From<StorageError> for CommandError {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::Rejected(error) => error,
            StorageError::OutcomeUnknown => Self {
                code: ErrorCode::OutcomeUnknown,
            },
            _ => Self {
                code: ErrorCode::Unavailable,
            },
        }
    }
}

type Job = Box<dyn FnOnce(&mut Connection) + Send>;
enum Message {
    Run(Job),
    Stop(oneshot::Sender<()>),
}

#[derive(Clone)]
pub struct Storage {
    sender: mpsc::Sender<Message>,
    registry: schema::Registry,
    clock_anchor: std::sync::Arc<std::sync::Mutex<Option<(std::time::Instant, i64)>>>,
}

struct Opening {
    sender: mpsc::Sender<Message>,
    ready: oneshot::Receiver<StorageResult<()>>,
}

impl Storage {
    pub async fn open(path: impl Into<PathBuf>) -> StorageResult<Self> {
        startup::Startup::new(path).automatic(None).await
    }

    fn begin_open(
        path: PathBuf,
        options: Option<startup::MigrationOptions>,
        registry: schema::Registry,
    ) -> StorageResult<Opening> {
        let (sender, mut receiver) = mpsc::channel(32);
        let (ready, opened) = oneshot::channel();
        thread::Builder::new()
            .name("zedone-storage".into())
            .spawn(move || {
                let initialized: StorageResult<Connection> = (|| {
                    schema::preflight_with(&path, registry)?;
                    let mut connection = Connection::open(path)?;
                    startup::initialize(&mut connection, options.as_ref(), registry, |_| Ok(()))?;
                    Ok(connection)
                })();
                let mut connection = match initialized {
                    Ok(c) => c,
                    Err(error) => {
                        let _ = ready.send(Err(error));
                        return;
                    }
                };
                if ready.send(Ok(())).is_err() {
                    return;
                }
                let mut stopped = Vec::new();
                while let Some(message) = receiver.blocking_recv() {
                    match message {
                        Message::Run(job) => job(&mut connection),
                        Message::Stop(reply) => {
                            receiver.close();
                            stopped.push(reply);
                        }
                    }
                }
                drop(connection);
                for reply in stopped {
                    let _ = reply.send(());
                }
            })
            .map_err(|_| StorageError::Unavailable)?;
        Ok(Opening {
            sender,
            ready: opened,
        })
    }

    async fn run<R: Send + 'static>(
        &self,
        work: impl FnOnce(&mut Connection) -> StorageResult<R> + Send + 'static,
    ) -> StorageResult<R> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Message::Run(Box::new(move |connection| {
                let result = std::panic::catch_unwind(AssertUnwindSafe(|| work(connection)))
                    .unwrap_or(Err(StorageError::OutcomeUnknown));
                let _ = reply.send(result);
            })))
            .await
            .map_err(|_| StorageError::Unavailable)?;
        response.await.map_err(|_| StorageError::OutcomeUnknown)?
    }

    pub async fn read<R: Send + 'static>(
        &self,
        work: impl FnOnce(&mut ReadContext<'_>) -> StorageResult<R> + Send + 'static,
    ) -> StorageResult<R> {
        let registry = self.registry;
        self.run(move |connection| {
            let tx = connection.transaction()?;
            registry.check_version(&tx)?;
            let result = work(&mut ReadContext::new(&tx))?;
            tx.commit()?;
            Ok(result)
        })
        .await
    }

    pub async fn write<R: Serialize + Send + 'static>(
        &self,
        key: Vec<u8>,
        request: Vec<u8>,
        work: impl FnOnce(&mut WriteContext<'_>) -> StorageResult<R> + Send + 'static,
    ) -> StorageResult<R> {
        self.write_checked(key, request, work, |_| Ok(())).await
    }

    async fn write_checked<R: Serialize + Send + 'static>(
        &self,
        key: Vec<u8>,
        request: Vec<u8>,
        work: impl FnOnce(&mut WriteContext<'_>) -> StorageResult<R> + Send + 'static,
        checkpoint: impl Fn(Stage) -> StorageResult<()> + Send + 'static,
    ) -> StorageResult<R> {
        let registry = self.registry;
        self.run(move |connection| {
            if key.is_empty() {
                return Err(StorageError::Unavailable);
            }
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            registry.check_version(&tx)?;
            let mut context = WriteContext::new(&tx);
            if context.read().recorded(&key)?.is_some() {
                return Err(StorageError::AlreadyRecorded);
            }
            let result = work(&mut context)?;
            checkpoint(Stage::AfterWork)?;
            context.flush()?;
            checkpoint(Stage::AfterChanges)?;
            let bytes = serde_json::to_vec(&result).map_err(|_| StorageError::Unavailable)?;
            tx.execute(
                "INSERT INTO command_results(key, request, result) VALUES (?1, ?2, ?3)",
                (&key, &request, bytes),
            )?;
            checkpoint(Stage::AfterResult)?;
            checkpoint(Stage::BeforeCommit)?;
            tx.commit().map_err(|_| StorageError::OutcomeUnknown)?;
            checkpoint(Stage::AfterCommit).map_err(|_| StorageError::OutcomeUnknown)?;
            Ok(result)
        })
        .await
    }

    pub async fn transact<R: Send + 'static>(
        &self,
        work: impl FnOnce(&mut WriteContext<'_>) -> StorageResult<R> + Send + 'static,
    ) -> StorageResult<R> {
        self.transact_checked(work, |_| Ok(())).await
    }

    async fn transact_checked<R: Send + 'static>(
        &self,
        work: impl FnOnce(&mut WriteContext<'_>) -> StorageResult<R> + Send + 'static,
        checkpoint: impl Fn(Stage) -> StorageResult<()> + Send + 'static,
    ) -> StorageResult<R> {
        let registry = self.registry;
        self.run(move |connection| {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            registry.check_version(&tx)?;
            let mut context = WriteContext::new(&tx);
            let result = work(&mut context)?;
            checkpoint(Stage::AfterWork)?;
            context.flush()?;
            checkpoint(Stage::AfterChanges)?;
            checkpoint(Stage::AfterResult)?;
            checkpoint(Stage::BeforeCommit)?;
            tx.commit().map_err(|_| StorageError::OutcomeUnknown)?;
            checkpoint(Stage::AfterCommit).map_err(|_| StorageError::OutcomeUnknown)?;
            Ok(result)
        })
        .await
    }

    pub async fn shutdown(self) -> StorageResult<()> {
        let (reply, response) = oneshot::channel();
        self.sender
            .send(Message::Stop(reply))
            .await
            .map_err(|_| StorageError::Unavailable)?;
        response.await.map_err(|_| StorageError::Unavailable)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    AfterWork,
    AfterChanges,
    AfterResult,
    BeforeCommit,
    AfterCommit,
}
