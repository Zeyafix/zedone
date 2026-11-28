use super::*;
use crate::{Storage, access::PendingAccess};
use std::sync::Arc;
use zedone_core::{
    access::{Credential, RequiredAccess, with_authorized},
    spaces::SpaceStore,
};

struct Fixture {
    root: tempfile::TempDir,
    path: PathBuf,
    copy: String,
    old_key: Vec<u8>,
    original_backup: crate::problem_state::Entry,
}
async fn fixture() -> Fixture {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("zedone.sqlite");
    let db = Storage::open(&path).await.unwrap();
    let pending = Arc::new(PendingAccess::generate().unwrap());
    let old_key = pending.credential().0.to_vec();
    let actor = pending.clone();
    db.transact(move |ctx| {
        ctx.bootstrap_access(&actor)?;
        ctx.create_space(&format!("{:032x}", 1), "До сбоя", actor.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    let folder = ownership::sibling(&path, ".backups");
    fs::create_dir(&folder).unwrap();
    let backup = db
        .prepare_migration_backup(&folder, 123, files::LIMITS)
        .await
        .unwrap();
    let copy = backup
        .path()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned();
    let original_backup = files::hash(&backup.path().join("database.sqlite")).unwrap();
    drop(backup);
    db.shutdown().await.unwrap();
    fs::write(&path, b"damaged after crash").unwrap();
    Fixture {
        root,
        path,
        copy,
        old_key,
        original_backup,
    }
}
fn inspect(e: &mut Emergency, key: &str) -> MaintenanceView {
    e.execute(key, [1; 16], MaintenanceCommand::Inspect {}, false)
        .unwrap()
        .result
}
fn prepare(e: &mut Emergency, key: &str, copy: &str) -> MaintenanceView {
    let v = inspect(e, key);
    let mut id = [0; 16];
    id.copy_from_slice(&v.version.0[..16]);
    id[0] = 0xa5;
    e.execute(
        key,
        id,
        MaintenanceCommand::Prepare {
            copy: copy.into(),
            expected: v.version,
        },
        false,
    )
    .unwrap()
    .result
}
fn apply_command(v: &MaintenanceView) -> MaintenanceCommand {
    MaintenanceCommand::Apply {
        plan: v.plan.as_ref().unwrap().id.clone(),
        expected: v.version,
    }
}
fn engine(f: &Fixture) -> (Emergency, String) {
    let mut e = Emergency::open(f.path.clone()).unwrap();
    e.activate_failed("Повреждение".into()).unwrap();
    let key = e.owner_session().unwrap().key;
    (e, key)
}

#[tokio::test]
async fn checked_full_restore_revokes_all_old_keys_rotates_actors_and_survives_restart_without_reexecution()
 {
    let f = fixture().await;
    let (mut e, key) = engine(&f);
    let before = fs::read(&f.path).unwrap();
    let v = prepare(&mut e, &key, &f.copy);
    let problem = v.plan.as_ref().unwrap().problem.clone();
    assert_eq!(fs::read(&f.path).unwrap(), before);
    assert!(Storage::open(&f.path).await.is_err());
    let command = apply_command(&v);
    let restored = e.execute(&key, [3; 16], command.clone(), false).unwrap();
    assert_eq!(
        restored.result.plan.as_ref().unwrap().phase,
        RecoveryPhase::Restored
    );
    let raw = e.directory.join(&problem).join("database.sqlite");
    assert_eq!(fs::read(raw).unwrap(), before);
    let candidate = e
        .directory
        .join(restored.result.plan.as_ref().unwrap().id.clone())
        .join("candidate.sqlite");
    let input = e
        .directory
        .join(restored.result.plan.as_ref().unwrap().id.clone())
        .join("input/database.sqlite");
    assert_eq!(files::hash(&input).unwrap(), f.original_backup);
    let old = Connection::open(&input).unwrap();
    let new = Connection::open(&candidate).unwrap();
    let old_actor: Vec<u8> = old
        .query_row("SELECT actor FROM documents", [], |r| r.get(0))
        .unwrap();
    let new_actor: Vec<u8> = new
        .query_row("SELECT actor FROM documents", [], |r| r.get(0))
        .unwrap();
    assert_ne!(old_actor, new_actor);
    drop(old);
    drop(new);
    drop(e);
    let db = Storage::open(&f.path).await.unwrap();
    let old_key = f.old_key.clone();
    db.timed_transact(move |tx, clock| {
        assert!(
            with_authorized(
                tx,
                &Credential(&old_key),
                &[RequiredAccess::ManageAccess],
                clock,
                |_, _| Ok(())
            )
            .is_err()
        );
        assert!(tx.read().space_sync_control(&format!("{:032x}", 1))?.paused);
        Ok(())
    })
    .await
    .unwrap();
    let local: LocalSession = read(&f.root.path().join("local-session.json")).unwrap();
    assert_ne!(local.key, key);
    let before = fs::read(&f.path).unwrap();
    let mut reopened = Emergency::open(f.path.clone()).unwrap();
    let replay = reopened.execute(&key, [3; 16], command, false).unwrap();
    assert!(replay.replayed);
    assert_eq!(fs::read(&f.path).unwrap(), before);
    assert!(
        reopened
            .execute(&key, [3; 16], MaintenanceCommand::Inspect {}, false)
            .is_err()
    );
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn external_old_database_empty_file_and_lost_database_never_bypass_recovery_generation() {
    for change in ["old", "empty", "missing"] {
        let f = fixture().await;
        let (mut e, key) = engine(&f);
        let v = prepare(&mut e, &key, &f.copy);
        e.execute(&key, [3; 16], apply_command(&v), false).unwrap();
        drop(e);
        match change {
            "old" => {
                fs::copy(
                    ownership::sibling(&f.path, ".backups")
                        .join(&f.copy)
                        .join("database.sqlite"),
                    &f.path,
                )
                .unwrap();
            }
            "empty" => fs::write(&f.path, []).unwrap(),
            _ => fs::remove_file(&f.path).unwrap(),
        }
        assert!(Storage::open(&f.path).await.is_err());
        if change == "missing" {
            assert!(!f.path.exists());
        }
        let (mut e, key) = engine(&f);
        let v = prepare(&mut e, &key, &f.copy);
        e.execute(&key, [8; 16], apply_command(&v), false).unwrap();
        drop(e);
        let db = Storage::open(&f.path).await.unwrap();
        db.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn missing_control_metadata_never_recreates_receipts_or_recovery_authority() {
    let f = fixture().await;
    let (e, _) = engine(&f);
    let directory = e.directory.clone();
    let owner = fs::read(directory.join("owner.json")).unwrap();
    drop(e);
    fs::remove_file(directory.join("state.json")).unwrap();
    assert!(Emergency::open(f.path.clone()).is_err());
    assert!(Storage::open(&f.path).await.is_err());
    assert_eq!(fs::read(directory.join("owner.json")).unwrap(), owner);
}

#[tokio::test]
async fn expired_recovery_role_cannot_publish_and_clock_floor_does_not_commit_working_mutations() {
    let f = fixture().await;
    let (e, key) = engine(&f);
    let mut state = e.load().unwrap();
    let role = e.authorize(&mut state, &key).unwrap();
    let old_revision = state.revision;
    state.revision += 1;
    state.reason = Some("uncommitted".into());
    let expired = Role {
        expires_ms: Some(1),
        ..role
    };
    assert!(e.refresh_role(&mut state, &expired).is_err());
    let stored = e.load().unwrap();
    assert_eq!(stored.revision, old_revision);
    assert_ne!(stored.reason, Some("uncommitted".into()));
    assert!(stored.last_ms >= state.last_ms);
}

#[tokio::test]
async fn interrupted_problem_deletion_resumes_from_durable_manifest_without_touching_database() {
    let f = fixture().await;
    let (mut e, key) = engine(&f);
    let v = prepare(&mut e, &key, &f.copy);
    let p = v.plan.unwrap();
    let cancelled = e
        .execute(
            &key,
            [4; 16],
            MaintenanceCommand::Cancel {
                plan: p.id,
                expected: v.version,
            },
            false,
        )
        .unwrap()
        .result;
    let command = MaintenanceCommand::DeleteProblem {
        problem: p.problem.clone(),
        expected: cancelled.version,
    };
    save(&e.directory.join("test-command.json"), &command).unwrap();
    drop(e);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "emergency::tests::crash_child", "--nocapture"])
        .env("ZEDONE_EMERGENCY_TEST_DB", &f.path)
        .env("ZEDONE_EMERGENCY_CRASH", "problem_delete")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(74));
    let mut e = Emergency::open(f.path.clone()).unwrap();
    assert!(e.load().unwrap().deleting.contains_key(&p.problem));
    e.execute(&key, [3; 16], command.clone(), false).unwrap();
    assert!(e.execute(&key, [3; 16], command, false).unwrap().replayed);
    assert!(!e.directory.join(p.problem).exists());
    assert_eq!(fs::read(&f.path).unwrap(), b"damaged after crash");
}

#[tokio::test]
async fn owner_can_explicitly_continue_after_lost_agent_intention_without_repeating_original_effect()
 {
    let f = fixture().await;
    let (mut e, key) = engine(&f);
    let v = prepare(&mut e, &key, &f.copy);
    let command = apply_command(&v);
    save(&e.directory.join("test-command.json"), &command).unwrap();
    drop(e);
    let status = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "emergency::tests::crash_child", "--nocapture"])
        .env("ZEDONE_EMERGENCY_TEST_DB", &f.path)
        .env("ZEDONE_EMERGENCY_CRASH", "database_replaced")
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(74));
    let mut e = Emergency::open(f.path.clone()).unwrap();
    let v = inspect(&mut e, &key);
    e.execute(
        &key,
        [9; 16],
        MaintenanceCommand::Continue {
            plan: v.plan.unwrap().id,
            expected: v.version,
        },
        false,
    )
    .unwrap();
    assert!(e.execute(&key, [3; 16], command, false).unwrap().replayed);
}

