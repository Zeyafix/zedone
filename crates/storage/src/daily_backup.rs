use crate::{Storage, StorageError, StorageResult, WriteContext, snapshot::SnapshotLimits};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeSet, HashMap},
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use zedone_core::{
    backups::{
        DailyBackupStore, DailyBackups, DailyCopy, DailyStatus, DeleteDailyBackup, ReadDailyBackup,
        RetryDailyBackup,
    },
    command::CommandResult,
    daily_backup::{Admission, DailyBackupPolicy, Moment, RETRY_DELAY_MS, Trigger},
    error::{CommandError, ErrorCode},
    replay::StateVersion,
    snapshots::{MAX_ARCHIVE_BYTES, MAX_ARCHIVE_CHANGES, SpaceArchive},
};
mod files;
#[cfg(test)]
mod tests;
const GLOBAL: &[u8] = b"\0zedone/internal/daily-backups/global/v1";
const PREFIX: &str = "\0zedone/internal/daily-backups/space/v1/";
const REQUEST: &[u8] = b"daily-backups.v1";
const LIMITS: SnapshotLimits = SnapshotLimits {
    max_bytes: MAX_ARCHIVE_BYTES,
    max_changes: MAX_ARCHIVE_CHANGES,
};
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Global {
    installation: String,
    folder_initialized: bool,
    elapsed_ms: u64,
    wall_ms: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Reference {
    id: String,
    space: String,
    created_at_unix_ms: u64,
    bytes: usize,
    digest: [u8; 32],
}
impl Reference {
    fn info(&self) -> DailyCopy {
        DailyCopy {
            id: self.id.clone(),
            created_at_unix_ms: self.created_at_unix_ms,
        }
    }
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct State {
    space: String,
    policy: DailyBackupPolicy,
    files: Vec<Reference>,
    obsolete: Vec<Reference>,
    #[serde(deserialize_with = "required_option")]
    last_success: Option<Reference>,
}
fn required_option<'de, D: serde::Deserializer<'de>, T: Deserialize<'de>>(
    d: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(d)
}
fn invalid() -> StorageError {
    StorageError::InvalidDatabase
}
fn unavailable() -> StorageError {
    StorageError::Unavailable
}
fn state_key(space: &str) -> Vec<u8> {
    format!("{PREFIX}{space}").into_bytes()
}
fn load<T: DeserializeOwned>(c: &Connection, key: &[u8]) -> StorageResult<Option<T>> {
    let row: Option<(Vec<u8>,Vec<u8>)> = c.query_row("SELECT substr(request,1,64), substr(result,1,262145) FROM command_results WHERE key=?1", [key], |r| Ok((r.get(0)?,r.get(1)?))).optional()?;
    row.map(|(request, bytes)| {
        if request != REQUEST || bytes.len() > 262144 {
            return Err(invalid());
        }
        serde_json::from_slice(&bytes).map_err(|_| invalid())
    })
    .transpose()
}
fn save(c: &Connection, key: &[u8], state: &impl Serialize) -> StorageResult<()> {
    let bytes = serde_json::to_vec(state).map_err(|_| invalid())?;
    if bytes.len() > 262144 {
        return Err(unavailable());
    }
    c.execute("INSERT INTO command_results VALUES (?1,?2,?3) ON CONFLICT(key) DO UPDATE SET request=excluded.request,result=excluded.result", (key, REQUEST, bytes))?;
    Ok(())
}
fn global(c: &Connection) -> StorageResult<Global> {
    let g: Global = load(c, GLOBAL)?.ok_or_else(unavailable)?;
    if !crate::columns::valid_id(&g.installation) {
        return Err(invalid());
    }
    Ok(g)
}
fn state(c: &Connection, space: &str) -> StorageResult<State> {
    let s: State = load(c, &state_key(space))?.unwrap_or_else(|| State {
        space: space.into(),
        policy: DailyBackupPolicy::default(),
        files: vec![],
        obsolete: vec![],
        last_success: None,
    });
    if s.space != space
        || s.files.len() > 7
        || s.obsolete.len() > 64
        || s.policy.last_success().is_some() != s.last_success.is_some()
    {
        return Err(invalid());
    }
    let mut ids = BTreeSet::new();
    for r in s
        .files
        .iter()
        .chain(&s.obsolete)
        .chain(s.last_success.iter())
    {
        if r.space != space || r.bytes == 0 || r.bytes > MAX_ARCHIVE_BYTES {
            return Err(invalid());
        }
    }
    for r in s.files.iter().chain(&s.obsolete) {
        if !ids.insert(&r.id) {
            return Err(invalid());
        }
    }
    Ok(s)
}
pub fn local_day() -> StorageResult<i32> {
    let name = iana_time_zone::get_timezone().map_err(|_| unavailable())?;
    let zone = jiff::tz::TimeZone::get(&name).map_err(|_| unavailable())?;
    calendar_day(jiff::Timestamp::now(), zone)
}
fn calendar_day(time: jiff::Timestamp, zone: jiff::tz::TimeZone) -> StorageResult<i32> {
    let date = time.to_zoned(zone).date();
    let days = date
        .since(jiff::civil::date(1970, 1, 1))
        .map_err(|_| unavailable())?
        .get_days();
    Ok(days)
}
type Anchors = Mutex<HashMap<String, (Instant, u64)>>;
static ANCHORS: OnceLock<Anchors> = OnceLock::new();
fn sample(g: &mut Global) -> StorageResult<(Moment, u64)> {
    let wall = u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| unavailable())?
            .as_millis(),
    )
    .map_err(|_| unavailable())?;
    observed(g, wall, local_day()?, Instant::now())
}
fn observed(g: &mut Global, wall: u64, day: i32, now: Instant) -> StorageResult<(Moment, u64)> {
    let mut anchors = ANCHORS
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| unavailable())?;
    let anchor = anchors.entry(g.installation.clone()).or_insert_with(|| {
        (
            now,
            g.elapsed_ms.saturating_add(wall.saturating_sub(g.wall_ms)),
        )
    });
    let elapsed = u64::try_from(
        now.checked_duration_since(anchor.0)
            .ok_or_else(unavailable)?
            .as_millis(),
    )
    .map_err(|_| unavailable())?;
    let mut logical = anchor.1.checked_add(elapsed).ok_or_else(unavailable)?;
    if logical < g.elapsed_ms {
        logical = g.elapsed_ms;
        *anchor = (now, logical);
    }
    g.elapsed_ms = logical;
    g.wall_ms = wall;
    Ok((
        Moment {
            local_day: day,
            elapsed_ms: logical,
        },
        wall,
    ))
}
fn revision(c: &Connection, space: &str) -> StorageResult<StateVersion> {
    let exists: bool = c.query_row(
        "SELECT EXISTS(SELECT 1 FROM documents WHERE space=?1)",
        [space],
        |r| r.get(0),
    )?;
    if !exists {
        return Err(CommandError {
            code: ErrorCode::NotFound,
        }
        .into());
    }
    let mut hash = Sha256::new();
    hash.update(b"zedone.daily.revision.v1");
    let mut query = c.prepare("SELECT hash FROM changes WHERE space=?1 ORDER BY hash")?;
    for row in query.query_map([space], |r| r.get::<_, Vec<u8>>(0))? {
        let bytes = row?;
        if bytes.len() != 32 {
            return Err(invalid());
        }
        hash.update(bytes);
    }
    let mut query = c.prepare("SELECT card,pending_id,title,description FROM card_editing WHERE space=?1 AND pending_id IS NOT NULL ORDER BY card")?;
    for row in query.query_map([space], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })? {
        let bytes = serde_json::to_vec(&row?).map_err(|_| invalid())?;
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    Ok(StateVersion(hash.finalize().into()))
}
fn view(s: &State, now: Moment, status: DailyStatus) -> DailyBackups {
    let failure = s.policy.last_failure();
    DailyBackups {
        space: s.space.clone(),
        status,
        last_success: s.last_success.as_ref().map(Reference::info),
        failed_sequence: failure.map(|f| f.sequence),
        retry_after_ms: failure
            .map(|f| RETRY_DELAY_MS.saturating_sub(now.elapsed_ms.saturating_sub(f.finished_ms))),
        copies: s.files.iter().rev().map(Reference::info).collect(),
        cleanup_pending: !s.obsolete.is_empty(),
    }
}
fn status(admission: Admission) -> DailyStatus {
    match admission {
        Admission::Start(_) => DailyStatus::Ready,
        Admission::Running(_) => DailyStatus::Running,
        Admission::Unchanged => DailyStatus::Unchanged,
        Admission::DailyLimit => DailyStatus::DailyLimit,
        Admission::CalendarRollback => DailyStatus::CalendarRollback,
        Admission::Waiting { .. } => DailyStatus::Waiting,
        Admission::RetryUnavailable => DailyStatus::Unavailable,
    }
}
fn attempt(
    c: &Connection,
    space: &str,
    trigger: Trigger,
    now_source: &impl Fn(&mut Global) -> StorageResult<(Moment, u64)>,
) -> StorageResult<DailyBackups> {
    attempt_checked(c, space, trigger, now_source, |_| Ok(()))
}
fn attempt_checked(
    c: &Connection,
    space: &str,
    trigger: Trigger,
    now_source: &impl Fn(&mut Global) -> StorageResult<(Moment, u64)>,
    checkpoint: impl Fn(&Connection) -> StorageResult<()>,
) -> StorageResult<DailyBackups> {
    let mut g = global(c)?;
    let (now, date) = now_source(&mut g)?;
    let mut s = state(c, space)?;
    let current = revision(c, space)?;
    let admitted = s
        .policy
        .begin(now, current, trigger)
        .map_err(|_| invalid())?;
    let Admission::Start(active) = admitted else {
        return Ok(view(&s, now, status(admitted)));
    };
    if s.obsolete.len() >= 64 {
        s.policy.fail(active.sequence, now).map_err(|_| invalid())?;
        save(c, &state_key(space), &s)?;
        save(c, GLOBAL, &g)?;
        return Ok(view(&s, now, DailyStatus::Unavailable));
    }
    let outcome = files::capture(c, &mut g, space, date);
    let (finished, _) = now_source(&mut g)?;
    let result = match outcome {
        Ok(reference) => {
            if s.policy.succeed(active.sequence, finished).is_ok() {
                s.files.push(reference.clone());
                s.last_success = Some(reference);
                if s.files.len() > 7 {
                    s.obsolete.push(s.files.remove(0));
                }
                DailyStatus::Saved
            } else {
                s.policy
                    .fail(active.sequence, finished)
                    .map_err(|_| invalid())?;
                DailyStatus::CalendarRollback
            }
        }
        Err(_) => {
            s.policy
                .fail(active.sequence, finished)
                .map_err(|_| invalid())?;
            DailyStatus::Unavailable
        }
    };
    save(c, &state_key(space), &s)?;
    save(c, GLOBAL, &g)?;
    checkpoint(c)?;
    Ok(view(&s, finished, result))
}
impl Storage {
    pub async fn initialize_daily_backups(&self) -> StorageResult<()> {
        self.transact(|ctx| {
            if load::<Global>(ctx.connection(), GLOBAL)?.is_none() {
                let mut id = [0; 16];
                getrandom::fill(&mut id).map_err(|_| unavailable())?;
                save(
                    ctx.connection(),
                    GLOBAL,
                    &Global {
                        installation: id.iter().map(|b| format!("{b:02x}")).collect(),
                        folder_initialized: false,
                        elapsed_ms: 0,
                        wall_ms: u64::try_from(
                            SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map_err(|_| unavailable())?
                                .as_millis(),
                        )
                        .map_err(|_| unavailable())?,
                    },
                )?;
            }
            Ok(())
        })
        .await
    }
    pub async fn daily_tick(&self, changed: bool) -> StorageResult<bool> {
        self.initialize_daily_backups().await?;
        let spaces = self
            .read(|ctx| {
                let mut q = ctx
                    .connection()
                    .prepare("SELECT space FROM documents ORDER BY space")?;
                Ok(q.query_map([], |r| r.get::<_, String>(0))?
                    .collect::<Result<Vec<_>, _>>()?)
            })
            .await?;
        let mut failed = false;
        let mut completed = false;
        for space in spaces {
            let outcome = self
                .transact(move |ctx| {
                    let s = state(ctx.connection(), &space)?;
                    let mut g = global(ctx.connection())?;
                    let (now, _) = sample(&mut g)?;
                    if s.policy
                        .last_success()
                        .is_some_and(|last| last.finished.local_day >= now.local_day)
                    {
                        return Ok((false, false));
                    }
                    if let Some(failure) = s.policy.last_failure()
                        && now.elapsed_ms.saturating_sub(failure.finished_ms) < RETRY_DELAY_MS
                    {
                        return Ok((true, false));
                    }
                    if changed || s.policy.last_failure().is_some() {
                        if crate::spaces::snapshot(ctx.read().document(&space)?, &space, true)?
                            .contents
                            .is_none()
                        {
                            return Ok((false, false));
                        }
                        let _ = attempt(ctx.connection(), &space, Trigger::Automatic, &sample)?;
                    }
                    let updated = state(ctx.connection(), &space)?;
                    Ok((
                        updated.policy.last_failure().is_some(),
                        updated.policy.last_success() != s.policy.last_success()
                            || updated.policy.last_failure() != s.policy.last_failure(),
                    ))
                })
                .await;
            let (pending, finished) = match outcome {
                Ok(result) => result,
                Err(_) => {
                    failed = true;
                    continue;
                }
            };
            failed |= pending;
            completed |= finished;
        }
        if completed {
            self.cleanup_daily_backups().await?;
        }
        Ok(failed)
    }
    pub async fn cleanup_daily_backups(&self) -> StorageResult<()> {
        self.transact(|ctx| files::cleanup(ctx.connection())).await
    }
    pub async fn retry_daily_cleanup(&self) -> StorageResult<()> {
        self.transact(|ctx| {
            let mut upper = PREFIX.as_bytes().to_vec();
            upper.push(255);
            let mut q = ctx
                .connection()
                .prepare("SELECT key FROM command_results WHERE key>=?1 AND key<?2")?;
            let keys = q
                .query_map((PREFIX.as_bytes(), upper), |r| r.get::<_, Vec<u8>>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            for key in keys {
                let space = std::str::from_utf8(&key[PREFIX.len()..]).map_err(|_| invalid())?;
                if !state(ctx.connection(), space)?.obsolete.is_empty() {
                    return files::cleanup(ctx.connection());
                }
            }
            Ok(())
        })
        .await
    }
}
impl DailyBackupStore for WriteContext<'_> {
    fn inspect_daily(&mut self, space: &str) -> CommandResult<DailyBackups> {
        let result = (|| {
            let mut g = global(self.connection())?;
            let (now, _) = sample(&mut g)?;
            let s = state(self.connection(), space)?;
            let mut policy = s.policy.clone();
            let admission = policy
                .begin(now, revision(self.connection(), space)?, Trigger::Automatic)
                .map_err(|_| invalid())?;
            Ok(view(&s, now, status(admission)))
        })();
        Ok(self.checked(result)?)
    }
    fn retry_daily(&mut self, request: &RetryDailyBackup) -> CommandResult<DailyBackups> {
        self.flush_document(&request.space)?;
        let result = attempt(
            self.connection(),
            &request.space,
            Trigger::Retry {
                failed_sequence: request.failed_sequence,
            },
            &sample,
        );
        Ok(self.checked(result)?)
    }
    fn delete_daily(&mut self, request: &DeleteDailyBackup) -> CommandResult<DailyBackups> {
        let result =
            (|| {
                let mut s = state(self.connection(), &request.space)?;
                let i = s.files.iter().position(|r| r.id == request.copy).ok_or(
                    StorageError::Rejected(CommandError {
                        code: ErrorCode::NotFound,
                    }),
                )?;
                if s.obsolete.len() >= 64 {
                    return Err(unavailable());
                }
                s.obsolete.push(s.files.remove(i));
                save(self.connection(), &state_key(&request.space), &s)?;
                Ok(())
            })();
        self.checked(result)?;
        self.inspect_daily(&request.space)
    }
    fn read_daily(&mut self, request: &ReadDailyBackup) -> CommandResult<SpaceArchive> {
        let result = (|| {
            let s = state(self.connection(), &request.space)?;
            let r = s
                .files
                .iter()
                .find(|r| r.id == request.copy)
                .ok_or(StorageError::Rejected(CommandError {
                    code: ErrorCode::NotFound,
                }))?;
            let bytes = files::read(self.connection(), r)?;
            Ok(SpaceArchive {
                archive: bytes.iter().map(|b| format!("{b:02x}")).collect(),
            })
        })();
        Ok(self.checked(result)?)
    }
}
