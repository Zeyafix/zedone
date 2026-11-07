use super::*;
use zedone_core::{
    card_history::{AutosaveCard, BeginCardEditing, FinishCardEditing, RestoreCardText},
    replay::StateVersion,
};

struct At(i64);
impl Clock for At {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_millis(self.0 as u64))
    }
}
macro_rules! at {
    ($storage:expr,$key:expr,$time:expr,$handler:ident,$request:expr) => {{
        let secret = $key.credential().0.to_vec();
        let request = $request;
        let time = $time;
        $storage
            .transact(move |ctx| {
                Ok(core::$handler(
                    ctx,
                    &Credential(&secret),
                    &At(time),
                    &request,
                )?)
            })
            .await
    }};
}
fn begin(id: u8, expected: StateVersion) -> CommandRequest<BeginCardEditing> {
    req(
        id,
        BeginCardEditing {
            space: sid(1),
            card: sid(9),
            expected,
        },
    )
}
fn session(id: u8) -> String {
    [id; 16].iter().map(|b| format!("{b:02x}")).collect()
}
fn save(
    id: u8,
    editing: u8,
    title: &str,
    description: &str,
    expected: StateVersion,
) -> CommandRequest<AutosaveCard> {
    req(
        id,
        AutosaveCard {
            space: sid(1),
            card: sid(9),
            session: session(editing),
            title: title.into(),
            description: description.into(),
            expected,
        },
    )
}
fn finish(id: u8, editing: u8, expected: StateVersion) -> CommandRequest<FinishCardEditing> {
    req(
        id,
        FinishCardEditing {
            space: sid(1),
            card: sid(9),
            session: session(editing),
            expected,
        },
    )
}
fn restore(id: u8, version: &str, expected: StateVersion) -> CommandRequest<RestoreCardText> {
    req(
        id,
        RestoreCardText {
            space: sid(1),
            card: sid(9),
            text_version: version.into(),
            expected,
        },
    )
}
async fn history_at(storage: &Storage, admin: &PendingAccess, id: u8, time: i64) -> CardHistory {
    at!(
        storage,
        admin,
        time,
        history,
        req(
            id,
            ReadCardHistory {
                space: sid(1),
                card: sid(9),
                after: None,
                limit: 100,
                expected: None
            }
        )
    )
    .unwrap()
    .result
}

#[tokio::test]
async fn thirty_second_boundary_uses_only_changed_pairs_and_never_extends_on_read_noop_or_replay() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
    let one = at!(
        storage,
        admin,
        1_000,
        autosave,
        save(3, 2, "one", "d1", first.version)
    )
    .unwrap()
    .result;
    assert_eq!(
        history_at(&storage, &admin, 4, 30_000).await.versions.len(),
        1
    );
    let two = at!(
        storage,
        admin,
        30_999,
        autosave,
        save(5, 2, "two", "d2", one.version)
    )
    .unwrap()
    .result;
    assert_eq!(
        history_at(&storage, &admin, 6, 60_000).await.versions.len(),
        1
    );
    let noop = at!(
        storage,
        admin,
        60_500,
        autosave,
        save(7, 2, "two", "d2", two.version)
    )
    .unwrap()
    .result;
    assert_eq!(noop, two);
    let replay = at!(
        storage,
        admin,
        60_998,
        autosave,
        save(5, 2, "two", "d2", one.version)
    )
    .unwrap();
    assert!(replay.replayed);
    let closed = history_at(&storage, &admin, 8, 60_999).await;
    assert_eq!(closed.versions.len(), 2);
    assert_eq!(
        (&closed.versions[1].title, &closed.versions[1].description),
        (&"two".into(), &"d2".into())
    );
    assert_eq!(
        closed.versions[1].id,
        format!("{}:{}", admin.subject(), session(3))
    );
    assert_eq!(fresh(&storage, &admin, 9, 9).await, two);
    assert_eq!(history_at(&storage, &admin, 10, 9_000_000).await, closed);
    let next = at!(
        storage,
        admin,
        9_000_001,
        autosave,
        save(11, 2, "three", "d3", two.version)
    )
    .unwrap()
    .result;
    run!(storage, admin, finish_editing, finish(12, 2, next.version)).unwrap();
    assert_eq!(versions(&storage, 9).await.len(), 3);
    assert!(
        at!(
            storage,
            admin,
            10_000_000,
            finish_editing,
            finish(12, 2, next.version)
        )
        .unwrap()
        .replayed
    );
    rejected(
        at!(
            storage,
            admin,
            10_000_001,
            autosave,
            save(13, 2, "old", "", next.version)
        ),
        ErrorCode::StaleState,
    );
}