use rusqlite::Connection;
#[tokio::test]
async fn restart_keeps_the_saved_failure_but_a_new_attempt_updates_it() {
    let f = fixture().await;
    let (mut e, key) = engine(&f);
    e.activate_failed("UnsupportedSchema".into()).unwrap();
    drop(e);
    let mut e = Emergency::open(f.path.clone()).unwrap();
    e.activate_failed("RetryRequired".into()).unwrap();
    assert_eq!(
        inspect(&mut e, &key).reason.as_deref(),
        Some("UnsupportedSchema")
    );
    e.activate_failed("InvalidDatabase".into()).unwrap();
    let v = inspect(&mut e, &key);
    assert_eq!(v.reason.as_deref(), Some("InvalidDatabase"));
    e.release_for_retry(
        &key,
        [20; 16],
        &MaintenanceCommand::Retry {
            expected: v.version,
        },
    )
    .unwrap();
    e.activate_failed("RetryRequired".into()).unwrap();
    assert_eq!(
        inspect(&mut e, &key).reason.as_deref(),
        Some("RetryRequired")
    );
}

#[tokio::test]
async fn ordinary_workers_and_recovery_exclude_each_other_until_acknowledged_shutdown() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("db");
    let first = Storage::open(&path).await.unwrap();
    let second = Storage::open(&path).await.unwrap();
    let mut e = Emergency::open(path.clone()).unwrap();
    e.activate_failed("test".into()).unwrap();
    assert!(e.exclusive.is_none());
    first.shutdown().await.unwrap();
    e.activate_failed("test".into()).unwrap();
    assert!(e.exclusive.is_none());
    second.shutdown().await.unwrap();
    e.activate_failed("test".into()).unwrap();
    assert!(Storage::open(&path).await.is_err());
    let key = e.owner_session().unwrap().key;
    let v = inspect(&mut e, &key);
    let cmd = MaintenanceCommand::Retry {
        expected: v.version,
    };
    e.release_for_retry(&key, [4; 16], &cmd).unwrap();
    let db = Storage::open(&path).await.unwrap();
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn cancellation_tampering_and_invalid_bundle_never_replace_original_or_old_backups() {
    for change in ["cancel", "candidate", "raw", "source", "backup"] {
        let f = fixture().await;
        let (mut e, key) = engine(&f);
        let original = fs::read(&f.path).unwrap();
        if change == "backup" {
            fs::write(
                ownership::sibling(&f.path, ".backups")
                    .join(&f.copy)
                    .join("space-0.bin"),
                b"bad",
            )
            .unwrap();
            let v = inspect(&mut e, &key);
            assert!(
                e.execute(
                    &key,
                    [2; 16],
                    MaintenanceCommand::Prepare {
                        copy: f.copy.clone(),
                        expected: v.version
                    },
                    false
                )
                .is_err()
            );
            assert_eq!(fs::read(&f.path).unwrap(), original);
            continue;
        }
        let v = prepare(&mut e, &key, &f.copy);
        let p = v.plan.as_ref().unwrap();
        match change {
            "cancel" => {
                e.execute(
                    &key,
                    [5; 16],
                    MaintenanceCommand::Cancel {
                        plan: p.id.clone(),
                        expected: v.version,
                    },
                    false,
                )
                .unwrap();
                assert!(!e.directory.join(&p.id).exists());
            }
            "candidate" => {
                fs::write(e.directory.join(&p.id).join("candidate.sqlite"), b"tamper").unwrap();
                assert!(e.execute(&key, [3; 16], apply_command(&v), false).is_err());
            }
            "raw" => {
                fs::write(
                    e.directory.join(&p.problem).join("database.sqlite"),
                    b"tamper",
                )
                .unwrap();
                assert!(e.execute(&key, [3; 16], apply_command(&v), false).is_err());
            }
            _ => {
                fs::write(&f.path, b"independent external write").unwrap();
                assert!(e.execute(&key, [3; 16], apply_command(&v), false).is_err());
                assert_eq!(fs::read(&f.path).unwrap(), b"independent external write");
                continue;
            }
        }
        assert_eq!(fs::read(&f.path).unwrap(), original);
        assert_eq!(
            files::hash(
                &ownership::sibling(&f.path, ".backups")
                    .join(&f.copy)
                    .join("database.sqlite")
            )
            .unwrap(),
            f.original_backup
        );
    }
}

