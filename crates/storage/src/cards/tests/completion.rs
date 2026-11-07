use super::*;
use zedone_core::{cards::SetCardCompleted, replay::StateVersion};

pub(super) fn request(
    id: u8,
    completed: bool,
    expected: StateVersion,
) -> CommandRequest<SetCardCompleted> {
    req(
        id,
        SetCardCompleted {
            space: sid(1),
            card: sid(9),
            completed,
            expected,
        },
    )
}

#[tokio::test]
async fn completion_is_reversible_in_each_column_and_preserves_every_other_field() {
    let (dir, storage, admin) = setup().await;
    let mut before = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let history = versions(&storage, 9).await;
    for (index, column) in [2, 3].into_iter().enumerate() {
        if column == 3 {
            // Moving is P04.05. Trusted fixture verifies independence now.
            storage
                .transact(|ctx| {
                    let obj = object(ctx.read().document(&sid(1))?, &sid(9))?;
                    ctx.edit(&sid(1), |doc| {
                        doc.put(&obj, "column", sid(3))?;
                        Ok(())
                    })?;
                    Ok(())
                })
                .await
                .unwrap();
            before = fresh(&storage, &admin, 20, 9).await;
        }
        let base = 2 + index as u8 * 5;
        let no_change = run!(
            storage,
            admin,
            set_completed,
            request(base, false, before.version)
        )
        .unwrap();
        assert!(!no_change.replayed);
        assert_eq!(no_change.result, before);
        let completed = run!(
            storage,
            admin,
            set_completed,
            request(base + 1, true, before.version)
        )
        .unwrap()
        .result;
        let mut expected = before.clone();
        expected.completed = vec![true];
        expected.version = completed.version;
        assert_eq!(completed, expected);
        assert_ne!(completed.version, before.version);
        rejected(
            run!(
                storage,
                admin,
                edit,
                edit_req(base + 2, 9, "stale", "", before.version)
            ),
            ErrorCode::StaleState,
        );
        let undone = run!(
            storage,
            admin,
            set_completed,
            request(base + 3, false, completed.version)
        )
        .unwrap()
        .result;
        expected.completed = vec![false];
        expected.version = undone.version;
        assert_eq!(undone, expected);
        assert_ne!(undone.version, before.version); // ABA never revives old commands.
        rejected(
            run!(
                storage,
                admin,
                set_completed,
                request(base + 4, true, before.version)
            ),
            ErrorCode::StaleState,
        );
        let replay = run!(
            storage,
            admin,
            set_completed,
            request(base + 1, true, before.version)
        )
        .unwrap();
        assert!(replay.replayed);
        assert_eq!(replay.result, completed);
        assert_eq!(fresh(&storage, &admin, 21 + index as u8, 9).await, undone);
        before = undone;
    }
    assert_eq!(versions(&storage, 9).await, history);
    let marked = run!(
        storage,
        admin,
        set_completed,
        request(31, true, before.version)
    )
    .unwrap()
    .result;
    storage.shutdown().await.unwrap();
    let reopened = Storage::open(dir.path().join("cards.db")).await.unwrap();
    assert_eq!(fresh(&reopened, &admin, 30, 9).await, marked);
    assert!(
        run!(
            reopened,
            admin,
            set_completed,
            request(31, true, before.version)
        )
        .unwrap()
        .replayed
    );
    let undone = run!(
        reopened,
        admin,
        set_completed,
        request(32, false, marked.version)
    )
    .unwrap()
    .result;
    assert_eq!(undone.completed, [false]);
    assert_eq!(undone.columns, before.columns);
    assert_eq!(versions(&reopened, 9).await, history);
}

