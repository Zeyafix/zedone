use crate::{Service, device_control::DeviceControl};
use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicU64, Ordering},
    },
};
use zedone_core::{
    error::{CommandError, ErrorCode},
    maintenance::{MaintenanceCommand, MaintenanceView},
    replay::Execution,
    wire::{CommandResponse, decode_request},
};
use zedone_storage::{
    Storage,
    emergency::{Emergency, LocalSession},
};

pub struct Maintenance {
    engine: Arc<Mutex<Emergency>>,
    service: Mutex<Option<Service>>,
    path: PathBuf,
    operations: tokio::sync::Mutex<()>,
    generation: AtomicU64,
    devices: Mutex<Vec<Weak<dyn DeviceControl>>>,
    endpoint: Mutex<Option<String>>,
}
impl Maintenance {
    pub async fn owner_access(&self, key: &str) -> Result<(), CommandError> {
        let engine = self.engine.clone();
        let key = key.to_owned();
        tokio::task::spawn_blocking(move || {
            engine
                .lock()
                .map_err(|_| zedone_storage::StorageError::Unavailable)?
                .owner_access(&key)
        })
        .await
        .map_err(|_| CommandError {
            code: ErrorCode::Unavailable,
        })?
        .map_err(CommandError::from)
    }
    pub async fn start_loopback(self: &Arc<Self>) -> Result<LocalListener, CommandError> {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|_| CommandError {
                code: ErrorCode::Unavailable,
            })?;
        let address = listener.local_addr().map_err(|_| CommandError {
            code: ErrorCode::Unavailable,
        })?;
        let endpoint = format!("http://{address}");
        self.set_endpoint(endpoint.clone());
        let router = crate::http::maintenance_router(self.clone(), None);
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, router)
                .with_graceful_shutdown(async {
                    let _ = stopped.await;
                })
                .await;
        });
        Ok(LocalListener {
            endpoint,
            stop: Some(stop),
            task,
        })
    }
    pub async fn open(
        path: PathBuf,
        service: Option<Service>,
        failure: Option<String>,
    ) -> Result<Arc<Self>, CommandError> {
        let source = path.clone();
        let engine = tokio::task::spawn_blocking(move || {
            let mut value = Emergency::open(source)?;
            if let Some(reason) = failure {
                value.activate_failed(reason)?;
            }
            Ok::<_, zedone_storage::StorageError>(value)
        })
        .await
        .map_err(|_| CommandError {
            code: ErrorCode::Unavailable,
        })?
        .map_err(CommandError::from)?;
        Ok(Arc::new(Self {
            engine: Arc::new(Mutex::new(engine)),
            service: Mutex::new(service),
            path,
            operations: tokio::sync::Mutex::new(()),
            generation: AtomicU64::new(0),
            devices: Mutex::new(Vec::new()),
            endpoint: Mutex::new(None),
        }))
    }
    pub fn service(&self) -> Option<Service> {
        self.service.lock().ok()?.clone()
    }
    pub fn generation(&self) -> u64 {
        self.generation.load(Ordering::Acquire)
    }
    pub fn register_device(&self, device: Arc<dyn DeviceControl>) {
        if let Ok(mut hooks) = self.devices.lock() {
            hooks.retain(|w| w.strong_count() > 0);
            hooks.push(Arc::downgrade(&device));
        }
    }
    pub fn local_session(&self) -> Result<LocalSession, CommandError> {
        self.engine
            .lock()
            .map_err(|_| CommandError {
                code: ErrorCode::Unavailable,
            })?
            .owner_session()
            .map_err(CommandError::from)
    }
    pub fn set_endpoint(&self, endpoint: String) {
        if let Ok(mut value) = self.endpoint.lock() {
            *value = Some(endpoint);
        }
    }
    pub async fn dispatch(
        self: &Arc<Self>,
        operation: &str,
        body: &[u8],
        key: &str,
    ) -> CommandResponse<serde_json::Value> {
        if operation != "maintenance.execute" || body.len() > crate::MAX_REQUEST_BYTES {
            return Err(CommandError {
                code: ErrorCode::InvalidRequest,
            })
            .into();
        }
        let request = match decode_request::<MaintenanceCommand>(body) {
            Ok(r) => r,
            Err(e) => return Err(e).into(),
        };
        let value = self.clone();
        let key = key.to_owned();
        let result =
            tokio::spawn(async move { value.run(key, request.id, request.parameters).await })
                .await
                .unwrap_or(Err(CommandError {
                    code: ErrorCode::OutcomeUnknown,
                }));
        result
            .and_then(|e| {
                Ok(Execution {
                    result: serde_json::to_value(e.result).map_err(|_| CommandError {
                        code: ErrorCode::Unavailable,
                    })?,
                    replayed: e.replayed,
                })
            })
            .into()
    }
    async fn run(
        &self,
        key: String,
        id: [u8; 16],
        command: MaintenanceCommand,
    ) -> Result<Execution<MaintenanceView>, CommandError> {
        let _guard = self.operations.lock().await;
        let engine = self.engine.clone();
        let check_key = key.clone();
        let checked = command.clone();
        let cached = tokio::task::spawn_blocking(move || {
            engine
                .lock()
                .map_err(|_| zedone_storage::StorageError::Unavailable)?
                .check(&check_key, id, &checked)
        })
        .await
        .map_err(|_| CommandError {
            code: ErrorCode::OutcomeUnknown,
        })?
        .map_err(CommandError::from)?;
        if let Some(result) = cached {
            return Ok(result);
        }
        if matches!(command, MaintenanceCommand::Enter { .. }) {
            let devices = self
                .devices
                .lock()
                .map_err(|_| CommandError {
                    code: ErrorCode::Unavailable,
                })?
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>();
            for device in devices {
                device.shutdown().await;
            }
            let old = self
                .service
                .lock()
                .map_err(|_| CommandError {
                    code: ErrorCode::Unavailable,
                })?
                .take();
            if let Some(service) = old {
                service
                    .storage
                    .clone()
                    .shutdown()
                    .await
                    .map_err(CommandError::from)?;
            }
            self.generation.fetch_add(1, Ordering::AcqRel);
        }
        if matches!(command, MaintenanceCommand::Retry { .. }) && self.service().is_none() {
            let engine = self.engine.clone();
            let retry_key = key.clone();
            let retry_command = command.clone();
            tokio::task::spawn_blocking(move || {
                engine
                    .lock()
                    .map_err(|_| zedone_storage::StorageError::Unavailable)?
                    .release_for_retry(&retry_key, id, &retry_command)
            })
            .await
            .map_err(|_| CommandError {
                code: ErrorCode::OutcomeUnknown,
            })?
            .map_err(CommandError::from)?;
            match Storage::open(self.path.clone()).await {
                Ok(db) => {
                    if let Err(error) = db.recover_editing_sessions().await {
                        let _ = db.shutdown().await;
                        let engine = self.engine.clone();
                        let reason = format!("{error}");
                        let _ = tokio::task::spawn_blocking(move || {
                            engine
                                .lock()
                                .map_err(|_| zedone_storage::StorageError::Unavailable)?
                                .activate_failed(reason)
                        })
                        .await;
                        return Err(error.into());
                    }
                    *self.service.lock().map_err(|_| CommandError {
                        code: ErrorCode::Unavailable,
                    })? = Some(Service::new(db).with_daily_backups());
                    self.generation.fetch_add(1, Ordering::AcqRel);
                }
                Err(error) => {
                    let engine = self.engine.clone();
                    let reason = format!("{error}");
                    let _ = tokio::task::spawn_blocking(move || {
                        engine
                            .lock()
                            .map_err(|_| zedone_storage::StorageError::Unavailable)?
                            .activate_failed(reason)
                    })
                    .await;
                    return Err(error.into());
                }
            }
        }
        let ready = self.service().is_some();
        let engine = self.engine.clone();
        let mut completed = tokio::task::spawn_blocking(move || {
            engine
                .lock()
                .map_err(|_| zedone_storage::StorageError::Unavailable)?
                .execute(&key, id, command, ready)
        })
        .await
        .map_err(|_| CommandError {
            code: ErrorCode::OutcomeUnknown,
        })?
        .map_err(CommandError::from)?;
        completed.result.endpoint = self.endpoint.lock().ok().and_then(|v| v.clone());
        Ok(completed)
    }
}

pub struct LocalListener {
    pub endpoint: String,
    stop: Option<tokio::sync::oneshot::Sender<()>>,
    task: tokio::task::JoinHandle<()>,
}
impl LocalListener {
    pub async fn shutdown(mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.task.abort();
        let _ = (&mut self.task).await;
    }
}
impl Drop for LocalListener {
    fn drop(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        self.task.abort();
    }
}