#[tokio::test]
async fn separate_readonly_authority_revocation_and_expected_versions_apply_to_replays() {
    let f = fixture().await;
    let (mut e, key) = engine(&f);
    let v = inspect(&mut e, &key);
    let read_key = random_key().unwrap();
    let issued = e
        .execute(
            &key,
            [10; 16],
            MaintenanceCommand::IssueAccess {
                label: "Агент".into(),
                verifier: key_verifier(&read_key).unwrap(),
                writable: false,
                expires_ms: None,
                expected: v.version,
            },
            false,
        )
        .unwrap()
        .result;
    let role = inspect(&mut e, &read_key);
    assert!(!role.owner && !role.writable);
    assert!(role.access.is_empty());
    assert!(
        e.execute(
            &read_key,
            [11; 16],
            MaintenanceCommand::Prepare {
                copy: f.copy.clone(),
                expected: role.version
            },
            false
        )
        .is_err()
    );
    assert!(
        e.execute(
            &key,
            [12; 16],
            MaintenanceCommand::Prepare {
                copy: f.copy.clone(),
                expected: v.version
            },
            false
        )
        .is_err()
    );
    e.execute(
        &key,
        [13; 16],
        MaintenanceCommand::RevokeAccess {
            subject: issued.issued_subject.unwrap(),
            expected: issued.version,
        },
        false,
    )
    .unwrap();
    assert!(
        e.execute(&read_key, [1; 16], MaintenanceCommand::Inspect {}, false)
            .is_err()
    );
    let old_key = f
        .old_key
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    assert!(
        e.execute(&old_key, [1; 16], MaintenanceCommand::Inspect {}, false)
            .is_err()
    );
}