#[tokio::test]
async fn overdue_autosave_starts_another_series_and_empty_or_stale_commands_cannot_close_it() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
    let one = at!(
        storage,
        admin,
        1_000,
        autosave,
        save(3, 2, "one", "d1", first.version)
    )
    .unwrap()
    .result;
    rejected(
        at!(
            storage,
            admin,
            40_000,
            autosave,
            save(4, 2, " \n", "invalid", one.version)
        ),
        ErrorCode::InvalidRequest,
    );
    rejected(
        at!(
            storage,
            admin,
            40_000,
            autosave,
            save(5, 2, "stale", "", first.version)
        ),
        ErrorCode::StaleState,
    );
    assert_eq!(versions(&storage, 9).await.len(), 1);
    let two = at!(
        storage,
        admin,
        40_000,
        autosave,
        save(6, 2, "two", "d2", one.version)
    )
    .unwrap()
    .result;
    assert_eq!(versions(&storage, 9).await[1].title, "one");
    run!(storage, admin, finish_editing, finish(7, 2, two.version)).unwrap();
    assert_eq!(
        versions(&storage, 9)
            .await
            .iter()
            .map(|v| v.title.as_str())
            .collect::<Vec<_>>(),
        vec![first.titles[0].as_str(), "one", "two"]
    );
}

#[tokio::test]
async fn participants_discrete_commands_and_client_restart_preserve_each_stage_without_empty_versions()
 {
    let (_dir, storage, admin) = setup().await;
    let agent = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    let g = agent.clone();
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &Now,
                &g,
                &GrantScope::Spaces([(sid(1), SpacePermission::ReadWrite)].into()),
                None,
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
    let one = at!(
        storage,
        admin,
        1_000,
        autosave,
        save(3, 2, "human", "h", first.version)
    )
    .unwrap()
    .result;
    // A read by another participant is not an editing boundary.
    assert_eq!(fresh(&storage, &agent, 4, 9).await, one);
    assert_eq!(versions(&storage, 9).await.len(), 1);
    let two = run!(
        storage,
        agent,
        edit,
        edit_req(5, 9, "agent", "a", one.version)
    )
    .unwrap()
    .result;
    let three = run!(
        storage,
        agent,
        edit,
        edit_req(6, 9, "agent2", "a2", two.version)
    )
    .unwrap()
    .result;
    assert_eq!(versions(&storage, 9).await.len(), 4);
    let four = at!(
        storage,
        admin,
        2_000,
        autosave,
        save(7, 2, "human2", "h2", three.version)
    )
    .unwrap()
    .result;
    let old_history = versions(&storage, 9).await;
    // A new tab/app client explicitly begins before showing history or saving.
    run!(storage, admin, begin_editing, begin(8, four.version)).unwrap();
    let v = versions(&storage, 9).await;
    assert_eq!(v.len(), 5);
    assert_eq!(v[1].subject, admin.subject());
    assert_eq!(v[2].subject, agent.subject());
    assert_eq!(v[4].title, "human2");
    assert_eq!(&v[..4], old_history);
    rejected(
        at!(
            storage,
            admin,
            2_001,
            autosave,
            save(9, 2, "old tab", "", four.version)
        ),
        ErrorCode::StaleState,
    );
    assert!(
        run!(storage, admin, begin_editing, begin(2, first.version))
            .unwrap()
            .replayed
    );
    assert_eq!(versions(&storage, 9).await, v);
    run!(storage, admin, finish_editing, finish(10, 8, four.version)).unwrap();
    assert_eq!(versions(&storage, 9).await, v);
}

