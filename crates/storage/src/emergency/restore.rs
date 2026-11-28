use super::*;
use files::{LIMITS, copy, hash, replace};
use rusqlite::Connection;

const SUFFIXES: [&str; 4] = ["", "-wal", "-shm", "-journal"];
impl Emergency {
    pub(super) fn problem(&self, name: &str) -> StorageResult<crate::problem_state::ProblemState> {
        if !valid_problem(name) {
            return Err(StorageError::InvalidDatabase);
        }
        crate::problem_state::ProblemState::open(self.directory.join(name), RAW_LIMIT)
    }
    fn plan_directory(&self, id: &str) -> StorageResult<PathBuf> {
        if !id.starts_with("recovery-plan-") || !backup_io::valid_name(id) {
            return Err(StorageError::InvalidDatabase);
        }
        let path = self.directory.join(id);
        if path.exists() {
            backup_io::directory(&path)?;
        }
        Ok(path)
    }
    pub(super) fn prepare(
        &mut self,
        state: &mut State,
        name: &str,
        _id: CommandId,
    ) -> StorageResult<()> {
        if !state.blocked
            || !backup_io::valid_name(name)
            || !name.starts_with("zedone-migration-candidate-")
        {
            return Err(error(ErrorCode::InvalidRequest));
        }
        if state.plan.as_ref().is_some_and(|p| {
            matches!(
                p.public.phase,
                RecoveryPhase::Prepared | RecoveryPhase::Applying | RecoveryPhase::Reverting
            )
        }) {
            return Err(StorageError::RetryRequired);
        }
        let problem = self.ensure_problem(state)?;
        self.problem(&problem)?.verify()?;
        let directory = tempfile::Builder::new()
            .prefix("recovery-plan-")
            .tempdir_in(&self.directory)
            .map_err(io)?;
        let selected = files::copy_bundle(
            &Path::new(&state.root).join(name),
            &directory.path().join("input"),
        )?;
        let candidate = directory.path().join("candidate.sqlite");
        copy(
            &directory.path().join("input/database.sqlite"),
            &candidate,
            LIMITS.max_database_bytes,
        )?;
        let (local, namespace) = sanitize(&candidate)?;
        state.plan = Some(Plan {
            public: RecoveryPlan {
                id: directory
                    .path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or(StorageError::Unavailable)?
                    .into(),
                copy: name.into(),
                spaces: selected.spaces,
                created_at_unix_ms: selected.created_at_unix_ms,
                phase: RecoveryPhase::Prepared,
                problem,
            },
            candidate: hash(&candidate)?,
            local,
            namespace,
            apply: None,
            rollback: None,
            rollback_source: None,
            superseded: Vec::new(),
        });
        backup_io::sync(directory.path())?;
        let _ = directory.keep();
        Ok(())
    }
    pub(super) fn apply(
        &mut self,
        state: &mut State,
        id: &str,
        role: &Role,
        command_id: CommandId,
        command: &MaintenanceCommand,
    ) -> StorageResult<()> {
        let mut plan = state
            .plan
            .clone()
            .filter(|p| p.public.id == id)
            .ok_or(error(ErrorCode::NotFound))?;
        let key = receipt_key(role, command_id);
        let request = zedone_core::replay::canonical_json(command)?;
        let resumed = plan.public.phase == RecoveryPhase::Applying;
        if resumed {
            if plan.apply.as_ref() != Some(&(key.clone(), request.clone())) {
                return Err(error(ErrorCode::CommandMismatch));
            }
        } else if plan.public.phase != RecoveryPhase::Prepared {
            return Err(StorageError::RetryRequired);
        }
        self.problem(&plan.public.problem)?.verify()?;
        let candidate = self
            .plan_directory(&plan.public.id)?
            .join("candidate.sqlite");
        if hash(&candidate)? != plan.candidate {
            return Err(StorageError::InvalidDatabase);
        }
        schema::REGISTRY.target(&migration_backup::readonly(&candidate)?)?;
        let old = crate::problem_state::inventory(
            &self
                .directory
                .join(&plan.public.problem)
                .join("database.sqlite"),
            RAW_LIMIT,
        )?;
        let target = [Some(plan.candidate.clone()), None, None, None];
        transition(&self.path, &old, &target, resumed)?;
        plan.public.phase = RecoveryPhase::Applying;
        plan.apply = Some((key, request));
        state.plan = Some(plan.clone());
        state.blocked = true;
        self.store(state)?;
        checkpoint("apply_intent")?;
        self.refresh_role(state, role)?;
        for suffix in &SUFFIXES[1..] {
            backup_io::remove_file(&ownership::sibling(&self.path, suffix))?;
        }
        checkpoint("sidecars_removed")?;
        files::replace_checked(&candidate, &self.path, || self.refresh_role(state, role))?;
        checkpoint("database_replaced")?;
        self.publish_identity(&plan)?;
        checkpoint("identity_published")?;
        self.refresh_role(state, role)?;
        plan.public.phase = RecoveryPhase::Restored;
        state.current_epoch = Some(plan.namespace);
        state.plan = Some(plan);
        state.blocked = false;
        state.reason=Some("База восстановлена. Прежние ключи отключены, обмен приостановлен. Откройте пространства явно.".into());
        Ok(())
    }
    fn publish_identity(&self, plan: &Plan) -> StorageResult<()> {
        let parent = self.path.parent().ok_or(StorageError::Unavailable)?;
        save(&parent.join("local-session.json"), &plan.local)?;
        self.publish_namespace(&plan.namespace)
    }
    pub(super) fn publish_namespace(&self, namespace: &[u8; 32]) -> StorageResult<()> {
        let parent = self.path.parent().ok_or(StorageError::Unavailable)?;
        let mut file = tempfile::NamedTempFile::new_in(parent).map_err(io)?;
        use std::io::Write;
        file.write_all(namespace).map_err(io)?;
        file.as_file().sync_all().map_err(io)?;
        file.persist(parent.join("host-namespace-v1"))
            .map_err(|_| StorageError::OutcomeUnknown)?;
        backup_io::sync(parent)
    }
    pub(super) fn continue_interrupted(
        &mut self,
        state: &mut State,
        id: &str,
        role: &Role,
        command_id: CommandId,
        command: &MaintenanceCommand,
    ) -> StorageResult<()> {
        let mut plan = state
            .plan
            .clone()
            .filter(|p| p.public.id == id)
            .ok_or(error(ErrorCode::NotFound))?;
        let new = (
            receipt_key(role, command_id),
            zedone_core::replay::canonical_json(command)?,
        );
        let binding = match plan.public.phase {
            RecoveryPhase::Applying => &mut plan.apply,
            RecoveryPhase::Reverting => &mut plan.rollback,
            _ => return Err(StorageError::RetryRequired),
        };
        let prior = binding
            .replace(new.clone())
            .ok_or(StorageError::InvalidDatabase)?;
        if prior != new {
            plan.superseded.push(prior);
        }
        let phase = plan.public.phase.clone();
        state.plan = Some(plan);
        self.store(state)?;
        if phase == RecoveryPhase::Applying {
            self.apply(state, id, role, command_id, command)
        } else {
            self.rollback(state, id, role, command_id, command)
        }
    }
    pub(super) fn rollback(
        &mut self,
        state: &mut State,
        id: &str,
        role: &Role,
        command_id: CommandId,
        command: &MaintenanceCommand,
    ) -> StorageResult<()> {
        let mut plan = state
            .plan
            .clone()
            .filter(|p| p.public.id == id)
            .ok_or(error(ErrorCode::NotFound))?;
        let key = receipt_key(role, command_id);
        let request = zedone_core::replay::canonical_json(command)?;
        let resumed = plan.public.phase == RecoveryPhase::Reverting;
        if resumed {
            if plan.rollback.as_ref() != Some(&(key.clone(), request.clone())) {
                return Err(error(ErrorCode::CommandMismatch));
            }
        } else if plan.public.phase != RecoveryPhase::Restored {
            return Err(StorageError::RetryRequired);
        }
        self.problem(&plan.public.problem)?.verify()?;
        if !resumed {
            let source = self.ensure_problem(state)?;
            plan.rollback_source = Some(source);
        }
        let source = plan
            .rollback_source
            .as_ref()
            .ok_or(StorageError::InvalidDatabase)?;
        self.problem(source)?.verify()?;
        let original = crate::problem_state::inventory(
            &self.directory.join(source).join("database.sqlite"),
            RAW_LIMIT,
        )?;
        let goal = self
            .directory
            .join(&plan.public.problem)
            .join("database.sqlite");
        let target = crate::problem_state::inventory(&goal, RAW_LIMIT)?;
        transition(&self.path, &original, &target, resumed)?;
        plan.public.phase = RecoveryPhase::Reverting;
        plan.rollback = Some((key, request));
        state.plan = Some(plan.clone());
        state.blocked = true;
        self.store(state)?;
        checkpoint("rollback_intent")?;
        self.refresh_role(state, role)?;
        for (index, suffix) in SUFFIXES.iter().enumerate() {
            let destination = ownership::sibling(&self.path, suffix);
            if target[index].is_some() {
                files::replace_checked(&ownership::sibling(&goal, suffix), &destination, || {
                    self.refresh_role(state, role)
                })?;
            } else {
                backup_io::remove_file(&destination)?;
            }
            checkpoint("rollback_file")?;
        }
        plan.public.phase = RecoveryPhase::Reverted;
        state.plan = Some(plan);
        state.reason=Some("Возвращено сохранённое проблемное состояние. Обычная работа и обмен остановлены; повтор запуска выполняется только явно.".into());
        Ok(())
    }
    pub(super) fn cancel(&self, state: &mut State, id: &str) -> StorageResult<()> {
        let plan = state
            .plan
            .as_mut()
            .filter(|p| p.public.id == id)
            .ok_or(error(ErrorCode::NotFound))?;
        if plan.public.phase != RecoveryPhase::Prepared {
            return Err(StorageError::RetryRequired);
        }
        let directory = self.plan_directory(id)?;
        if !directory.exists() {
            plan.public.phase = RecoveryPhase::Cancelled;
            return Ok(());
        }
        let input = directory.join("input");
        if input.exists() {
            let entries = backup_io::list(&input)?;
            if entries.iter().any(|e| {
                e.directory
                    || !(e.name == "manifest.json"
                        || e.name == "database.sqlite"
                        || (e.name.starts_with("space-")
                            && e.name.ends_with(".bin")
                            && backup_io::valid_name(&e.name)))
            }) {
                return Err(StorageError::InvalidDatabase);
            }
            for entry in entries {
                backup_io::remove_file(&input.join(entry.name))?;
            }
            backup_io::remove_dir(&input)?;
        }
        backup_io::remove_file(&directory.join("candidate.sqlite"))?;
        backup_io::remove_dir(&directory)?;
        plan.public.phase = RecoveryPhase::Cancelled;
        Ok(())
    }
    pub(super) fn validate_reverted(
        &mut self,
        state: &mut State,
        role: &Role,
        id: CommandId,
        command: &MaintenanceCommand,
    ) -> StorageResult<()> {
        let mut plan = state.plan.clone().ok_or(StorageError::InvalidDatabase)?;
        let temp = tempfile::Builder::new()
            .prefix("retry-image-")
            .tempdir_in(&self.directory)
            .map_err(io)?;
        let staged = temp.path().join("database.sqlite");
        let before = crate::problem_state::inventory(&self.path, RAW_LIMIT)?;
        for (index, suffix) in SUFFIXES.iter().enumerate() {
            if before[index].is_some() {
                copy(
                    &ownership::sibling(&self.path, suffix),
                    &ownership::sibling(&staged, suffix),
                    RAW_LIMIT,
                )?;
            }
        }
        let (local, namespace) = sanitize(&staged)?;
        if crate::problem_state::inventory(&self.path, RAW_LIMIT)? != before {
            return Err(error(ErrorCode::StaleState));
        }
        let candidate = self
            .plan_directory(&plan.public.id)?
            .join("candidate.sqlite");
        replace(&staged, &candidate)?;
        plan.local = local;
        plan.namespace = namespace;
        plan.candidate = hash(&candidate)?;
        plan.public.problem = self.ensure_problem(state)?;
        plan.public.phase = RecoveryPhase::Prepared;
        plan.apply = None;
        plan.rollback = None;
        plan.rollback_source = None;
        let plan_id = plan.public.id.clone();
        state.plan = Some(plan);
        self.apply(state, &plan_id, role, id, command)?;
        self.store(state)
    }
}
fn transition(
    path: &Path,
    old: &[Option<crate::problem_state::Entry>; 4],
    new: &[Option<crate::problem_state::Entry>; 4],
    resumed: bool,
) -> StorageResult<()> {
    let actual = crate::problem_state::inventory(path, RAW_LIMIT)?;
    if !resumed && actual != *old {
        return Err(error(ErrorCode::StaleState));
    }
    if resumed
        && actual
            .iter()
            .enumerate()
            .any(|(i, e)| *e != old[i] && *e != new[i] && e.is_some())
    {
        return Err(error(ErrorCode::StaleState));
    }
    Ok(())
}
fn sanitize(path: &Path) -> StorageResult<(LocalSession, [u8; 32])> {
    let mut namespace = [0; 32];
    getrandom::fill(&mut namespace).map_err(|_| StorageError::Unavailable)?;
    let mut connection = Connection::open(path)?;
    connection.pragma_update(None, "foreign_keys", true)?;
    let tx = connection.transaction()?;
    schema::REGISTRY.migrate(&tx)?;
    crate::backup_catalog::detach_after_recovery(&tx)?;
    crate::daily_backup::detach_after_recovery(&tx)?;
    tx.execute_batch("UPDATE access_keys SET active=0; UPDATE access_principals SET revoked=1; DELETE FROM pairing_requests; DELETE FROM pairing_invitations; DELETE FROM sync_incoming; DELETE FROM sync_selections; DELETE FROM peers; INSERT INTO sync_spaces(space,paused,revision) SELECT space,1,1 FROM documents WHERE true ON CONFLICT(space) DO UPDATE SET paused=1,revision=revision+1; UPDATE access_clock SET calendar_ms=0,effective_ms=0;")?;
    let spaces: Vec<String> = tx
        .prepare("SELECT space FROM documents")?
        .query_map([], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    for space in spaces {
        let mut actor = [0; 16];
        getrandom::fill(&mut actor).map_err(|_| StorageError::Unavailable)?;
        tx.execute(
            "UPDATE documents SET actor=?1 WHERE space=?2",
            rusqlite::params![actor.as_slice(), space],
        )?;
    }
    let local = LocalSession {
        subject: backup_io::random_id()?,
        key: random_key()?,
    };
    let mut bytes = [0; 32];
    for (i, pair) in local.key.as_bytes().as_chunks::<2>().0.iter().enumerate() {
        bytes[i] = u8::from_str_radix(
            std::str::from_utf8(pair).map_err(|_| StorageError::Unavailable)?,
            16,
        )
        .map_err(|_| StorageError::Unavailable)?;
    }
    tx.execute(
        "INSERT INTO access_principals VALUES (?1,1,NULL,0)",
        [&local.subject],
    )?;
    tx.execute(
        "INSERT INTO access_spaces SELECT ?1,space,1 FROM documents",
        [&local.subject],
    )?;
    tx.execute(
        "INSERT INTO access_keys VALUES (?1,?2,1)",
        rusqlite::params![crate::access::verifier(&bytes).as_slice(), local.subject],
    )?;
    tx.execute("INSERT INTO access_bootstrap VALUES (1,?1) ON CONFLICT(singleton) DO UPDATE SET subject=excluded.subject",[&local.subject])?;
    tx.execute(
        "DELETE FROM command_results WHERE key=?1",
        [b"\0zedone/internal/migration-backup/v1".as_slice()],
    )?;
    tx.execute("INSERT INTO command_results(key,request,result) VALUES (?1,?2,?3) ON CONFLICT(key) DO UPDATE SET request=excluded.request,result=excluded.result",rusqlite::params![super::EPOCH_KEY,b"recovery-epoch-v1".as_slice(),serde_json::to_vec(&namespace).map_err(|_|StorageError::Unavailable)?])?;
    tx.commit()?;
    connection.pragma_update(None, "journal_mode", "DELETE")?;
    schema::REGISTRY.target(&connection)?;
    connection.close().map_err(|_| StorageError::Unavailable)?;
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .map_err(io)?
        .sync_all()
        .map_err(io)?;
    Ok((local, namespace))
}
fn checkpoint(_name: &str) -> StorageResult<()> {
    #[cfg(test)]
    if std::env::var("ZEDONE_EMERGENCY_CRASH").as_deref() == Ok(_name) {
        std::process::exit(74);
    }
    Ok(())
}
