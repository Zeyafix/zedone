//! A bounded synthetic handler only. No product command or test access backend
//! is compiled into the library, adapters or contract.
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use automerge::{ROOT, ReadDoc, transaction::Transactable};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use utoipa::ToSchema;
use zedone_core::{
    access::{
        AccessTransaction, Clock, Credential, Grant, GrantScope, RequiredAccess, ResolvedAccess,
        SpacePermission,
    },
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, CommandId, Execution, RecordedCommand, ReplayTransaction, StateVersion},
    wire::{CommandRequest, CommandResponse, decode_request},
};

use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
struct Card {
    title: String,
    description: String,
    tags: Vec<String>,
    completed: bool,
    column: String,
    order: u32,
    trashed: bool,
}

#[derive(Clone, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct SetCompleted {
    space: String,
    card: String,
    expected: StateVersion,
    completed: bool,
}
impl Command for SetCompleted {
    const NAME: &'static str = "test.set_completed.v1";
    type Output = Completion;
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct Completion {
    completed: bool,
    version: StateVersion,
}

struct TestClock;
impl Clock for TestClock {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_secs(100))
    }
}

fn access() -> ResolvedAccess {
    ResolvedAccess {
        subject: "subject".into(),
        grant: Grant {
            subject: "subject".into(),
            scope: GrantScope::Spaces(BTreeMap::from([
                ("a".into(), SpacePermission::ReadWrite),
                ("b".into(), SpacePermission::ReadWrite),
            ])),
            expires_at: None,
            revoked: false,
        },
    }
}

struct Session<'a, 'b> {
    ctx: &'a mut WriteContext<'b>,
    access: &'a ResolvedAccess,
    receipt_reads: usize,
}
impl AccessTransaction for Session<'_, '_> {
    fn resolve_access(
        &mut self,
        credential: &Credential<'_>,
    ) -> CommandResult<Option<ResolvedAccess>> {
        Ok((credential.0 == b"test-only").then(|| self.access.clone()))
    }
}
impl ReplayTransaction for Session<'_, '_> {
    fn recorded(&mut self, key: &[u8]) -> CommandResult<Option<RecordedCommand>> {
        self.receipt_reads += 1;
        Ok(self.ctx.read().recorded(key)?)
    }
    fn record(&mut self, key: &[u8], command: &RecordedCommand) -> CommandResult<()> {
        Ok(self.ctx.record(key, command)?)
    }
}

fn load(ctx: &mut ReadContext<'_>, space: &str) -> StorageResult<Card> {
    let doc = ctx.document(space)?;
    let value = doc
        .get(ROOT, "card")?
        .ok_or(StorageError::InvalidDatabase)?
        .0;
    serde_json::from_str(value.as_str().ok_or(StorageError::InvalidDatabase)?)
        .map_err(|_| StorageError::InvalidDatabase)
}

fn version(ctx: &mut ReadContext<'_>, space: &str) -> StorageResult<StateVersion> {
    let card = load(ctx, space)?;
    let mut causes: Vec<String> = ctx
        .document(space)?
        .get_all(ROOT, "card")?
        .into_iter()
        .map(|(_, id)| id.to_string())
        .collect();
    causes.sort();
    Ok(state_version(
        &("test.card.v1", space, "card"),
        &(card, causes),
    )?)
}

