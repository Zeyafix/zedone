use crate::{device_host::DeviceHost, network, pairing::PendingSession, secrets::SecretStore};
use serde::de::DeserializeOwned;
use zedone_core::{
    access::{Credential, RequiredAccess, SpacePermission, with_authorized},
    command::Command,
    error::{CommandError, ErrorCode},
    replay::Execution,
    sync::{
        CatalogSync, FORMAT, PROTOCOL, PullSync, PushSync, SyncCatalog, SyncControl, SyncStore,
    },
    wire::{CommandRequest, CommandResponse},
};

fn failure(code: ErrorCode) -> CommandError {
    CommandError { code }
}
async fn remote<C: Command>(session: &PendingSession, p: C) -> Result<C::Output, ErrorCode>
where
    C::Output: DeserializeOwned,
{
    let mut id = [0; 16];
    getrandom::fill(&mut id).map_err(|_| ErrorCode::Unavailable)?;
    let body = serde_json::to_vec(&CommandRequest {
        id,
        operation: C::NAME.into(),
        parameters: p,
    })
    .map_err(|_| ErrorCode::InvalidRequest)?;
    match network::command(
        session.address.ok_or(ErrorCode::Unavailable)?,
        session.fingerprint(),
        &format!("/v1/commands/{}", C::NAME),
        session.credential().0,
        &body,
    )
    .await?
    {
        CommandResponse::Ok(value) => {
            serde_json::from_value(value.result).map_err(|_| ErrorCode::OutcomeUnknown)
        }
        CommandResponse::Error(value) => Err(value.error.code),
    }
}
fn local<T>(
    tx: &mut zedone_storage::WriteContext<'_>,
    clock: &impl zedone_core::access::Clock,
    key: &[u8; 32],
    space: &str,
    pairing: &str,
    expected: Option<&SyncControl>,
    work: impl FnOnce(
        &mut zedone_storage::WriteContext<'_>,
        &str,
        SyncControl,
    ) -> zedone_storage::StorageResult<T>,
) -> zedone_storage::StorageResult<T> {
    let mut required = vec![RequiredAccess::ManageDevices];
    if tx.sync_space_exists(space)? {
        required.push(RequiredAccess::Space {
            id: space,
            permission: SpacePermission::ReadWrite,
        });
    }
    Ok(with_authorized(
        tx,
        &Credential(key),
        &required,
        clock,
        |tx, subject| {
            let control = tx.sync_control(space, pairing)?;
            if !control.selected || control.paused {
                return Err(failure(ErrorCode::Conflict));
            }
            if let Some(e) = expected
                && (e.selection_version != control.selection_version
                    || e.pause_version != control.pause_version)
            {
                return Err(failure(ErrorCode::StaleState));
            }
            work(tx, subject, control).map_err(CommandError::from)
        },
    )?)
}
impl<S: SecretStore + 'static> DeviceHost<S> {
    async fn remote_catalog(&self, session: &PendingSession) -> Result<SyncCatalog, ErrorCode> {
        let catalog = remote(session, CatalogSync {}).await?;
        if catalog.pairing != session.request.parameters.request {
            return Err(ErrorCode::InvalidRequest);
        }
        if catalog.protocol != PROTOCOL || catalog.format != FORMAT {
            return Err(ErrorCode::IncompatibleSync);
        }
        Ok(catalog)
    }
    pub(crate) async fn exchange_dispatch(
        &self,
        operation: &str,
        body: &[u8],
        key: &str,
    ) -> CommandResponse<serde_json::Value> {
        let result = async {
            self.authorize(key).await?;
            if body.len() > crate::MAX_REQUEST_BYTES {
                return Err(ErrorCode::InvalidRequest);
            }
            let _guard = self.operations.lock().await;
            let session = self
                .sessions
                .connected()
                .await?
                .ok_or(ErrorCode::AccessDenied)?;
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct Run {
                space: String,
            }
            let result = match operation {
                "exchange.catalog" => {
                    let value: serde_json::Value =
                        serde_json::from_slice(body).map_err(|_| ErrorCode::InvalidRequest)?;
                    if value != serde_json::json!({}) {
                        return Err(ErrorCode::InvalidRequest);
                    }
                    serde_json::to_value(self.remote_catalog(&session).await?)
                        .map_err(|_| ErrorCode::Unavailable)?
                }
                "exchange.run" => {
                    let input: Run =
                        serde_json::from_slice(body).map_err(|_| ErrorCode::InvalidRequest)?;
                    tokio::time::timeout(
                        std::time::Duration::from_secs(15),
                        self.exchange_space(&session, &input.space, key),
                    )
                    .await
                    .map_err(|_| ErrorCode::OutcomeUnknown)??
                }
                _ => return Err(ErrorCode::InvalidRequest),
            };
            Ok(Execution {
                result,
                replayed: false,
            })
        }
        .await
        .map_err(failure);
        result.into()
    }
    async fn exchange_space(
        &self,
        session: &PendingSession,
        space: &str,
        key: &str,
    ) -> Result<serde_json::Value, ErrorCode> {
        if !zedone_core::invitations::valid_id(space) {
            return Err(ErrorCode::InvalidRequest);
        }
        let secret = crate::decode_key(key).map_err(|e| e.code)?;
        let pairing = session.request.parameters.request.clone();
        let space = space.to_owned();
        let (s, p) = (space.clone(), pairing.clone());
        self.service
            .storage
            .timed_transact(move |tx, clock| {
                local(tx, clock, &secret, &s, &p, None, |_, _, _| Ok(()))
            })
            .await
            .map_err(|e| CommandError::from(e).code)?;
        let catalog = self.remote_catalog(session).await?;
        let entry = catalog
            .spaces
            .iter()
            .find(|s| s.space == space)
            .ok_or(ErrorCode::AccessDenied)?;
        if !entry.selected || entry.paused {
            return Ok(
                serde_json::json!({"space":space,"state":if entry.paused{"remote_paused"}else{"remote_unselected"}}),
            );
        }
        let writable = entry.writable;
        let mut received = 0usize;
        let mut sent = 0usize;
        let mut pulled = false;
        for _ in 0..32 {
            let (s, p) = (space.clone(), pairing.clone());
            let (control, known) = self
                .service
                .storage
                .timed_transact(move |tx, clock| {
                    local(tx, clock, &secret, &s, &p, None, |tx, _, control| {
                        Ok((control, tx.sync_known(&s, &p)?))
                    })
                })
                .await
                .map_err(|e| CommandError::from(e).code)?;
            let incoming = remote(
                session,
                PullSync {
                    protocol: PROTOCOL,
                    space: space.clone(),
                    known,
                },
            )
            .await?;
            received += incoming.changes.len();
            let complete = incoming.complete;
            let (s, p) = (space.clone(), pairing.clone());
            self.service
                .storage
                .timed_transact(move |tx, clock| {
                    local(
                        tx,
                        clock,
                        &secret,
                        &s,
                        &p,
                        Some(&control),
                        |tx, subject, _| tx.receive_sync_page(&s, &p, subject, &incoming),
                    )
                })
                .await
                .map_err(|e| CommandError::from(e).code)?;
            if complete {
                pulled = true;
                break;
            }
        }
        let (s, p) = (space.clone(), pairing.clone());
        let exists = self
            .service
            .storage
            .timed_transact(move |tx, clock| {
                local(tx, clock, &secret, &s, &p, None, |tx, _, _| {
                    Ok(tx.sync_space_exists(&s)?)
                })
            })
            .await
            .map_err(|e| CommandError::from(e).code)?;
        if !exists || !pulled {
            return Ok(
                serde_json::json!({"space":space,"state":"in_progress","received":received,"sent":0}),
            );
        }
        if writable {
            for _ in 0..32 {
                let (s, p) = (space.clone(), pairing.clone());
                let (control, outgoing) = self
                    .service
                    .storage
                    .timed_transact(move |tx, clock| {
                        local(tx, clock, &secret, &s, &p, None, |tx, _, control| {
                            Ok((control, tx.outgoing_sync_page(&s, &p)?))
                        })
                    })
                    .await
                    .map_err(|e| CommandError::from(e).code)?;
                let complete = outgoing.complete;
                sent += outgoing.changes.len();
                let expected_heads = outgoing.heads.clone();
                let ack = remote(
                    session,
                    PushSync {
                        protocol: PROTOCOL,
                        space: space.clone(),
                        base: outgoing.base.clone(),
                        changes: outgoing.changes,
                    },
                )
                .await?;
                if ack.heads != expected_heads || ack.base != outgoing.base || !ack.complete {
                    return Err(ErrorCode::OutcomeUnknown);
                }
                let (s, p) = (space.clone(), pairing.clone());
                self.service
                    .storage
                    .timed_transact(move |tx, clock| {
                        local(tx, clock, &secret, &s, &p, Some(&control), |tx, _, _| {
                            tx.acknowledge_sync(&s, &p, &ack)
                        })
                    })
                    .await
                    .map_err(|e| CommandError::from(e).code)?;
                if complete {
                    return Ok(
                        serde_json::json!({"space":space,"state":"up_to_date","received":received,"sent":sent}),
                    );
                }
            }
        } else {
            return Ok(
                serde_json::json!({"space":space,"state":"read_only","received":received,"sent":0}),
            );
        }
        Ok(serde_json::json!({"space":space,"state":"in_progress","received":received,"sent":sent}))
    }
    pub async fn exchange_automatic(&self, key: &str) -> Result<(), ErrorCode> {
        self.authorize(key).await?;
        let _guard = self.operations.lock().await;
        let Some(session) = self.sessions.connected().await? else {
            return Ok(());
        };
        let pairing = session.request.parameters.request.clone();
        let spaces = self
            .service
            .storage
            .transact(move |tx| tx.selected_sync_spaces(&pairing))
            .await
            .map_err(|e| CommandError::from(e).code)?;
        drop(_guard);
        for space in spaces {
            let _guard = self.operations.lock().await;
            let Some(current) = self.sessions.connected().await? else {
                break;
            };
            if current.request.parameters.request != session.request.parameters.request {
                break;
            }
            let _ = tokio::time::timeout(
                std::time::Duration::from_secs(15),
                self.exchange_space(&current, &space, key),
            )
            .await;
        }
        Ok(())
    }
}
