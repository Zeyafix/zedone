use crate::{
    StorageError, StorageResult, backup_io, migration_backup,
    ownership::{self, Ownership},
    schema,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};
use zedone_core::{
    error::{CommandError, ErrorCode},
    maintenance::{
        MaintenanceAccess, MaintenanceCommand, MaintenanceView, RecoveryPhase, RecoveryPlan,
    },
    replay::{CommandId, Execution, StateVersion},
};

mod files;
mod restore;
#[cfg(test)]
mod tests;
use files::{RAW_LIMIT, io, read, save};

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocalSession {
    pub subject: String,
    pub key: String,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    info: MaintenanceAccess,
    verifier: [u8; 32],
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Receipt {
    request: Vec<u8>,
    result: MaintenanceView,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Deletion {
    manifest: Vec<u8>,
    bindings: Vec<(String, Vec<u8>)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Plan {
    public: RecoveryPlan,
    candidate: crate::problem_state::Entry,
    local: LocalSession,
    namespace: [u8; 32],
    apply: Option<(String, Vec<u8>)>,
    rollback: Option<(String, Vec<u8>)>,
    rollback_source: Option<String>,
    #[serde(default)]
    superseded: Vec<(String, Vec<u8>)>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    version: u32,
    identity: String,
    revision: u64,
    last_ms: u64,
    root: String,
    blocked: bool,
    #[serde(default)]
    current_epoch: Option<[u8; 32]>,
    reason: Option<String>,
    plan: Option<Plan>,
    problems: Vec<String>,
    grants: Vec<Grant>,
    receipts: BTreeMap<String, Receipt>,
    #[serde(default)]
    deleting: BTreeMap<String, Deletion>,
}
pub struct Emergency {
    path: PathBuf,
    directory: PathBuf,
    exclusive: Option<Ownership>,
}
#[derive(Clone)]
pub struct Role {
    pub subject: String,
    pub owner: bool,
    pub writable: bool,
    pub expires_ms: Option<u64>,
}
const EPOCH_KEY: &[u8] = b"\0zedone/internal/recovery-epoch/v1";
pub(crate) fn validate_epoch(path: &Path, connection: &rusqlite::Connection) -> StorageResult<()> {
    use rusqlite::OptionalExtension;
    let directory = ownership::sibling(&ownership::database_path(path)?, ".maintenance");
    let expected = if directory.exists() {
        let state: State = read(&directory.join("state.json"))?;
        state.current_epoch
    } else {
        None
    };
    if schema::empty(connection)? {
        return if directory.exists() {
            Err(StorageError::RetryRequired)
        } else {
            Ok(())
        };
    }
    let row: Option<(Vec<u8>, Vec<u8>)> = connection
        .query_row(
            "SELECT request,substr(result,1,1025) FROM command_results WHERE key=?1",
            [EPOCH_KEY],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let actual = match row {
        None => None,
        Some((request, result)) => {
            if request != b"recovery-epoch-v1" {
                return Err(StorageError::InvalidDatabase);
            }
            Some(
                serde_json::from_slice::<[u8; 32]>(&result)
                    .map_err(|_| StorageError::InvalidDatabase)?,
            )
        }
    };
    if actual != expected {
        return Err(StorageError::RetryRequired);
    }
    Ok(())
}
fn error(code: ErrorCode) -> StorageError {
    StorageError::Rejected(CommandError { code })
}
fn random_key() -> StorageResult<String> {
    let mut bytes = [0; 32];
    getrandom::fill(&mut bytes).map_err(|_| StorageError::Unavailable)?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
pub fn key_verifier(key: &str) -> StorageResult<[u8; 32]> {
    use sha2::{Digest, Sha256};
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(error(ErrorCode::AccessDenied));
    }
    let mut bytes = [0; 32];
    for (i, pair) in key.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        bytes[i] = u8::from_str_radix(
            std::str::from_utf8(pair).map_err(|_| error(ErrorCode::AccessDenied))?,
            16,
        )
        .map_err(|_| error(ErrorCode::AccessDenied))?;
    }
    let mut hash = Sha256::new();
    hash.update(b"zedone/maintenance-key/v1\0");
    hash.update(bytes);
    Ok(hash.finalize().into())
}
pub(crate) fn admit(path: &Path) -> StorageResult<()> {
    let directory = ownership::sibling(&ownership::database_path(path)?, ".maintenance");
    if !directory.exists() {
        return Ok(());
    }
    let state: State = read(&directory.join("state.json"))?;
    if state.version != 1 || state.blocked || !path.is_file() {
        return Err(StorageError::RetryRequired);
    }
    if let Some(epoch) = state.current_epoch {
        let namespace = path
            .parent()
            .ok_or(StorageError::Unavailable)?
            .join("host-namespace-v1");
        if backup_io::read(&namespace, 32)?.as_slice() != epoch {
            return Err(StorageError::RetryRequired);
        }
    }
    Ok(())
}
impl Emergency {
    pub fn open(path: PathBuf) -> StorageResult<Self> {
        let path = ownership::database_path(&path)?;
        let directory = ownership::sibling(&path, ".maintenance");
        let parent = path.parent().ok_or(StorageError::Unavailable)?;
        if !directory.exists() {
            let temporary = tempfile::Builder::new()
                .prefix("maintenance-bootstrap-")
                .tempdir_in(parent)
                .map_err(io)?;
            let owner = LocalSession {
                subject: backup_io::random_id()?,
                key: random_key()?,
            };
            let state = State {
                version: 1,
                identity: owner.subject.clone(),
                revision: 1,
                last_ms: 0,
                root: ownership::sibling(&path, ".backups")
                    .to_str()
                    .ok_or(StorageError::Unavailable)?
                    .into(),
                blocked: false,
                current_epoch: None,
                reason: None,
                plan: None,
                problems: Vec::new(),
                grants: Vec::new(),
                receipts: BTreeMap::new(),
                deleting: BTreeMap::new(),
            };
            save(&temporary.path().join("owner.json"), &owner)?;
            save(&temporary.path().join("state.json"), &state)?;
            backup_io::sync(temporary.path())?;
            match fs::rename(temporary.path(), &directory) {
                Ok(()) => {
                    let _ = temporary.keep();
                    backup_io::sync(parent)?;
                }
                Err(_) if directory.exists() => {}
                Err(error) => return Err(io(error)),
            }
        }
        backup_io::directory(&directory)?;
        let value = Self {
            path,
            directory,
            exclusive: None,
        };
        let _lock = value.control_lock()?;
        let owner = value.owner_session()?;
        if value.load()?.identity != owner.subject {
            return Err(StorageError::InvalidDatabase);
        }
        Ok(value)
    }
    fn control_lock(&self) -> StorageResult<ControlLock> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(self.directory.join("control.lock"))
            .map_err(io)?;
        file.try_lock().map_err(|_| StorageError::Unavailable)?;
        Ok(ControlLock(file))
    }
    fn load(&self) -> StorageResult<State> {
        let state: State = read(&self.directory.join("state.json"))?;
        if state.version != 1
            || state.revision == 0
            || state.identity != self.owner_session()?.subject
        {
            return Err(StorageError::InvalidDatabase);
        }
        Ok(state)
    }
    fn store(&self, state: &State) -> StorageResult<()> {
        save(&self.directory.join("state.json"), state)
    }
    pub fn owner_session(&self) -> StorageResult<LocalSession> {
        let session: LocalSession = read(&self.directory.join("owner.json"))?;
        if !valid_id(&session.subject) {
            return Err(StorageError::InvalidDatabase);
        }
        key_verifier(&session.key)?;
        Ok(session)
    }
    pub fn owner_access(&self, key: &str) -> StorageResult<()> {
        let _lock = self.control_lock()?;
        let mut state = self.load()?;
        if !self.authorize(&mut state, key)?.owner {
            return Err(error(ErrorCode::AccessDenied));
        }
        Ok(())
    }
    fn authorize(&self, state: &mut State, key: &str) -> StorageResult<Role> {
        use subtle::ConstantTimeEq;
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| StorageError::Unavailable)?
                .as_millis(),
        )
        .map_err(|_| StorageError::Unavailable)?;
        if now < state.last_ms {
            return Err(StorageError::Unavailable);
        }
        state.last_ms = now;
        self.store(state)?;
        let verifier = key_verifier(key)?;
        let owner = self.owner_session()?;
        if bool::from(key_verifier(&owner.key)?.ct_eq(&verifier)) {
            return Ok(Role {
                subject: owner.subject,
                owner: true,
                writable: true,
                expires_ms: None,
            });
        }
        state
            .grants
            .iter()
            .find(|g| {
                !g.info.revoked
                    && g.info.expires_ms.is_none_or(|ms| ms > now)
                    && bool::from(g.verifier.ct_eq(&verifier))
            })
            .map(|g| Role {
                subject: g.info.subject.clone(),
                owner: false,
                writable: g.info.writable,
                expires_ms: g.info.expires_ms,
            })
            .ok_or(error(ErrorCode::AccessDenied))
    }
    pub fn check(
        &self,
        key: &str,
        id: CommandId,
        command: &MaintenanceCommand,
    ) -> StorageResult<Option<Execution<MaintenanceView>>> {
        let _lock = self.control_lock()?;
        let mut state = self.load()?;
        let role = self.authorize(&mut state, key)?;
        let result = self.begin(&state, &role, id, command)?;
        self.store(&state)?;
        Ok(result)
    }
    fn begin(
        &self,
        state: &State,
        role: &Role,
        id: CommandId,
        command: &MaintenanceCommand,
    ) -> StorageResult<Option<Execution<MaintenanceView>>> {
        if id == [0; 16] {
            return Err(error(ErrorCode::InvalidRequest));
        }
        if !matches!(command, MaintenanceCommand::Inspect {}) && !role.writable {
            return Err(error(ErrorCode::AccessDenied));
        }
        if matches!(
            command,
            MaintenanceCommand::IssueAccess { .. }
                | MaintenanceCommand::RevokeAccess { .. }
                | MaintenanceCommand::SelectFolder { .. }
                | MaintenanceCommand::Continue { .. }
        ) && !role.owner
        {
            return Err(error(ErrorCode::AccessDenied));
        }
        let key = receipt_key(role, id);
        let request = zedone_core::replay::canonical_json(command)?;
        if let Some(receipt) = state.receipts.get(&key) {
            if receipt.request != request {
                return Err(error(ErrorCode::CommandMismatch));
            }
            return Ok(Some(Execution {
                result: receipt.result.clone(),
                replayed: true,
            }));
        }
        if let Some(expected) = command.expected() {
            let continuing = state.plan.as_ref().is_some_and(|p| {
                p.apply
                    .iter()
                    .chain(p.rollback.iter())
                    .any(|(k, r)| *k == key && *r == request)
            }) || state
                .deleting
                .values()
                .any(|d| d.bindings.iter().any(|(k, r)| *k == key && *r == request));
            if !continuing {
                zedone_core::replay::require_current(expected, version(state)?)?;
            }
        }
        Ok(None)
    }
    fn refresh_role(&self, state: &mut State, role: &Role) -> StorageResult<()> {
        let now = u64::try_from(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|_| StorageError::Unavailable)?
                .as_millis(),
        )
        .map_err(|_| StorageError::Unavailable)?;
        if now < state.last_ms {
            return Err(StorageError::Unavailable);
        }
        let mut durable = self.load()?;
        if now < durable.last_ms {
            return Err(StorageError::Unavailable);
        }
        durable.last_ms = now;
        self.store(&durable)?;
        state.last_ms = now;
        if role.expires_ms.is_some_and(|ms| ms <= now) {
            return Err(error(ErrorCode::AccessDenied));
        }
        Ok(())
    }
    pub fn activate_failed(&mut self, reason: String) -> StorageResult<()> {
        let _lock = self.control_lock()?;
        let mut state = self.load()?;
        if self.acquire().is_err() {
            state.reason=Some("База недоступна для служебной операции. Закройте другие процессы ZeDone и проверьте доступность файлов.".into());
            return self.store(&state);
        }
        if reason != "RetryRequired" || !state.blocked || state.reason.is_none() {
            state.reason = Some(reason);
        }
        state.blocked = true;
        self.store(&state)?;
        if self.ensure_problem(&mut state).is_err() {
            state.reason=Some("Не удалось сохранить проблемное состояние. Проверьте свободное место и доступность файлов; перед восстановлением сохранение будет проверено снова.".into());
        }
        self.store(&state)
    }
    fn acquire(&mut self) -> StorageResult<()> {
        if self.exclusive.is_none() {
            self.exclusive = Some(Ownership::exclusive(&self.path)?);
        }
        Ok(())
    }
    fn ensure_problem(&self, state: &mut State) -> StorageResult<String> {
        if let Some(name) = state.problems.last()
            && valid_problem(name)
            && self.problem(name).is_ok()
            && crate::problem_state::inventory(&self.path, RAW_LIMIT)?
                == crate::problem_state::inventory(
                    &self.directory.join(name).join("database.sqlite"),
                    RAW_LIMIT,
                )?
        {
            return Ok(name.clone());
        }
        let problem = match fs::symlink_metadata(&self.path) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                crate::problem_state::preserve_absent(&self.path, &self.directory, RAW_LIMIT)?
            }
            _ => crate::problem_state::preserve(&self.path, &self.directory, RAW_LIMIT)?,
        };
        let name = problem
            .path()
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or(StorageError::Unavailable)?
            .to_owned();
        state.problems.push(name.clone());
        Ok(name)
    }
    fn view(&self, state: &State, role: &Role, ready: bool) -> StorageResult<MaintenanceView> {
        let copies = files::catalog(Path::new(&state.root));
        Ok(MaintenanceView {
            version: version(state)?,
            normal_ready: ready,
            reason: state.reason.clone(),
            owner: role.owner,
            writable: role.writable,
            folder: backup_io::label(Path::new(&state.root)).ok(),
            folder_available: copies.is_ok(),
            copies: copies.unwrap_or_default(),
            plan: state.plan.as_ref().map(|p| p.public.clone()),
            problems: state.problems.clone(),
            access: if role.owner {
                state.grants.iter().map(|g| g.info.clone()).collect()
            } else {
                Vec::new()
            },
            issued_subject: None,
            endpoint: None,
        })
    }
    pub fn execute(
        &mut self,
        key: &str,
        id: CommandId,
        command: MaintenanceCommand,
        ready: bool,
    ) -> StorageResult<Execution<MaintenanceView>> {
        let _lock = self.control_lock()?;
        let mut state = self.load()?;
        let role = self.authorize(&mut state, key)?;
        if let Some(result) = self.begin(&state, &role, id, &command)? {
            return Ok(result);
        }
        let mut issued = None;
        let mut extra_receipts = Vec::new();
        match &command {
            MaintenanceCommand::Inspect {} => {}
            MaintenanceCommand::Enter { .. } => {
                self.acquire()?;
                state.blocked = true;
                state.reason = Some("Обычная работа остановлена явно.".into());
                self.store(&state)?;
                self.ensure_problem(&mut state)?;
            }
            MaintenanceCommand::SelectFolder { root, .. } => {
                if root.len() > 32768 {
                    return Err(error(ErrorCode::InvalidRequest));
                }
                backup_io::directory(Path::new(root))?;
                state.root = root.clone();
            }
            MaintenanceCommand::Prepare { copy, .. } => {
                if ready {
                    return Err(error(ErrorCode::AccessDenied));
                }
                self.acquire()?;
                if !state.blocked {
                    state.blocked = true;
                    self.store(&state)?;
                }
                self.prepare(&mut state, copy, id)?;
            }
            MaintenanceCommand::Apply { plan, .. } => {
                if ready {
                    return Err(error(ErrorCode::AccessDenied));
                }
                self.acquire()?;
                self.apply(&mut state, plan, &role, id, &command)?;
            }
            MaintenanceCommand::Continue { plan, .. } => {
                if ready {
                    return Err(error(ErrorCode::AccessDenied));
                }
                self.acquire()?;
                self.continue_interrupted(&mut state, plan, &role, id, &command)?;
            }
            MaintenanceCommand::Cancel { plan, .. } => {
                self.cancel(&mut state, plan)?;
            }
            MaintenanceCommand::Rollback { plan, .. } => {
                if ready {
                    return Err(error(ErrorCode::AccessDenied));
                }
                self.acquire()?;
                self.rollback(&mut state, plan, &role, id, &command)?;
            }
            MaintenanceCommand::Retry { .. } => {
                if state.blocked {
                    return Err(StorageError::RetryRequired);
                }
                if ready {
                    state.reason = None;
                }
            }
            MaintenanceCommand::DeleteProblem { problem, .. } => {
                if !state.problems.contains(problem) || !valid_problem(problem) {
                    return Err(error(ErrorCode::NotFound));
                }
                if state.plan.as_ref().is_some_and(|p| {
                    p.public.problem == *problem
                        && matches!(
                            p.public.phase,
                            RecoveryPhase::Prepared
                                | RecoveryPhase::Applying
                                | RecoveryPhase::Reverting
                        )
                }) {
                    return Err(error(ErrorCode::AccessDenied));
                }
                let binding = (
                    receipt_key(&role, id),
                    zedone_core::replay::canonical_json(&command)?,
                );
                let mut deletion = match state.deleting.get(problem) {
                    Some(value) => {
                        if !role.owner && !value.bindings.contains(&binding) {
                            return Err(error(ErrorCode::AccessDenied));
                        }
                        value.clone()
                    }
                    None => Deletion {
                        manifest: self.problem(problem)?.deletion_record()?,
                        bindings: Vec::new(),
                    },
                };
                if !deletion.bindings.contains(&binding) {
                    deletion.bindings.push(binding);
                }
                state.deleting.insert(problem.clone(), deletion.clone());
                self.store(&state)?;
                crate::problem_state::discard_remaining(
                    &self.directory.join(problem),
                    &deletion.manifest,
                    || self.refresh_role(&mut state, &role),
                )?;
                extra_receipts.extend(deletion.bindings);
                state.deleting.remove(problem);
                state.problems.retain(|p| p != problem);
            }
            MaintenanceCommand::IssueAccess {
                label,
                verifier,
                writable,
                expires_ms,
                ..
            } => {
                if label.trim().is_empty()
                    || label.chars().count() > 80
                    || *verifier == [0; 32]
                    || expires_ms.is_some_and(|ms| ms <= state.last_ms)
                    || state.grants.iter().any(|g| g.verifier == *verifier)
                {
                    return Err(error(ErrorCode::InvalidRequest));
                }
                let subject = backup_io::random_id()?;
                issued = Some(subject.clone());
                state.grants.push(Grant {
                    info: MaintenanceAccess {
                        subject,
                        label: label.clone(),
                        writable: *writable,
                        expires_ms: *expires_ms,
                        revoked: false,
                    },
                    verifier: *verifier,
                });
            }
            MaintenanceCommand::RevokeAccess { subject, .. } => {
                let grant = state
                    .grants
                    .iter_mut()
                    .find(|g| g.info.subject == *subject)
                    .ok_or(error(ErrorCode::NotFound))?;
                grant.info.revoked = true;
            }
        }
        if !matches!(command, MaintenanceCommand::Inspect {}) {
            state.revision = state
                .revision
                .checked_add(1)
                .ok_or(StorageError::Unavailable)?;
        }
        self.refresh_role(&mut state, &role)?;
        let mut result = self.view(&state, &role, ready)?;
        self.refresh_role(&mut state, &role)?;
        result.issued_subject = issued;
        for (key, request) in extra_receipts {
            let subject = key
                .get(..32)
                .ok_or(StorageError::InvalidDatabase)?
                .to_owned();
            let original = Role {
                owner: subject == self.owner_session()?.subject,
                subject,
                writable: true,
                expires_ms: None,
            };
            state.receipts.insert(
                key,
                Receipt {
                    request,
                    result: self.view(&state, &original, ready)?,
                },
            );
        }
        if state.plan.as_ref().is_some_and(|p| {
            matches!(
                p.public.phase,
                RecoveryPhase::Restored | RecoveryPhase::Reverted
            )
        }) {
            let superseded = state
                .plan
                .as_mut()
                .map(|p| std::mem::take(&mut p.superseded))
                .unwrap_or_default();
            let owner = self.owner_session()?.subject;
            for (key, request) in superseded {
                let subject = key
                    .get(..32)
                    .ok_or(StorageError::InvalidDatabase)?
                    .to_owned();
                let original = Role {
                    owner: subject == owner,
                    subject,
                    writable: true,
                    expires_ms: None,
                };
                let outcome = self.view(&state, &original, false)?;
                state.receipts.insert(
                    key,
                    Receipt {
                        request,
                        result: outcome,
                    },
                );
            }
        }
        if !matches!(command, MaintenanceCommand::Inspect {}) {
            state.receipts.insert(
                receipt_key(&role, id),
                Receipt {
                    request: zedone_core::replay::canonical_json(&command)?,
                    result: result.clone(),
                },
            );
        }
        self.store(&state)?;
        Ok(Execution {
            result,
            replayed: false,
        })
    }
    pub fn release_for_retry(
        &mut self,
        key: &str,
        id: CommandId,
        command: &MaintenanceCommand,
    ) -> StorageResult<()> {
        if !matches!(command, MaintenanceCommand::Retry { .. }) {
            return Err(error(ErrorCode::InvalidRequest));
        }
        let _lock = self.control_lock()?;
        let mut state = self.load()?;
        let role = self.authorize(&mut state, key)?;
        self.begin(&state, &role, id, command)?;
        if state
            .plan
            .as_ref()
            .is_some_and(|p| p.public.phase == RecoveryPhase::Reverted)
        {
            self.acquire()?;
            self.validate_reverted(&mut state, &role, id, command)?;
        }
        if state.plan.as_ref().is_some_and(|p| {
            matches!(
                p.public.phase,
                RecoveryPhase::Prepared | RecoveryPhase::Applying | RecoveryPhase::Reverting
            )
        }) {
            return Err(StorageError::RetryRequired);
        }
        if let Some(epoch) = state.current_epoch {
            self.publish_namespace(&epoch)?;
        }
        state.blocked = false;
        self.store(&state)?;
        self.exclusive = None;
        Ok(())
    }
}
struct ControlLock(std::fs::File);
impl Drop for ControlLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}
fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        && id != "00000000000000000000000000000000"
}
fn valid_problem(id: &str) -> bool {
    id.starts_with("problem-state-") && backup_io::valid_name(id)
}
fn receipt_key(role: &Role, id: CommandId) -> String {
    format!(
        "{}-{}",
        role.subject,
        id.iter().map(|b| format!("{b:02x}")).collect::<String>()
    )
}
fn version(state: &State) -> StorageResult<StateVersion> {
    Ok(crate::state_version(
        &("maintenance.v1", &state.identity),
        &state.revision,
    )?)
}