fn run_command(
    ctx: &mut WriteContext<'_>,
    command: &SetCompleted,
    id: CommandId,
    access: &ResolvedAccess,
) -> StorageResult<Execution<Completion>> {
    if command.card != "card" || command.space.is_empty() {
        return Err(CommandError {
            code: ErrorCode::InvalidRequest,
        }
        .into());
    }
    let mut session = Session {
        ctx,
        access,
        receipt_reads: 0,
    };
    let result = replay::execute(
        &mut session,
        &Credential(b"test-only"),
        &TestClock,
        &[RequiredAccess::Space {
            id: &command.space,
            permission: SpacePermission::ReadWrite,
        }],
        id,
        command,
        |tx| {
            let mut card = load(tx.ctx.read(), &command.space)?;
            replay::require_current(command.expected, version(tx.ctx.read(), &command.space)?)?;
            card.completed = command.completed;
            let bytes = serde_json::to_string(&card).unwrap();
            tx.ctx.edit(&command.space, |doc| {
                let effects = doc.get(ROOT, "effects")?.unwrap().0.as_u64().unwrap();
                doc.put(ROOT, "card", bytes)?;
                doc.put(ROOT, "effects", effects + 1)?;
                Ok(())
            })?;
            Ok(Completion {
                completed: card.completed,
                version: version(tx.ctx.read(), &command.space)?,
            })
        },
    );
    if result
        .as_ref()
        .is_err_and(|e| e.code == ErrorCode::AccessDenied)
    {
        assert_eq!(
            session.receipt_reads, 0,
            "authorization must precede lookup"
        );
    }
    Ok(result?)
}

async fn call(
    storage: &Storage,
    command: SetCompleted,
    id: CommandId,
) -> StorageResult<Execution<Completion>> {
    storage
        .transact(move |ctx| run_command(ctx, &command, id, &access()))
        .await
}