#[tokio::test]
async fn rollback_restores_raw_bytes_keeps_gate_closed_and_allows_new_explicit_recovery() {
    let f = fixture().await;
    let (mut e, key) = engine(&f);
    let original = fs::read(&f.path).unwrap();
    let v = prepare(&mut e, &key, &f.copy);
    let restored = e
        .execute(&key, [3; 16], apply_command(&v), false)
        .unwrap()
        .result;
    let reverted = e
        .execute(
            &key,
            [4; 16],
            MaintenanceCommand::Rollback {
                plan: restored.plan.unwrap().id,
                expected: restored.version,
            },
            false,
        )
        .unwrap()
        .result;
    assert_eq!(fs::read(&f.path).unwrap(), original);
    assert_eq!(reverted.plan.unwrap().phase, RecoveryPhase::Reverted);
    assert!(Storage::open(&f.path).await.is_err());
    let v = inspect(&mut e, &key);
    assert!(
        e.release_for_retry(
            &key,
            [5; 16],
            &MaintenanceCommand::Retry {
                expected: v.version
            }
        )
        .is_err()
    );
    assert_eq!(fs::read(&f.path).unwrap(), original);
    let v = prepare(&mut e, &key, &f.copy);
    e.execute(&key, [6; 16], apply_command(&v), false).unwrap();
    drop(e);
    let db = Storage::open(&f.path).await.unwrap();
    db.shutdown().await.unwrap();
}