#[tokio::test]
async fn restore_pair_is_a_separate_version_even_if_equal_and_can_be_reversed_without_losing_history()
 {
    let (dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let second = run!(
        storage,
        admin,
        edit,
        edit_req(2, 9, "two", "d2", first.version)
    )
    .unwrap()
    .result;
    let third = run!(
        storage,
        admin,
        edit,
        edit_req(3, 9, "three", "d3", second.version)
    )
    .unwrap()
    .result;
    let original = versions(&storage, 9).await;
    let fourth = run!(
        storage,
        admin,
        restore_text,
        restore(4, &original[0].id, third.version)
    )
    .unwrap()
    .result;
    assert_eq!(fourth.titles, first.titles);
    assert_eq!(fourth.descriptions, first.descriptions);
    let fifth = run!(
        storage,
        admin,
        restore_text,
        restore(5, &original[2].id, fourth.version)
    )
    .unwrap()
    .result;
    assert_eq!(fifth.titles, third.titles);
    assert_eq!(fifth.descriptions, third.descriptions);
    let same = run!(
        storage,
        admin,
        restore_text,
        restore(6, &original[2].id, fifth.version)
    )
    .unwrap()
    .result;
    assert_eq!(same.version, fifth.version); // Same current pair; the new visible history version is independent.
    let all = versions(&storage, 9).await;
    assert_eq!(all.len(), 6);
    assert_eq!(&all[..3], original);
    assert_eq!(
        (all[3].title.clone(), all[3].description.clone()),
        (original[0].title.clone(), original[0].description.clone())
    );
    assert!(
        run!(
            storage,
            admin,
            restore_text,
            restore(4, &original[0].id, third.version)
        )
        .unwrap()
        .replayed
    );
    assert_eq!(fresh(&storage, &admin, 7, 9).await, same);
    rejected(
        run!(
            storage,
            admin,
            restore_text,
            restore(8, &original[0].id, third.version)
        ),
        ErrorCode::StaleState,
    );
    rejected(
        run!(
            storage,
            admin,
            restore_text,
            restore(9, &format!("{}:{}", sid(99), sid(99)), same.version)
        ),
        ErrorCode::NotFound,
    );
    storage.shutdown().await.unwrap();
    let reopened = Storage::open(dir.path().join("cards.db")).await.unwrap();
    reopened.recover_editing_sessions().await.unwrap();
    assert_eq!(versions(&reopened, 9).await, all);
    assert_eq!(fresh(&reopened, &admin, 10, 9).await, same);
}

#[tokio::test]
async fn recovery_finishes_saved_series_once_without_overwriting_newer_current_text() {
    let (dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
    let one = at!(
        storage,
        admin,
        1_000,
        autosave,
        save(3, 2, "one", "d1", first.version)
    )
    .unwrap()
    .result;
    at!(
        storage,
        admin,
        2_000,
        autosave,
        save(4, 2, "two", "d2", one.version)
    )
    .unwrap();
    // A later imported/current change must survive recovery of the older pair.
    storage
        .transact(|ctx| {
            let obj = object(ctx.read().document(&sid(1))?, &sid(9))?;
            ctx.edit(&sid(1), |doc| {
                doc.put(&obj, "title", "newer")?;
                doc.put(&obj, "description", "newer description")?;
                Ok(())
            })
        })
        .await
        .unwrap();
    let current = fresh(&storage, &admin, 5, 9).await;
    storage.shutdown().await.unwrap();
    let reopened = Storage::open(dir.path().join("cards.db")).await.unwrap();
    for _ in 0..3 {
        reopened.recover_editing_sessions().await.unwrap();
    }
    let v = versions(&reopened, 9).await;
    assert_eq!(v.len(), 2);
    assert_eq!(v[1].title, "two");
    assert_eq!(v[1].description, "d2");
    assert_eq!(fresh(&reopened, &admin, 6, 9).await, current);
    assert!(
        at!(
            reopened,
            admin,
            3_000,
            autosave,
            save(4, 2, "two", "d2", one.version)
        )
        .unwrap()
        .replayed
    );
    rejected(
        at!(
            reopened,
            admin,
            3_000,
            autosave,
            save(7, 2, "old session", "", current.version)
        ),
        ErrorCode::StaleState,
    );
    run!(reopened, admin, begin_editing, begin(8, current.version)).unwrap();
    let next = at!(
        reopened,
        admin,
        3_001,
        autosave,
        save(9, 8, "after restart", "d3", current.version)
    )
    .unwrap()
    .result;
    run!(reopened, admin, finish_editing, finish(10, 8, next.version)).unwrap();
    assert_eq!(versions(&reopened, 9).await.len(), 3);
}

#[tokio::test]
async fn series_recovery_rolls_back_at_every_boundary_and_unknown_commit_does_not_duplicate() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
    let saved = at!(
        storage,
        admin,
        1_000,
        autosave,
        save(3, 2, "saved", "pair", first.version)
    )
    .unwrap()
    .result;
    for checkpoint in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let error = storage
            .transact_checked(
                |ctx| ctx.recover_text_sessions(),
                move |stage| {
                    if stage == checkpoint {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await
            .unwrap_err();
        assert_eq!(
            error,
            if checkpoint == Stage::AfterCommit {
                StorageError::OutcomeUnknown
            } else {
                StorageError::Unavailable
            }
        );
        assert_eq!(
            versions(&storage, 9).await.len(),
            if checkpoint == Stage::AfterCommit {
                2
            } else {
                1
            }
        );
        assert_eq!(
            fresh(&storage, &admin, 40 + checkpoint as u8, 9).await,
            saved
        );
    }
    storage.recover_editing_sessions().await.unwrap();
    assert_eq!(versions(&storage, 9).await.len(), 2);
}

#[tokio::test]
async fn autosave_and_restoration_rollback_text_pending_history_and_receipt_together() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let initial = versions(&storage, 9).await[0].id.clone();
    run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
    for checkpoint in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
    ] {
        let a = admin.clone();
        assert_eq!(
            storage
                .transact_checked(
                    move |ctx| Ok(core::autosave(
                        ctx,
                        &a.credential(),
                        &At(1_000),
                        &save(3, 2, "pending", "pair", first.version)
                    )?),
                    move |stage| if stage == checkpoint {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                )
                .await
                .unwrap_err(),
            StorageError::Unavailable
        );
        assert_eq!(
            fresh(&storage, &admin, 40 + checkpoint as u8, 9).await,
            first
        );
        assert_eq!(versions(&storage, 9).await.len(), 1);
    }
    let saved = at!(
        storage,
        admin,
        1_000,
        autosave,
        save(3, 2, "pending", "pair", first.version)
    )
    .unwrap()
    .result;
    storage.transact(|ctx|{ctx.connection().execute_batch("CREATE TRIGGER fail_history_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;Ok(())}).await.unwrap();
    rejected(
        run!(
            storage,
            admin,
            restore_text,
            restore(4, &initial, saved.version)
        ),
        ErrorCode::Unavailable,
    );
    assert_eq!(versions(&storage, 9).await.len(), 1);
    storage
        .transact(|ctx| {
            ctx.connection()
                .execute_batch("DROP TRIGGER fail_history_receipt")?;
            Ok(())
        })
        .await
        .unwrap();
    let a = admin.clone();
    let id = initial.clone();
    assert_eq!(
        storage
            .transact_checked(
                move |ctx| Ok(core::restore_text(
                    ctx,
                    &a.credential(),
                    &Now,
                    &restore(4, &id, saved.version)
                )?),
                |stage| if stage == Stage::AfterCommit {
                    Err(StorageError::Unavailable)
                } else {
                    Ok(())
                }
            )
            .await
            .unwrap_err(),
        StorageError::OutcomeUnknown
    );
    let repeated = run!(
        storage,
        admin,
        restore_text,
        restore(4, &initial, saved.version)
    )
    .unwrap();
    assert!(repeated.replayed);
    assert_eq!(repeated.result.titles, first.titles);
    let v = versions(&storage, 9).await;
    assert_eq!(v.len(), 3);
    assert_eq!(v[1].title, "pending");
    storage.recover_editing_sessions().await.unwrap();
    assert_eq!(versions(&storage, 9).await, v);
}

#[test]
fn history_crash_child() {
    let Ok(path) = std::env::var("ZEDONE_HISTORY_CRASH_DB") else {
        return;
    };
    let point = std::env::var("ZEDONE_HISTORY_CRASH_POINT").unwrap();
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async move {
            let storage = Storage::open(path).await.unwrap();
            storage
                .transact_checked(
                    |ctx| ctx.recover_text_sessions(),
                    move |stage| {
                        if format!("{stage:?}") == point {
                            std::process::exit(73);
                        }
                        Ok(())
                    },
                )
                .await
                .unwrap();
            panic!("crash point not reached");
        });
}

#[tokio::test]
async fn real_process_exit_during_series_recovery_preserves_one_version_and_fresh_session() {
    for point in ["AfterWork", "AfterChanges", "BeforeCommit", "AfterCommit"] {
        let (dir, storage, admin) = setup().await;
        let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
        run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
        let saved = at!(
            storage,
            admin,
            1_000,
            autosave,
            save(3, 2, "saved before crash", "pair", first.version)
        )
        .unwrap()
        .result;
        storage.shutdown().await.unwrap();
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "cards::tests::history::history_crash_child",
                "--nocapture",
            ])
            .env("ZEDONE_HISTORY_CRASH_DB", dir.path().join("cards.db"))
            .env("ZEDONE_HISTORY_CRASH_POINT", point)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(73),
            "{point}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let reopened = Storage::open(dir.path().join("cards.db")).await.unwrap();
        for _ in 0..2 {
            reopened.recover_editing_sessions().await.unwrap();
        }
        assert_eq!(fresh(&reopened, &admin, 4, 9).await, saved);
        let v = versions(&reopened, 9).await;
        assert_eq!(v.len(), 2);
        assert_eq!(v[1].title, "saved before crash");
        run!(reopened, admin, begin_editing, begin(5, saved.version)).unwrap();
        let next = at!(
            reopened,
            admin,
            1_001,
            autosave,
            save(6, 5, "after crash", "next pair", saved.version)
        )
        .unwrap()
        .result;
        run!(reopened, admin, finish_editing, finish(7, 5, next.version)).unwrap();
        assert_eq!(versions(&reopened, 9).await.len(), 3);
    }
}

#[tokio::test]
async fn history_mutations_require_current_permissions_and_sessions_are_bound_to_subjects() {
    let (_dir, storage, admin) = setup().await;
    let reader = Arc::new(PendingAccess::generate().unwrap());
    let writer = Arc::new(PendingAccess::generate().unwrap());
    let (a, r, w) = (admin.clone(), reader.clone(), writer.clone());
    storage
        .transact(move |ctx| {
            ctx.issue_access(
                &a.credential(),
                &Now,
                &r,
                &GrantScope::Spaces([(sid(1), SpacePermission::Read)].into()),
                None,
            )?;
            ctx.issue_access(
                &a.credential(),
                &Now,
                &w,
                &GrantScope::Spaces([(sid(1), SpacePermission::ReadWrite)].into()),
                None,
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let initial = versions(&storage, 9).await[0].id.clone();
    rejected(
        run!(storage, reader, begin_editing, begin(2, first.version)),
        ErrorCode::AccessDenied,
    );
    run!(storage, writer, begin_editing, begin(2, first.version)).unwrap();
    rejected(
        at!(
            storage,
            admin,
            1_000,
            autosave,
            save(3, 2, "forged owner", "", first.version)
        ),
        ErrorCode::StaleState,
    );
    rejected(
        at!(
            storage,
            reader,
            1_000,
            autosave,
            save(3, 2, "forbidden", "", first.version)
        ),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(storage, reader, finish_editing, finish(4, 2, first.version)),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(
            storage,
            reader,
            restore_text,
            restore(5, &initial, first.version)
        ),
        ErrorCode::AccessDenied,
    );
    let saved = at!(
        storage,
        writer,
        1_000,
        autosave,
        save(3, 2, "writer", "pair", first.version)
    )
    .unwrap()
    .result;
    let (a, w) = (admin.clone(), writer.clone());
    storage
        .transact(move |ctx| ctx.revoke_access(&a.credential(), &Now, w.subject()))
        .await
        .unwrap();
    rejected(
        at!(
            storage,
            writer,
            2_000,
            autosave,
            save(3, 2, "writer", "pair", first.version)
        ),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(storage, writer, begin_editing, begin(2, first.version)),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(storage, writer, finish_editing, finish(4, 2, saved.version)),
        ErrorCode::AccessDenied,
    );
    let history = history_at(&storage, &reader, 5, 31_000).await;
    assert_eq!(history.versions.len(), 2);
    assert_eq!(history.versions[1].subject, writer.subject());
    assert_eq!(fresh(&storage, &admin, 6, 9).await, saved);
}

#[tokio::test]
async fn restoration_checks_all_card_fields_and_trash_finalizes_without_changing_pair() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let initial = versions(&storage, 9).await[0].id.clone();
    run!(storage, admin, begin_editing, begin(2, first.version)).unwrap();
    let saved = at!(
        storage,
        admin,
        1_000,
        autosave,
        save(3, 2, "pending", "pair", first.version)
    )
    .unwrap()
    .result;
    let completed = run!(
        storage,
        admin,
        set_completed,
        completion::request(4, true, saved.version)
    )
    .unwrap()
    .result;
    rejected(
        run!(
            storage,
            admin,
            restore_text,
            restore(5, &initial, saved.version)
        ),
        ErrorCode::StaleState,
    );
    assert_eq!(versions(&storage, 9).await.len(), 1);
    let trashed = run!(
        storage,
        admin,
        trash,
        req(
            6,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: completed.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(versions(&storage, 9).await.len(), 2);
    rejected(
        run!(
            storage,
            admin,
            restore_text,
            restore(7, &initial, trashed.version)
        ),
        ErrorCode::InvalidRequest,
    );
    let active = run!(
        storage,
        admin,
        restore,
        req(
            8,
            RestoreCard {
                space: sid(1),
                card: sid(9),
                expected: trashed.version
            }
        )
    )
    .unwrap()
    .result;
    rejected(
        at!(
            storage,
            admin,
            1_001,
            autosave,
            save(9, 2, "old", "", active.version)
        ),
        ErrorCode::StaleState,
    );
    let restored = run!(
        storage,
        admin,
        restore_text,
        restore(10, &initial, active.version)
    )
    .unwrap()
    .result;
    assert_eq!(restored.titles, first.titles);
    assert_eq!(restored.completed, [true]);
    assert_eq!(restored.columns, active.columns);
    assert_eq!(restored.positions, active.positions);
    assert_eq!(versions(&storage, 9).await.len(), 3);
}