async fn setup() -> (TempDir, Storage, SetCompleted) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let card = Card {
        title: "title".into(),
        description: "text".into(),
        tags: vec![],
        completed: false,
        column: "column".into(),
        order: 0,
        trashed: false,
    };

    storage
        .write(b"fixture".to_vec(), vec![], move |ctx| {
            for space in ["a", "b"] {
                ctx.create_document(space)?;
                ctx.edit(space, |tx| {
                    tx.put(ROOT, "card", serde_json::to_string(&card).unwrap())?;
                    tx.put(ROOT, "effects", 0_u64)?;
                    tx.put(ROOT, "history-0", "title + text")?;
                    Ok(())
                })?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let expected = storage.read(|ctx| version(ctx, "a")).await.unwrap();
    (
        dir,
        storage,
        SetCompleted {
            space: "a".into(),
            card: "card".into(),
            expected,
            completed: true,
        },
    )
}

async fn effects(storage: &Storage) -> u64 {
    storage
        .read(|ctx| {
            Ok(ctx
                .document("a")?
                .get(ROOT, "effects")?
                .unwrap()
                .0
                .as_u64()
                .unwrap())
        })
        .await
        .unwrap()
}

// Exercise the wire boundary around the same test-only core/storage handler.
// This is not an HTTP/CLI/IPC adapter or a product authentication backend.
async fn deliver(storage: &Storage, bytes: &[u8]) -> serde_json::Value {
    let completed = match decode_request::<SetCompleted>(bytes) {
        Ok(request) => call(storage, request.parameters, request.id)
            .await
            .map_err(CommandError::from),
        Err(error) => Err(error),
    };
    serde_json::to_value(CommandResponse::from(completed)).unwrap()
}

#[tokio::test]
async fn wire_delivery_keeps_original_receipt_after_reverse_and_rejects_changed_intentions() {
    let (dir, storage, command) = setup().await;
    let request = CommandRequest {
        id: [1; 16],
        operation: SetCompleted::NAME.into(),
        parameters: command.clone(),
    };
    let original = serde_json::to_vec(&request).unwrap();
    let first = deliver(&storage, &original).await;
    assert_eq!(first["status"], "ok");
    assert_eq!(first["replayed"], false);
    let mut reverse = command;
    reverse.expected = serde_json::from_value(first["result"]["version"].clone()).unwrap();
    reverse.completed = false;
    call(&storage, reverse, [2; 16]).await.unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let repeated = deliver(&storage, &original).await;
    assert_eq!(repeated["result"], first["result"]);
    assert_eq!(repeated["replayed"], true);
    let mut changed = serde_json::to_value(&request).unwrap();
    changed["parameters"]["completed"] = false.into();
    let mismatch = deliver(&storage, &serde_json::to_vec(&changed).unwrap()).await;
    assert_eq!(
        mismatch,
        serde_json::json!({"status":"error", "error":{"code":"command_mismatch"}})
    );
    changed["id"] = serde_json::json!([3; 16].to_vec());
    let stale = deliver(&storage, &serde_json::to_vec(&changed).unwrap()).await;
    assert_eq!(
        stale,
        serde_json::json!({"status":"error", "error":{"code":"stale_state"}})
    );
    assert_eq!(effects(&storage).await, 2);
    storage
        .read(|ctx| {
            assert!(!load(ctx, "a")?.completed);
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

fn assert_code<T: std::fmt::Debug>(result: StorageResult<T>, code: ErrorCode) {
    assert_eq!(
        result.unwrap_err(),
        StorageError::Rejected(CommandError { code })
    );
}

#[tokio::test]
async fn replay_after_reverse_and_restart_preserves_current_state_and_original_result() {
    let (dir, storage, command) = setup().await;
    let first = call(&storage, command.clone(), [1; 16]).await.unwrap();
    assert!(!first.replayed);
    let mut reverse = command.clone();
    reverse.expected = first.result.version;
    reverse.completed = false;
    call(&storage, reverse, [2; 16]).await.unwrap();
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let again = call(&storage, command.clone(), [1; 16]).await.unwrap();
    assert!(again.replayed);
    assert_eq!(again.result, first.result);
    assert_eq!(effects(&storage).await, 2);
    storage
        .read(|ctx| {
            assert!(!load(ctx, "a")?.completed);
            assert_eq!(
                ctx.document("a")?
                    .get(ROOT, "history-0")?
                    .unwrap()
                    .0
                    .as_str(),
                Some("title + text")
            );
            Ok(())
        })
        .await
        .unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn changed_parameters_target_condition_or_operation_cannot_reuse_a_completed_id() {
    let (_dir, storage, command) = setup().await;
    let first = call(&storage, command.clone(), [1; 16]).await.unwrap();
    let mut changed = command.clone();
    changed.completed = false;
    assert_code(
        call(&storage, changed, [1; 16]).await,
        ErrorCode::CommandMismatch,
    );
    let mut changed = command.clone();
    changed.space = "b".into();
    assert_code(
        call(&storage, changed, [1; 16]).await,
        ErrorCode::CommandMismatch,
    );
    let mut changed = command.clone();
    changed.expected = first.result.version;
    assert_code(
        call(&storage, changed, [1; 16]).await,
        ErrorCode::CommandMismatch,
    );
    #[derive(Serialize, Deserialize, ToSchema)]
    struct Other(SetCompleted);
    impl Command for Other {
        const NAME: &'static str = "test.other.v1";
        type Output = Completion;
    }
    let result = storage
        .transact(move |ctx| {
            let rights = access();
            let mut tx = Session {
                ctx,
                access: &rights,
                receipt_reads: 0,
            };
            Ok(replay::execute(
                &mut tx,
                &Credential(b"test-only"),
                &TestClock,
                &[RequiredAccess::Space {
                    id: "a",
                    permission: SpacePermission::ReadWrite,
                }],
                [1; 16],
                &Other(command),
                |_| panic!("different operation executed"),
            )?)
        })
        .await;
    assert_code(result, ErrorCode::CommandMismatch);
    assert_eq!(effects(&storage).await, 1);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn every_semantic_card_change_rejects_an_unexecuted_old_command() {
    for field in 0..8 {
        let (_dir, storage, command) = setup().await;
        storage
            .transact(move |ctx| {
                let mut card = load(ctx.read(), "a")?;
                match field {
                    0 => card.title.push('!'),
                    1 => card.description.push('!'),
                    2 => card.tags.push("tag".into()),
                    3 => card.completed = true,
                    4 => card.column.push('!'),
                    5 => card.order = 1,
                    6 => card.trashed = true,
                    7 => {
                        card.tags.push("tag".into());
                        card.title.push('!');
                    }
                    _ => unreachable!(),
                }
                ctx.edit("a", |tx| {
                    tx.put(ROOT, "card", serde_json::to_string(&card).unwrap())?;
                    Ok(())
                })
            })
            .await
            .unwrap();
        assert_code(
            call(&storage, command.clone(), [1; 16]).await,
            ErrorCode::StaleState,
        );
        assert_eq!(effects(&storage).await, 0);
        let fresh = storage.read(|ctx| version(ctx, "a")).await.unwrap();
        let mut new_command = command;
        new_command.expected = fresh;
        call(&storage, new_command, [2; 16]).await.unwrap();
        assert_eq!(effects(&storage).await, 1);
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn unrelated_space_and_technical_history_do_not_invalidate_a_card() {
    let (_dir, storage, command) = setup().await;
    storage
        .transact(|ctx| {
            for space in ["a", "b"] {
                ctx.edit(space, |tx| {
                    tx.put(ROOT, "technical-history", "closed")?;
                    Ok(())
                })?;
            }
            let mut other = load(ctx.read(), "b")?;
            other.title.push('!');
            ctx.edit("b", |tx| {
                tx.put(ROOT, "card", serde_json::to_string(&other).unwrap())?;
                Ok(())
            })
        })
        .await
        .unwrap();
    call(&storage, command, [1; 16]).await.unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn concurrent_delivery_across_workers_runs_one_effect_and_one_receipt() {
    let (dir, storage, command) = setup().await;
    let other = Storage::open(dir.path().join("device.db")).await.unwrap();
    let mut deliveries = Vec::new();
    for n in 0..40 {
        let worker = if n % 2 == 0 {
            storage.clone()
        } else {
            other.clone()
        };
        let command = command.clone();
        deliveries.push(tokio::spawn(async move {
            call(&worker, command, [1; 16]).await.unwrap()
        }));
    }
    let mut executed = 0;
    for delivery in deliveries {
        executed += usize::from(!delivery.await.unwrap().replayed);
    }
    assert_eq!(executed, 1);
    assert_eq!(effects(&storage).await, 1);
    other.shutdown().await.unwrap();
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn competing_new_intentions_cannot_bypass_the_freshness_check() {
    let (_dir, storage, command) = setup().await;
    let mut deliveries = Vec::new();
    for n in 1..=16 {
        let worker = storage.clone();
        let command = command.clone();
        deliveries.push(tokio::spawn(async move {
            call(&worker, command, [n; 16]).await
        }));
    }
    let mut executed = 0;
    for delivery in deliveries {
        match delivery.await.unwrap() {
            Ok(_) => executed += 1,
            result => assert_code(result, ErrorCode::StaleState),
        }
    }
    assert_eq!(executed, 1);
    assert_eq!(effects(&storage).await, 1);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn revoked_expired_and_reduced_rights_cannot_read_a_saved_result() {
    let (_dir, storage, command) = setup().await;
    call(&storage, command.clone(), [1; 16]).await.unwrap();
    for case in 0..3 {
        let command = command.clone();
        let result = storage
            .transact(move |ctx| {
                let mut rights = access();
                match case {
                    0 => rights.grant.revoked = true,
                    1 => rights.grant.expires_at = Some(UNIX_EPOCH + Duration::from_secs(100)),
                    2 => {
                        rights.grant.scope = GrantScope::Spaces(BTreeMap::from([(
                            "a".into(),
                            SpacePermission::Read,
                        )]))
                    }
                    _ => unreachable!(),
                }
                run_command(ctx, &command, [1; 16], &rights)
            })
            .await;
        assert_code(result, ErrorCode::AccessDenied);
    }
    assert!(call(&storage, command, [1; 16]).await.unwrap().replayed);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn subject_namespace_does_not_reveal_or_replay_someone_elses_result() {
    let (_dir, storage, command) = setup().await;
    let first = call(&storage, command.clone(), [1; 16]).await.unwrap();
    let mut next = command;
    next.expected = first.result.version;
    next.completed = false;
    let result = storage
        .transact(move |ctx| {
            let mut rights = access();
            rights.subject = "another".into();
            rights.grant.subject = rights.subject.clone();
            run_command(ctx, &next, [1; 16], &rights)
        })
        .await
        .unwrap();
    assert!(!result.replayed);
    assert!(!result.result.completed);
    assert_eq!(effects(&storage).await, 2);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn rights_are_resolved_after_waiting_on_worker_queue() {
    let (_dir, storage, command) = setup().await;
    let rights = Arc::new(Mutex::new(access()));
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let worker = storage.clone();
    let blocker = tokio::spawn(async move {
        worker
            .transact(move |_| {
                entered.send(()).unwrap();
                held.recv().unwrap();
                Ok(())
            })
            .await
    });
    waiting.await.unwrap();
    let worker = storage.clone();
    let source = rights.clone();
    let delivery = tokio::spawn(async move {
        worker
            .transact(move |ctx| {
                let guard = source.lock().unwrap();
                run_command(ctx, &command, [1; 16], &guard)
            })
            .await
    });
    tokio::task::yield_now().await;
    rights.lock().unwrap().grant.revoked = true;
    release.send(()).unwrap();
    blocker.await.unwrap().unwrap();
    assert_code(delivery.await.unwrap(), ErrorCode::AccessDenied);
    assert_eq!(effects(&storage).await, 0);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn receipt_sql_failure_rolls_back_mutation_and_retry_can_execute_once() {
    let (dir, storage, command) = setup().await;
    let connection = rusqlite::Connection::open(dir.path().join("device.db")).unwrap();
    connection.execute_batch("CREATE TRIGGER fail_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'test'); END;").unwrap();
    assert_code(
        call(&storage, command.clone(), [1; 16]).await,
        ErrorCode::Unavailable,
    );
    assert_eq!(effects(&storage).await, 0);
    connection
        .execute_batch("DROP TRIGGER fail_receipt;")
        .unwrap();
    assert!(
        !call(&storage, command.clone(), [1; 16])
            .await
            .unwrap()
            .replayed
    );
    assert!(call(&storage, command, [1; 16]).await.unwrap().replayed);
    assert_eq!(effects(&storage).await, 1);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn corrupt_receipt_fails_closed_instead_of_executing_again() {
    let (dir, storage, command) = setup().await;
    call(&storage, command.clone(), [1; 16]).await.unwrap();
    let connection = rusqlite::Connection::open(dir.path().join("device.db")).unwrap();
    connection
        .execute(
            "UPDATE command_results SET result = X'00' WHERE key != ?1",
            [b"fixture".as_slice()],
        )
        .unwrap();
    assert_code(
        call(&storage, command, [1; 16]).await,
        ErrorCode::Unavailable,
    );
    assert_eq!(effects(&storage).await, 1);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn fault_boundaries_never_return_false_success_or_duplicate_effects() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (_dir, storage, command) = setup().await;
        let delivery = command.clone();
        let result = storage
            .transact_checked(
                move |ctx| run_command(ctx, &delivery, [1; 16], &access()),
                move |here| {
                    if here == stage {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await;
        assert_eq!(
            result.unwrap_err(),
            if stage == Stage::AfterCommit {
                StorageError::OutcomeUnknown
            } else {
                StorageError::Unavailable
            }
        );
        let retry = call(&storage, command, [1; 16]).await.unwrap();
        assert_eq!(retry.replayed, stage == Stage::AfterCommit);
        assert_eq!(effects(&storage).await, 1);
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn cancelled_response_does_not_cancel_admitted_command() {
    let (_dir, storage, command) = setup().await;
    let (entered, waiting) = tokio::sync::oneshot::channel();
    let (release, held) = std::sync::mpsc::channel();
    let worker = storage.clone();
    let delivery = command.clone();
    let task = tokio::spawn(async move {
        worker
            .transact(move |ctx| {
                let result = run_command(ctx, &delivery, [1; 16], &access())?;
                entered.send(()).unwrap();
                held.recv().unwrap();
                Ok(result)
            })
            .await
    });
    waiting.await.unwrap();
    task.abort();
    release.send(()).unwrap();
    assert!(task.await.unwrap_err().is_cancelled());
    assert!(call(&storage, command, [1; 16]).await.unwrap().replayed);
    assert_eq!(effects(&storage).await, 1);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn process_stop_and_reopen_replays_only_a_committed_command() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (dir, storage, command) = setup().await;
        storage.shutdown().await.unwrap();
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "replay_tests::replay_crash_child", "--nocapture"])
            .env("ZEDONE_REPLAY_CRASH_DB", dir.path().join("device.db"))
            .env("ZEDONE_REPLAY_CRASH_STAGE", format!("{stage:?}"))
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(71));
        let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
        let retry = call(&storage, command.clone(), [1; 16]).await.unwrap();
        assert_eq!(retry.replayed, stage == Stage::AfterCommit);
        assert!(call(&storage, command, [1; 16]).await.unwrap().replayed);
        assert_eq!(effects(&storage).await, 1);
        storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn returning_to_the_same_values_does_not_revive_an_old_unexecuted_intention() {
    let (_dir, storage, command) = setup().await;
    let first = call(&storage, command.clone(), [1; 16]).await.unwrap();
    let mut reverse = command.clone();
    reverse.expected = first.result.version;
    reverse.completed = false;
    let reversed = call(&storage, reverse, [2; 16]).await.unwrap();
    assert_ne!(reversed.result.version, command.expected);
    assert_code(
        call(&storage, command.clone(), [3; 16]).await,
        ErrorCode::StaleState,
    );
    assert!(call(&storage, command, [1; 16]).await.unwrap().replayed);
    assert_eq!(effects(&storage).await, 2);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn zero_command_id_is_rejected_without_mutation() {
    let (_dir, storage, command) = setup().await;
    assert_code(
        call(&storage, command, [0; 16]).await,
        ErrorCode::InvalidRequest,
    );
    assert_eq!(effects(&storage).await, 0);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn caught_receipt_error_still_prevents_partial_commit() {
    let (_dir, storage, command) = setup().await;
    let result = storage
        .transact(move |ctx| {
            run_command(ctx, &command, [1; 16], &access())?;
            // Invalid empty storage key fails after the successful synthetic edit.
            let rejected = ctx.record(
                &[],
                &RecordedCommand {
                    request: vec![],
                    result: vec![],
                },
            );
            assert!(rejected.is_err());
            Ok(())
        })
        .await;
    assert_eq!(result.unwrap_err(), StorageError::Unavailable);
    assert_eq!(effects(&storage).await, 0);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn panic_after_handler_is_unknown_and_original_retry_executes_only_once() {
    let (_dir, storage, command) = setup().await;
    let delivery = command.clone();
    let result: StorageResult<()> = storage
        .transact(move |ctx| {
            run_command(ctx, &delivery, [1; 16], &access())?;
            panic!("test-only failure before commit")
        })
        .await;
    assert_eq!(result.unwrap_err(), StorageError::OutcomeUnknown);
    assert_eq!(effects(&storage).await, 0);
    assert!(
        !call(&storage, command.clone(), [1; 16])
            .await
            .unwrap()
            .replayed
    );
    assert!(call(&storage, command, [1; 16]).await.unwrap().replayed);
    assert_eq!(effects(&storage).await, 1);
    storage.shutdown().await.unwrap();
}

#[test]
fn container_projection_detects_data_membership_and_causal_changes() {
    // This tests the shared token primitive, not P04 deletion projections.
    let original = serde_json::json!({
        "name": "column", "position": 0, "membership_causes": ["add-1"],
        "cards": [{"id": "card", "title": "title", "cause": "edit-1"}]
    });
    let token = state_version(&("container.v1", "a", "column"), &original).unwrap();
    let mut variants = Vec::new();
    let mut changed = original.clone();
    changed["name"] = "renamed".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed["position"] = 1.into();
    variants.push(changed);
    let mut changed = original.clone();
    changed["cards"][0]["title"] = "edited".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed["cards"][0]["cause"] = "edit-2".into();
    variants.push(changed);
    let mut changed = original.clone();
    changed["cards"] = serde_json::json!([]);
    variants.push(changed);
    let mut changed = original.clone();
    changed["membership_causes"] = serde_json::json!(["add-1", "add-2", "remove-2"]);
    variants.push(changed);
    for changed in variants {
        let next = state_version(&("container.v1", "a", "column"), &changed).unwrap();
        assert_eq!(
            replay::require_current(token, next).unwrap_err().code,
            ErrorCode::StaleState
        );
    }
}

#[derive(Deserialize, ToSchema)]
struct Unserializable;
impl Serialize for Unserializable {
    fn serialize<S: serde::Serializer>(&self, _serializer: S) -> Result<S::Ok, S::Error> {
        Err(serde::ser::Error::custom("test-only serialization failure"))
    }
}

#[test]
fn unrepresentable_authoritative_projection_is_unavailable() {
    assert_eq!(
        state_version(&"test", &Unserializable).unwrap_err().code,
        ErrorCode::Unavailable
    );
}

#[tokio::test]
async fn result_serialization_failure_cannot_commit_the_handler_effect() {
    #[derive(Serialize, Deserialize, ToSchema)]
    struct Broken(SetCompleted);
    impl Command for Broken {
        const NAME: &'static str = "test.unserializable.v1";
        type Output = Unserializable;
    }
    let (_dir, storage, command) = setup().await;
    let delivery = command.clone();
    let result: StorageResult<()> = storage
        .transact(move |ctx| {
            let rights = access();
            let mut tx = Session {
                ctx,
                access: &rights,
                receipt_reads: 0,
            };
            replay::execute(
                &mut tx,
                &Credential(b"test-only"),
                &TestClock,
                &[RequiredAccess::Space {
                    id: "a",
                    permission: SpacePermission::ReadWrite,
                }],
                [1; 16],
                &Broken(delivery),
                |session| {
                    session.ctx.edit("a", |tx| {
                        tx.put(ROOT, "effects", 1_u64)?;
                        Ok(())
                    })?;
                    Ok(Unserializable)
                },
            )?;
            Ok(())
        })
        .await;
    assert_code(result, ErrorCode::Unavailable);
    assert_eq!(effects(&storage).await, 0);
    assert!(!call(&storage, command, [1; 16]).await.unwrap().replayed);
    assert_eq!(effects(&storage).await, 1);
    storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn replay_crash_child() {
    let Some(path) = std::env::var_os("ZEDONE_REPLAY_CRASH_DB") else {
        return;
    };
    let stage = std::env::var("ZEDONE_REPLAY_CRASH_STAGE").unwrap();
    let storage = Storage::open(std::path::PathBuf::from(path)).await.unwrap();
    let expected = storage.read(|ctx| version(ctx, "a")).await.unwrap();
    let command = SetCompleted {
        space: "a".into(),
        card: "card".into(),
        expected,
        completed: true,
    };
    storage
        .transact_checked(
            move |ctx| run_command(ctx, &command, [1; 16], &access()),
            move |here| {
                if format!("{here:?}") == stage {
                    std::process::exit(71)
                }
                Ok(())
            },
        )
        .await
        .unwrap();
    panic!("checkpoint not reached");
}

#[test]
fn tokens_are_fixed_size_scoped_and_independent_of_map_insertion_order() {
    let one: serde_json::Value = serde_json::from_str(r#"{"a":1,"b":2}"#).unwrap();
    let two: serde_json::Value = serde_json::from_str(r#"{"b":2,"a":1}"#).unwrap();
    let first = state_version(&("v1", "a", "card"), &one).unwrap();
    assert_eq!(first, state_version(&("v1", "a", "card"), &two).unwrap());
    assert_ne!(first, state_version(&("v1", "b", "card"), &one).unwrap());
    assert_ne!(first, state_version(&("v2", "a", "card"), &one).unwrap());
    assert_eq!(first.0.len(), 32);
}