#[test]
fn crash_child() {
    let Some(path) = std::env::var_os("ZEDONE_EMERGENCY_TEST_DB") else {
        return;
    };
    let mut e = Emergency::open(PathBuf::from(path)).unwrap();
    let key = e.owner_session().unwrap().key;
    let command: MaintenanceCommand = read(&e.directory.join("test-command.json")).unwrap();
    let _ = e.execute(&key, [3; 16], command, false);
    panic!("crash point not reached");
}
#[tokio::test]
async fn interrupted_rollback_keeps_gate_closed_and_resumes_only_the_saved_command() {
    for point in ["rollback_intent", "rollback_file"] {
        let f = fixture().await;
        let (mut e, key) = engine(&f);
        let original = fs::read(&f.path).unwrap();
        let v = prepare(&mut e, &key, &f.copy);
        let restored = e
            .execute(&key, [2; 16], apply_command(&v), false)
            .unwrap()
            .result;
        let command = MaintenanceCommand::Rollback {
            plan: restored.plan.unwrap().id,
            expected: restored.version,
        };
        save(&e.directory.join("test-command.json"), &command).unwrap();
        drop(e);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "emergency::tests::crash_child", "--nocapture"])
            .env("ZEDONE_EMERGENCY_TEST_DB", &f.path)
            .env("ZEDONE_EMERGENCY_CRASH", point)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(74));
        let interrupted = fs::read(&f.path).unwrap();
        assert!(Storage::open(&f.path).await.is_err());
        assert_eq!(fs::read(&f.path).unwrap(), interrupted);
        let mut e = Emergency::open(f.path.clone()).unwrap();
        e.execute(&key, [3; 16], command.clone(), false).unwrap();
        assert_eq!(fs::read(&f.path).unwrap(), original);
        assert!(e.execute(&key, [3; 16], command, false).unwrap().replayed);
        drop(e);
        assert!(Storage::open(&f.path).await.is_err());
        assert_eq!(fs::read(&f.path).unwrap(), original);
    }
}
#[tokio::test]
async fn interrupted_replace_remains_closed_until_exact_same_explicit_command_and_never_replays_on_open()
 {
    for point in [
        "apply_intent",
        "sidecars_removed",
        "database_replaced",
        "identity_published",
    ] {
        let f = fixture().await;
        let (mut e, key) = engine(&f);
        let v = prepare(&mut e, &key, &f.copy);
        let command = apply_command(&v);
        save(&e.directory.join("test-command.json"), &command).unwrap();
        drop(e);
        let status = std::process::Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "emergency::tests::crash_child", "--nocapture"])
            .env("ZEDONE_EMERGENCY_TEST_DB", &f.path)
            .env("ZEDONE_EMERGENCY_CRASH", point)
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(74));
        let bytes = fs::read(&f.path).unwrap();
        assert!(Storage::open(&f.path).await.is_err());
        assert_eq!(fs::read(&f.path).unwrap(), bytes);
        let mut e = Emergency::open(f.path.clone()).unwrap();
        e.execute(&key, [3; 16], command.clone(), false).unwrap();
        assert!(e.execute(&key, [3; 16], command, false).unwrap().replayed);
        drop(e);
        let db = Storage::open(&f.path).await.unwrap();
        db.shutdown().await.unwrap();
    }
}
