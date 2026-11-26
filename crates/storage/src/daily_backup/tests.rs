use super::*;
use crate::{Stage, access::PendingAccess};
use std::{
    fs,
    process::Command,
    sync::atomic::{AtomicUsize, Ordering},
};
use zedone_core::spaces::SpaceStore;
#[test]
fn actual_timezone_source_and_offset_dates_are_distinct_from_utc_dates() {
    assert!(local_day().unwrap() > 0);
    let time = "2026-11-26T22:10:00Z".parse::<jiff::Timestamp>().unwrap();
    let utc = calendar_day(time, jiff::tz::TimeZone::UTC).unwrap();
    let baku = calendar_day(time, jiff::tz::TimeZone::get("Asia/Baku").unwrap()).unwrap();
    assert_eq!(baku, utc + 1);
    let west = calendar_day(time, jiff::tz::TimeZone::get("America/New_York").unwrap()).unwrap();
    assert_eq!(west, utc);
}
#[test]
fn running_retry_uses_monotonic_time_while_restart_resumes_persisted_wait() {
    let mut id = [0; 16];
    getrandom::fill(&mut id).unwrap();
    let installation = id.iter().map(|b| format!("{b:02x}")).collect::<String>();
    let mut g = Global {
        installation: installation.clone(),
        folder_initialized: false,
        elapsed_ms: 0,
        wall_ms: 1000,
    };
    let start = Instant::now();
    assert_eq!(observed(&mut g, 1000, 10, start).unwrap().0.elapsed_ms, 0);
    assert_eq!(
        observed(
            &mut g,
            86_401_000,
            11,
            start + std::time::Duration::from_millis(299000)
        )
        .unwrap()
        .0
        .elapsed_ms,
        299000
    );
    assert_eq!(
        observed(
            &mut g,
            100,
            10,
            start + std::time::Duration::from_millis(300000)
        )
        .unwrap()
        .0
        .elapsed_ms,
        300000
    );
    ANCHORS.get().unwrap().lock().unwrap().remove(&installation);
    assert_eq!(
        observed(&mut g, 1100, 10, start).unwrap().0.elapsed_ms,
        301000
    );
    ANCHORS.get().unwrap().lock().unwrap().remove(&installation);
    assert_eq!(observed(&mut g, 0, 9, start).unwrap().0.elapsed_ms, 301000);
    ANCHORS.get().unwrap().lock().unwrap().remove(&installation);
}
const SPACE: &str = "00000000000000000000000000000001";
const OTHER: &str = "00000000000000000000000000000002";
async fn fixture() -> (tempfile::TempDir, Storage) {
    let dir = tempfile::tempdir().unwrap();
    let db = Storage::open(dir.path().join("source.db")).await.unwrap();
    db.transact(|ctx| {
        let admin = PendingAccess::generate()?;
        ctx.bootstrap_access(&admin)?;
        ctx.create_space(SPACE, "Мысли ё🙂", admin.subject())?;
        ctx.create_space(OTHER, "Не менять", admin.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    db.initialize_daily_backups().await.unwrap();
    (dir, db)
}
async fn attempt_at(db: &Storage, day: i32, ms: u64, trigger: Trigger) -> DailyBackups {
    db.transact(move |ctx| {
        attempt(ctx.connection(), SPACE, trigger, &|g| {
            g.elapsed_ms = ms;
            g.wall_ms = ms;
            Ok((
                Moment {
                    local_day: day,
                    elapsed_ms: ms,
                },
                ms,
            ))
        })
    })
    .await
    .unwrap()
}
async fn rename(db: &Storage, name: &str) {
    let name = name.to_owned();
    db.transact(move |ctx| {
        ctx.rename_space(SPACE, &name)?;
        Ok(())
    })
    .await
    .unwrap();
}
async fn saved(db: &Storage) -> State {
    db.read(|ctx| state(ctx.connection(), SPACE)).await.unwrap()
}
async fn root(db: &Storage) -> PathBuf {
    db.read(|ctx| files::folder(ctx.connection()))
        .await
        .unwrap()
}
#[tokio::test]
async fn daily_files_are_full_verified_and_do_not_change_the_source_or_other_space() {
    let (_dir, db) = fixture().await;
    let before = db
        .export_snapshot(SPACE.into(), 0, LIMITS)
        .await
        .unwrap()
        .encode(LIMITS)
        .unwrap();
    let other = db
        .export_snapshot(OTHER.into(), 0, LIMITS)
        .await
        .unwrap()
        .encode(LIMITS)
        .unwrap();
    assert_eq!(
        attempt_at(&db, 10, 10, Trigger::Automatic).await.status,
        DailyStatus::Saved
    );
    let saved = saved(&db).await;
    let reference = saved.files[0].clone();
    let bytes = db
        .read(move |ctx| files::read(ctx.connection(), &reference))
        .await
        .unwrap();
    assert_eq!(
        crate::snapshot::SpaceSnapshot::decode(&bytes, LIMITS)
            .unwrap()
            .metadata()
            .space,
        SPACE
    );
    assert_eq!(
        before,
        db.export_snapshot(SPACE.into(), 0, LIMITS)
            .await
            .unwrap()
            .encode(LIMITS)
            .unwrap()
    );
    assert_eq!(
        other,
        db.export_snapshot(OTHER.into(), 0, LIMITS)
            .await
            .unwrap()
            .encode(LIMITS)
            .unwrap()
    );
    db.cleanup_daily_backups().await.unwrap();
    assert!(
        root(&db)
            .await
            .join(&saved.files[0].id)
            .join("commit.json")
            .is_file()
    );
    assert_eq!(
        attempt_at(&db, 11, 1000, Trigger::Automatic).await.status,
        DailyStatus::Unchanged
    );
    db.shutdown().await.unwrap();
}
#[tokio::test]
async fn seven_daily_copies_rotate_only_after_committed_success_and_explicit_deletion_keeps_limit()
{
    let (_dir, db) = fixture().await;
    for day in 1..=9 {
        rename(&db, &format!("День {day}")).await;
        assert_eq!(
            attempt_at(&db, day, day as u64 * 1000, Trigger::Automatic)
                .await
                .status,
            DailyStatus::Saved
        );
        let saved = saved(&db).await;
        assert_eq!(saved.files.len(), (day as usize).min(7));
        if day > 7 {
            assert!(root(&db).await.join(&saved.obsolete[0].id).is_dir());
        }
        db.cleanup_daily_backups().await.unwrap();
        assert_eq!(
            fs::read_dir(root(&db).await).unwrap().count(),
            (day as usize).min(7)
        );
    }
    let latest = saved(&db).await.files.last().unwrap().id.clone();
    db.transact(move |ctx| {
        let mut s = state(ctx.connection(), SPACE)?;
        let i = s.files.iter().position(|r| r.id == latest).unwrap();
        s.obsolete.push(s.files.remove(i));
        save(ctx.connection(), &state_key(SPACE), &s)
    })
    .await
    .unwrap();
    db.cleanup_daily_backups().await.unwrap();
    rename(&db, "После удаления").await;
    assert_eq!(
        attempt_at(&db, 9, 10000, Trigger::Automatic).await.status,
        DailyStatus::DailyLimit
    );
    assert_eq!(saved(&db).await.files.len(), 6);
}
#[tokio::test]
async fn unavailable_folder_preserves_old_files_and_failed_retry_sets_a_new_five_minute_pause() {
    let (dir, db) = fixture().await;
    attempt_at(&db, 1, 0, Trigger::Automatic).await;
    db.cleanup_daily_backups().await.unwrap();
    let folder = root(&db).await;
    let away = dir.path().join("saved-folder");
    fs::rename(&folder, &away).unwrap();
    rename(&db, "Позже").await;
    let failed = attempt_at(&db, 2, 1000, Trigger::Automatic).await;
    assert_eq!(failed.status, DailyStatus::Unavailable);
    assert_eq!(failed.copies.len(), 1);
    assert_eq!(
        attempt_at(&db, 2, 300_000, Trigger::Automatic)
            .await
            .retry_after_ms,
        Some(1000)
    );
    let failed_again = attempt_at(
        &db,
        2,
        300_001,
        Trigger::Retry {
            failed_sequence: failed.failed_sequence.unwrap(),
        },
    )
    .await;
    assert_eq!(failed_again.status, DailyStatus::Unavailable);
    fs::rename(away, &folder).unwrap();
    assert_eq!(
        attempt_at(&db, 2, 301_000, Trigger::Automatic).await.status,
        DailyStatus::Waiting
    );
    let success = attempt_at(
        &db,
        2,
        301_001,
        Trigger::Retry {
            failed_sequence: failed_again.failed_sequence.unwrap(),
        },
    )
    .await;
    assert_eq!(success.status, DailyStatus::Saved);
    assert!(success.failed_sequence.is_none());
    assert_eq!(fs::read_dir(&folder).unwrap().count(), 2);
}
#[tokio::test]
async fn a_crash_before_or_after_commit_recovers_without_losing_prior_files_or_repeating_success() {
    for commit in [false, true] {
        let (dir, db) = fixture().await;
        attempt_at(&db, 1, 0, Trigger::Automatic).await;
        db.cleanup_daily_backups().await.unwrap();
        rename(&db, "Следующий день").await;
        let folder = root(&db).await;
        db.shutdown().await.unwrap();
        let status = Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "daily_backup::tests::daily_crash_child",
                "--nocapture",
            ])
            .env("ZEDONE_DAILY_CHILD", dir.path())
            .env("ZEDONE_DAILY_COMMIT", if commit { "yes" } else { "no" })
            .status()
            .unwrap();
        assert_eq!(status.code(), Some(87));
        assert_eq!(fs::read_dir(&folder).unwrap().count(), 2);
        let db = Storage::open(dir.path().join("source.db")).await.unwrap();
        db.cleanup_daily_backups().await.unwrap();
        assert_eq!(saved(&db).await.files.len(), if commit { 2 } else { 1 });
        assert_eq!(
            fs::read_dir(&folder).unwrap().count(),
            if commit { 2 } else { 1 }
        );
        let result = attempt_at(&db, 2, 2001, Trigger::Automatic).await;
        assert_eq!(
            result.status,
            if commit {
                DailyStatus::Unchanged
            } else {
                DailyStatus::Saved
            }
        );
        db.shutdown().await.unwrap();
    }
}
#[tokio::test]
async fn daily_crash_child() {
    let Some(dir) = std::env::var_os("ZEDONE_DAILY_CHILD") else {
        return;
    };
    let db = Storage::open(PathBuf::from(dir).join("source.db"))
        .await
        .unwrap();
    let committed = std::env::var("ZEDONE_DAILY_COMMIT").unwrap() == "yes";
    let _ = db
        .transact_checked(
            |ctx| {
                attempt(ctx.connection(), SPACE, Trigger::Automatic, &|g| {
                    g.elapsed_ms = 2000;
                    g.wall_ms = 2000;
                    Ok((
                        Moment {
                            local_day: 2,
                            elapsed_ms: 2000,
                        },
                        2000,
                    ))
                })
            },
            move |phase| {
                if phase
                    == if committed {
                        Stage::AfterCommit
                    } else {
                        Stage::BeforeCommit
                    }
                {
                    std::process::exit(87)
                }
                Ok(())
            },
        )
        .await;
    panic!("missing crash checkpoint");
}
#[tokio::test]
async fn completion_date_and_capture_boundary_survive_midnight_and_future_edits() {
    let (_dir, db) = fixture().await;
    let result = db
        .transact(|ctx| {
            let calls = AtomicUsize::new(0);
            attempt(ctx.connection(), SPACE, Trigger::Automatic, &|g| {
                let n = calls.fetch_add(1, Ordering::SeqCst);
                g.elapsed_ms = n as u64;
                g.wall_ms = n as u64;
                Ok((
                    Moment {
                        local_day: if n == 0 { 10 } else { 11 },
                        elapsed_ms: n as u64,
                    },
                    n as u64,
                ))
            })
        })
        .await
        .unwrap();
    assert_eq!(result.status, DailyStatus::Saved);
    rename(&db, "Во время записи / после границы").await;
    assert_eq!(
        attempt_at(&db, 11, 100, Trigger::Automatic).await.status,
        DailyStatus::DailyLimit
    );
    assert_eq!(
        attempt_at(&db, 12, 101, Trigger::Automatic).await.status,
        DailyStatus::Saved
    );
}
#[tokio::test]
async fn rollback_retains_old_rotation_candidates_and_partial_commit_marker_is_repaired() {
    let (_dir, db) = fixture().await;
    for day in 1..=7 {
        rename(&db, &format!("{day}")).await;
        attempt_at(&db, day, day as u64, Trigger::Automatic).await;
        db.cleanup_daily_backups().await.unwrap();
    }
    rename(&db, "8").await;
    let result = db
        .transact(|ctx| {
            attempt_checked(
                ctx.connection(),
                SPACE,
                Trigger::Automatic,
                &|g| {
                    g.elapsed_ms = 8;
                    g.wall_ms = 8;
                    Ok((
                        Moment {
                            local_day: 8,
                            elapsed_ms: 8,
                        },
                        8,
                    ))
                },
                |_| Err(unavailable()),
            )
        })
        .await;
    assert!(result.is_err());
    assert_eq!(saved(&db).await.files.len(), 7);
    db.cleanup_daily_backups().await.unwrap();
    assert_eq!(fs::read_dir(root(&db).await).unwrap().count(), 7);
    let file = saved(&db).await.files[0].id.clone();
    fs::write(root(&db).await.join(file).join("commit.json"), b"{").unwrap();
    db.cleanup_daily_backups().await.unwrap();
}
#[tokio::test]
async fn corrupted_copy_and_foreign_files_never_trigger_old_snapshot_deletion() {
    let (_dir, db) = fixture().await;
    for day in 1..=8 {
        rename(&db, &format!("{day}")).await;
        attempt_at(&db, day, day as u64, Trigger::Automatic).await;
        if day < 8 {
            db.cleanup_daily_backups().await.unwrap();
        }
    }
    let s = saved(&db).await;
    let root = root(&db).await;
    fs::write(root.join("manual-export.bin"), b"foreign").unwrap();
    fs::write(root.join(&s.files[6].id).join("space.bin"), b"corrupt").unwrap();
    assert!(db.cleanup_daily_backups().await.is_err());
    assert!(root.join(&s.obsolete[0].id).is_dir());
    assert_eq!(
        fs::read(root.join("manual-export.bin")).unwrap(),
        b"foreign"
    );
}
#[tokio::test]
async fn same_database_parallel_attempts_have_one_daily_success() {
    let (dir, first) = fixture().await;
    let second = Storage::open(dir.path().join("source.db")).await.unwrap();
    let (a, b) = tokio::join!(
        attempt_at(&first, 1, 0, Trigger::Automatic),
        attempt_at(&second, 1, 0, Trigger::Automatic)
    );
    assert_eq!(
        [a.status, b.status]
            .iter()
            .filter(|&&s| s == DailyStatus::Saved)
            .count(),
        1
    );
    assert_eq!(saved(&first).await.files.len(), 1);
    second.shutdown().await.unwrap();
    first.shutdown().await.unwrap();
}

#[tokio::test]
async fn full_product_archive_keeps_pending_history_trash_tags_conflicts_and_access_outside_the_file()
 {
    let (_dir, db, _) = crate::restore_backup::tests::fixture().await;
    db.initialize_daily_backups().await.unwrap();
    let before = db
        .read(|ctx| {
            let mut q = ctx
                .connection()
                .prepare("SELECT hash,bytes FROM changes WHERE space=?1 ORDER BY hash")?;
            Ok(q.query_map([SPACE], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?)
        })
        .await
        .unwrap();
    let expected = db.export_snapshot(SPACE.into(), 50, LIMITS).await.unwrap();
    assert_eq!(
        attempt_at(&db, 1, 50, Trigger::Automatic).await.status,
        DailyStatus::Saved
    );
    let s = saved(&db).await;
    let r = s.files[0].clone();
    let actual = db
        .read(move |ctx| files::read(ctx.connection(), &r))
        .await
        .unwrap();
    let archive = crate::snapshot::SpaceSnapshot::decode(&actual, LIMITS).unwrap();
    assert_eq!(archive.changes().len(), expected.changes().len());
    let mut graph = automerge::Automerge::new();
    graph
        .apply_changes(archive.changes().iter().cloned())
        .unwrap();
    let mut original = automerge::Automerge::new();
    original
        .apply_changes(expected.changes().iter().cloned())
        .unwrap();
    for card in [4, 5] {
        let card = format!("{card:032x}");
        assert_eq!(
            crate::cards::history(&graph, &crate::cards::object(&graph, &card).unwrap()).unwrap(),
            crate::cards::history(&original, &crate::cards::object(&original, &card).unwrap())
                .unwrap()
        );
    }
    assert_eq!(
        crate::spaces::domain_projections(&graph).unwrap(),
        crate::spaces::domain_projections(&original).unwrap()
    );
    let after = db
        .read(|ctx| {
            let mut q = ctx
                .connection()
                .prepare("SELECT hash,bytes FROM changes WHERE space=?1 ORDER BY hash")?;
            Ok(q.query_map([SPACE], |r| {
                Ok((r.get::<_, Vec<u8>>(0)?, r.get::<_, Vec<u8>>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?)
        })
        .await
        .unwrap();
    assert_eq!(before, after);
    let forged = db
        .transact(|ctx| {
            let mut g = global(ctx.connection())?;
            let foreign = files::capture(ctx.connection(), &mut g, OTHER, 51)?;
            let mut state = state(ctx.connection(), SPACE)?;
            let mut wrong = foreign.clone();
            wrong.space = SPACE.into();
            state.obsolete.push(wrong);
            save(ctx.connection(), &state_key(SPACE), &state)?;
            Ok(foreign.id)
        })
        .await
        .unwrap();
    assert!(db.cleanup_daily_backups().await.is_err());
    assert!(root(&db).await.join(forged).join("space.bin").is_file());
    db.shutdown().await.unwrap();
}