#[tokio::test]
async fn new_description_rejects_old_completion_but_read_history_and_other_card_do_not() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let edited = run!(
        storage,
        admin,
        edit,
        edit_req(
            2,
            9,
            &first.titles[0],
            "New unfinished steps",
            first.version
        )
    )
    .unwrap()
    .result;
    for completed in [true, false] {
        rejected(
            run!(
                storage,
                admin,
                set_completed,
                request(3 + u8::from(completed), completed, first.version)
            ),
            ErrorCode::StaleState,
        );
    }
    assert_eq!(fresh(&storage, &admin, 5, 9).await, edited);
    let history = versions(&storage, 9).await;
    run!(storage, admin, create, creation(6, 8)).unwrap();
    let done = run!(
        storage,
        admin,
        set_completed,
        request(7, true, edited.version)
    )
    .unwrap()
    .result;
    assert_eq!(done.descriptions, ["New unfinished steps"]);
    assert_eq!(done.completed, [true]);
    assert_eq!(versions(&storage, 9).await, history);
    // The same command ID cannot be reused for a different value.
    rejected(
        run!(
            storage,
            admin,
            set_completed,
            request(7, false, edited.version)
        ),
        ErrorCode::CommandMismatch,
    );
    let trashed = run!(
        storage,
        admin,
        trash,
        req(
            8,
            TrashCard {
                space: sid(1),
                card: sid(9),
                expected: done.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(trashed.completed, [true]);
    rejected(
        run!(
            storage,
            admin,
            set_completed,
            request(9, false, trashed.version)
        ),
        ErrorCode::InvalidRequest,
    );
    let restored = run!(
        storage,
        admin,
        restore,
        req(
            10,
            RestoreCard {
                column: None,
                expected_column: None,
                space: sid(1),
                card: sid(9),
                expected: trashed.version
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(restored.completed, [true]);
    assert_eq!(versions(&storage, 9).await, history);
}

#[tokio::test]
async fn completion_rolls_back_at_every_precommit_boundary_and_replays_after_lost_commit_reply() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
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
                    move |ctx| Ok(core::set_completed(
                        ctx,
                        &a.credential(),
                        &Now,
                        &request(2, true, first.version)
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
        assert_eq!(fresh(&storage, &admin, 3, 9).await, first);
    }
    storage.transact(|ctx| {ctx.connection().execute_batch("CREATE TRIGGER fail_completion_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?; Ok(())}).await.unwrap();
    rejected(
        run!(
            storage,
            admin,
            set_completed,
            request(2, true, first.version)
        ),
        ErrorCode::Unavailable,
    );
    storage
        .transact(|ctx| {
            ctx.connection()
                .execute_batch("DROP TRIGGER fail_completion_receipt")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(fresh(&storage, &admin, 4, 9).await, first);
    let a = admin.clone();
    assert_eq!(
        storage
            .transact_checked(
                move |ctx| Ok(core::set_completed(
                    ctx,
                    &a.credential(),
                    &Now,
                    &request(2, true, first.version)
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
    let retry = run!(
        storage,
        admin,
        set_completed,
        request(2, true, first.version)
    )
    .unwrap();
    assert!(retry.replayed);
    assert_eq!(retry.result.completed, [true]);
    assert_eq!(versions(&storage, 9).await.len(), 1);
}

#[tokio::test]
async fn concurrent_completion_and_text_edit_cannot_both_use_the_same_version() {
    let (dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let other = Storage::open(dir.path().join("cards.db")).await.unwrap();
    let (a, b) = (admin.clone(), admin.clone());
    let (one, two) = tokio::join!(
        storage.transact(move |ctx| Ok(core::set_completed(
            ctx,
            &a.credential(),
            &Now,
            &request(2, true, first.version)
        )?)),
        other.transact(move |ctx| Ok(core::edit(
            ctx,
            &b.credential(),
            &Now,
            &edit_req(3, 9, "Concurrent", "", first.version)
        )?))
    );
    assert_ne!(one.is_ok(), two.is_ok());
    let (winner, loser) = if one.is_ok() {
        (one.unwrap(), two)
    } else {
        (two.unwrap(), one)
    };
    rejected(loser, ErrorCode::StaleState);
    assert_eq!(fresh(&storage, &admin, 4, 9).await, winner.result);
}

#[tokio::test]
async fn completion_requires_current_write_access_even_for_noop_and_replay() {
    let (_dir, storage, admin) = setup().await;
    let first = run!(storage, admin, create, creation(1, 9)).unwrap().result;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let expired = Arc::new(PendingAccess::generate().unwrap());
    let readonly = Arc::new(PendingAccess::generate().unwrap());
    let (a, m, e, r) = (
        admin.clone(),
        member.clone(),
        expired.clone(),
        readonly.clone(),
    );
    storage
        .transact(move |ctx| {
            for (key, permission, expires) in [
                (&m, SpacePermission::ReadWrite, None),
                (
                    &e,
                    SpacePermission::ReadWrite,
                    Some(UNIX_EPOCH + Duration::from_millis(101)),
                ),
                (&r, SpacePermission::Read, None),
            ] {
                ctx.issue_access(
                    &a.credential(),
                    &Now,
                    key,
                    &GrantScope::Spaces(BTreeMap::from([(sid(1), permission)])),
                    expires,
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    for completed in [true, false] {
        rejected(
            run!(
                storage,
                readonly,
                set_completed,
                request(2, completed, first.version)
            ),
            ErrorCode::AccessDenied,
        );
    }
    let e = expired.clone();
    rejected(
        storage
            .timed_transact(move |ctx, clock| {
                Ok(core::set_completed(
                    ctx,
                    &e.credential(),
                    clock,
                    &request(2, true, first.version),
                )?)
            })
            .await,
        ErrorCode::AccessDenied,
    );
    let completed = run!(
        storage,
        member,
        set_completed,
        request(2, true, first.version)
    )
    .unwrap()
    .result;
    let (a, m) = (admin.clone(), member.clone());
    storage
        .transact(move |ctx| {
            ctx.revoke_access(&a.credential(), &Now, m.subject())?;
            Ok(())
        })
        .await
        .unwrap();
    rejected(
        run!(
            storage,
            member,
            set_completed,
            request(2, true, first.version)
        ),
        ErrorCode::AccessDenied,
    );
    assert_eq!(fresh(&storage, &admin, 3, 9).await, completed);
    for parent in ["column", "space"] {
        storage
            .transact(move |ctx| {
                let lifecycle = format!("trash:{}:{}", sid(7), sid(8));
                if parent == "column" {
                    ctx.set_column_lifecycle(&sid(1), &sid(2), &lifecycle)?;
                } else {
                    ctx.set_column_lifecycle(&sid(1), &sid(2), "active")?;
                    ctx.set_space_lifecycle(&sid(1), &lifecycle)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        rejected(
            run!(
                storage,
                admin,
                set_completed,
                request(4, false, completed.version)
            ),
            ErrorCode::InvalidRequest,
        );
    }
}
