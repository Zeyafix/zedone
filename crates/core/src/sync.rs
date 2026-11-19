use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission, with_authorized},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    pairing::{PairingStatus, PairingStore},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, SpaceStore},
    wire::{CommandRequest, CommandResponse},
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub type Heads = Vec<[u8; 32]>;
fn heads_schema() -> utoipa::openapi::schema::Array {
    utoipa::openapi::schema::ArrayBuilder::new()
        .items(
            utoipa::openapi::RefOr::<utoipa::openapi::schema::Schema>::from(
                crate::replay::byte_array_schema::<32>(),
            ),
        )
        .max_items(Some(1024))
        .build()
}
pub const PROTOCOL: u32 = 1;
pub const FORMAT: u32 = 1;
pub const PAGE_BYTES: usize = 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SyncControl {
    pub space: String,
    pub pairing: String,
    pub selected: bool,
    pub paused: bool,
    pub selection_version: StateVersion,
    pub pause_version: StateVersion,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SyncEntry {
    pub space: String,
    pub names: Vec<String>,
    pub writable: bool,
    pub selected: bool,
    pub paused: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SyncCatalog {
    pub protocol: u32,
    pub format: u32,
    pub pairing: String,
    pub spaces: Vec<SyncEntry>,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct InspectSync {
    pub space: String,
    pub pairing: String,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SelectSync {
    pub space: String,
    pub pairing: String,
    pub selected: bool,
    pub expected: StateVersion,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PauseSync {
    pub space: String,
    pub pairing: String,
    pub paused: bool,
    pub expected: StateVersion,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CatalogSync {}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PullSync {
    pub protocol: u32,
    pub space: String,
    #[schema(schema_with = heads_schema)]
    pub known: Heads,
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct PushSync {
    pub protocol: u32,
    pub space: String,
    #[schema(schema_with = heads_schema)]
    pub base: Heads,
    pub changes: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SyncPage {
    pub protocol: u32,
    pub space: String,
    #[schema(schema_with = heads_schema)]
    pub base: Heads,
    #[schema(schema_with = heads_schema)]
    pub heads: Heads,
    pub changes: Vec<String>,
    pub complete: bool,
}
macro_rules! command {
    ($t:ty,$name:literal,$out:ty) => {
        impl Command for $t {
            const NAME: &'static str = $name;
            type Output = $out;
        }
    };
}
command!(InspectSync, "sync.inspect", SyncControl);
command!(SelectSync, "sync.select", SyncControl);
command!(PauseSync, "sync.pause", SyncControl);
command!(CatalogSync, "sync.catalog", SyncCatalog);
command!(PullSync, "sync.pull", SyncPage);
command!(PushSync, "sync.push", SyncPage);

pub trait SyncStore: SpaceStore + PairingStore {
    fn sync_catalog_spaces(
        &mut self,
        subject: &str,
    ) -> CommandResult<Vec<crate::spaces::SpaceSnapshot>>;
    fn sync_control(&mut self, space: &str, pairing: &str) -> CommandResult<SyncControl>;
    fn select_sync(&mut self, p: &SelectSync) -> CommandResult<SyncControl>;
    fn pause_sync(&mut self, p: &PauseSync) -> CommandResult<SyncControl>;
    fn sync_space_exists(&mut self, space: &str) -> CommandResult<bool>;
    fn pull_sync(&mut self, space: &str, pairing: &str, known: &Heads) -> CommandResult<SyncPage>;
    fn push_sync(
        &mut self,
        space: &str,
        pairing: &str,
        base: &Heads,
        changes: &[String],
    ) -> CommandResult<SyncPage>;
}
fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn validate<C: Command>(r: &CommandRequest<C>) -> CommandResult<()> {
    if r.operation != C::NAME || r.id == [0; 16] {
        Err(error(ErrorCode::InvalidRequest))
    } else {
        Ok(())
    }
}
fn target(space: &str, pairing: &str) -> CommandResult<()> {
    if !crate::spaces::valid_id(space) || !crate::spaces::valid_id(pairing) {
        Err(error(ErrorCode::InvalidRequest))
    } else {
        Ok(())
    }
}
fn paired(tx: &mut impl SyncStore, subject: &str) -> CommandResult<()> {
    if tx
        .pairing(subject)?
        .is_none_or(|r| r.snapshot.status != PairingStatus::Confirmed)
    {
        return Err(error(ErrorCode::AccessDenied));
    }
    Ok(())
}
fn management(
    tx: &mut impl SyncStore,
    key: &Credential<'_>,
    clock: &impl Clock,
    space: &str,
) -> CommandResult<()> {
    let mut required = vec![RequiredAccess::ManageDevices];
    if tx.sync_space_exists(space)? {
        required.push(RequiredAccess::Space {
            id: space,
            permission: SpacePermission::ReadWrite,
        });
    }
    with_authorized(tx, key, &required, clock, |_, _| Ok(()))
}

#[utoipa::path(post,path="/v1/commands/sync.inspect",operation_id="sync_inspect",request_body=CommandRequest<InspectSync>,responses((status=200,body=CommandResponse<SyncControl>)),security(("access_key"=[])))]
pub fn inspect(
    tx: &mut impl SyncStore,
    key: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<InspectSync>,
) -> CommandResult<Execution<SyncControl>> {
    validate(r)?;
    target(&r.parameters.space, &r.parameters.pairing)?;
    management(tx, key, clock, &r.parameters.space)?;
    Ok(Execution {
        result: tx.sync_control(&r.parameters.space, &r.parameters.pairing)?,
        replayed: false,
    })
}
#[utoipa::path(post,path="/v1/commands/sync.select",operation_id="sync_select",request_body=CommandRequest<SelectSync>,responses((status=200,body=CommandResponse<SyncControl>)),security(("access_key"=[])))]
pub fn select(
    tx: &mut impl SyncStore,
    key: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<SelectSync>,
) -> CommandResult<Execution<SyncControl>> {
    validate(r)?;
    target(&r.parameters.space, &r.parameters.pairing)?;
    management(tx, key, clock, &r.parameters.space)?;
    replay::execute(
        tx,
        key,
        clock,
        &[RequiredAccess::ManageDevices],
        r.id,
        &r.parameters,
        |tx| tx.select_sync(&r.parameters),
    )
}
#[utoipa::path(post,path="/v1/commands/sync.pause",operation_id="sync_pause",request_body=CommandRequest<PauseSync>,responses((status=200,body=CommandResponse<SyncControl>)),security(("access_key"=[])))]
pub fn pause(
    tx: &mut impl SyncStore,
    key: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<PauseSync>,
) -> CommandResult<Execution<SyncControl>> {
    validate(r)?;
    target(&r.parameters.space, &r.parameters.pairing)?;
    management(tx, key, clock, &r.parameters.space)?;
    replay::execute(
        tx,
        key,
        clock,
        &[RequiredAccess::ManageDevices],
        r.id,
        &r.parameters,
        |tx| tx.pause_sync(&r.parameters),
    )
}
#[utoipa::path(post,path="/v1/commands/sync.catalog",operation_id="sync_catalog",request_body=CommandRequest<CatalogSync>,responses((status=200,body=CommandResponse<SyncCatalog>)),security(("access_key"=[])))]
pub fn catalog(
    tx: &mut impl SyncStore,
    key: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<CatalogSync>,
) -> CommandResult<Execution<SyncCatalog>> {
    validate(r)?;
    let access = tx
        .resolve_access(key)?
        .ok_or(error(ErrorCode::AccessDenied))?;
    with_authorized(
        tx,
        key,
        &[RequiredAccess::ActiveGrant],
        clock,
        |tx, subject| {
            paired(tx, subject)?;
            let mut spaces = Vec::new();
            for s in tx.sync_catalog_spaces(subject)? {
                let control = tx.sync_control(&s.space, subject)?;
                if s.state != SpaceState::Active && !control.selected {
                    continue;
                }
                let writable = access.grant.scope.spaces().and_then(|m| m.get(&s.space))
                    == Some(&SpacePermission::ReadWrite);
                spaces.push(SyncEntry {
                    space: s.space,
                    names: s.names,
                    writable,
                    selected: control.selected,
                    paused: control.paused,
                });
            }
            Ok(Execution {
                result: SyncCatalog {
                    protocol: PROTOCOL,
                    format: FORMAT,
                    pairing: subject.into(),
                    spaces,
                },
                replayed: false,
            })
        },
    )
}
fn admitted<T: SyncStore, R>(
    tx: &mut T,
    key: &Credential<'_>,
    clock: &impl Clock,
    space: &str,
    write: bool,
    work: impl FnOnce(&mut T, &str) -> CommandResult<R>,
) -> CommandResult<Execution<R>> {
    with_authorized(
        tx,
        key,
        &[RequiredAccess::Space {
            id: space,
            permission: if write {
                SpacePermission::ReadWrite
            } else {
                SpacePermission::Read
            },
        }],
        clock,
        |tx, subject| {
            paired(tx, subject)?;
            let c = tx.sync_control(space, subject)?;
            if !c.selected || c.paused {
                return Err(error(ErrorCode::Conflict));
            }
            Ok(Execution {
                result: work(tx, subject)?,
                replayed: false,
            })
        },
    )
}
#[utoipa::path(post,path="/v1/commands/sync.pull",operation_id="sync_pull",request_body=CommandRequest<PullSync>,responses((status=200,body=CommandResponse<SyncPage>)),security(("access_key"=[])))]
pub fn pull(
    tx: &mut impl SyncStore,
    key: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<PullSync>,
) -> CommandResult<Execution<SyncPage>> {
    validate(r)?;
    if !crate::spaces::valid_id(&r.parameters.space) || r.parameters.known.len() > 1024 {
        return Err(error(ErrorCode::InvalidRequest));
    }
    admitted(tx, key, clock, &r.parameters.space, false, |tx, subject| {
        if r.parameters.protocol != PROTOCOL {
            return Err(error(ErrorCode::IncompatibleSync));
        }
        tx.pull_sync(&r.parameters.space, subject, &r.parameters.known)
    })
}
#[utoipa::path(post,path="/v1/commands/sync.push",operation_id="sync_push",request_body=CommandRequest<PushSync>,responses((status=200,body=CommandResponse<SyncPage>)),security(("access_key"=[])))]
pub fn push(
    tx: &mut impl SyncStore,
    key: &Credential<'_>,
    clock: &impl Clock,
    r: &CommandRequest<PushSync>,
) -> CommandResult<Execution<SyncPage>> {
    validate(r)?;
    if !crate::spaces::valid_id(&r.parameters.space) || r.parameters.base.len() > 1024 {
        return Err(error(ErrorCode::InvalidRequest));
    }
    admitted(
        tx,
        key,
        clock,
        &r.parameters.space,
        !r.parameters.changes.is_empty(),
        |tx, subject| {
            if r.parameters.protocol != PROTOCOL {
                return Err(error(ErrorCode::IncompatibleSync));
            }
            tx.push_sync(
                &r.parameters.space,
                subject,
                &r.parameters.base,
                &r.parameters.changes,
            )
        },
    )
}
