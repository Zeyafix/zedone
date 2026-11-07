use super::*;
use automerge::ActorId;
use zedone_core::columns::ColumnStore;
use zedone_core::spaces::SpaceState;

#[tokio::test]
async fn create_rename_and_assignment_roll_back_or_replay_at_transaction_boundaries() {
    for action in ["create", "rename", "assign"] {
        for checkpoint in [
            Stage::AfterWork,
            Stage::AfterChanges,
            Stage::AfterResult,
            Stage::BeforeCommit,
            Stage::AfterCommit,
        ] {
            let (_dir, s, a) = setup().await;
            let mut t = None;
            if action != "create" {
                t = Some(
                    run!(s, a, core::create, create(1, 5, "Work"))
                        .unwrap()
                        .result,
                );
            }
            let before = card(&s, 3).await;
            let expected = before.version;
            let version = t.as_ref().map(|t| t.version);
            let b = a.clone();
            let result = s
                .transact_checked(
                    move |ctx| {
                        match action {
                            "create" => {
                                core::create(ctx, &b.credential(), &Now, &create(2, 5, "Work"))?;
                            }
                            "rename" => {
                                core::rename(
                                    ctx,
                                    &b.credential(),
                                    &Now,
                                    &rename(2, 5, "New", version.unwrap()),
                                )?;
                            }
                            _ => {
                                core::set_card_tag(
                                    ctx,
                                    &b.credential(),
                                    &Now,
                                    &set(2, 3, 5, true, expected, version.unwrap()),
                                )?;
                            }
                        }
                        Ok(())
                    },
                    move |at| {
                        if at == checkpoint {
                            Err(StorageError::Unavailable)
                        } else {
                            Ok(())
                        }
                    },
                )
                .await;
            assert_eq!(
                result.unwrap_err(),
                if checkpoint == Stage::AfterCommit {
                    StorageError::OutcomeUnknown
                } else {
                    StorageError::Unavailable
                }
            );
            if checkpoint != Stage::AfterCommit {
                assert_eq!(card(&s, 3).await, before);
                if let Some(t) = t {
                    assert_eq!(tag(&s, 5).await, t);
                } else {
                    rejected(
                        run!(
                            s,
                            a,
                            core::read,
                            req(
                                3,
                                ReadTag {
                                    space: sid(1),
                                    tag: sid(5)
                                }
                            )
                        ),
                        ErrorCode::NotFound,
                    );
                }
            }
            let replayed = match action {
                "create" => {
                    run!(s, a, core::create, create(2, 5, "Work"))
                        .unwrap()
                        .replayed
                }
                "rename" => {
                    run!(s, a, core::rename, rename(2, 5, "New", version.unwrap()))
                        .unwrap()
                        .replayed
                }
                _ => {
                    run!(
                        s,
                        a,
                        core::set_card_tag,
                        set(2, 3, 5, true, expected, version.unwrap())
                    )
                    .unwrap()
                    .replayed
                }
            };
            assert_eq!(replayed, checkpoint == Stage::AfterCommit);
        }
    }
}

#[tokio::test]
async fn expired_and_revoked_writers_cannot_assign_or_replay_and_trash_blocks_new_assignments() {
    let (_dir, s, a) = setup().await;
    let t = run!(s, a, core::create, create(1, 5, "Work"))
        .unwrap()
        .result;
    let c = card(&s, 3).await;
    let expired = Arc::new(PendingAccess::generate().unwrap());
    let writer = Arc::new(PendingAccess::generate().unwrap());
    let (b, e, w) = (a.clone(), expired.clone(), writer.clone());
    s.transact(move |ctx| {
        for (key, expiry) in [
            (&e, Some(UNIX_EPOCH + Duration::from_millis(101))),
            (&w, None),
        ] {
            ctx.issue_access(
                &b.credential(),
                &Now,
                key,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::ReadWrite)])),
                expiry,
            )?;
        }
        Ok(())
    })
    .await
    .unwrap();
    let e = expired.clone();
    let expected = c.version;
    let expected_tag = t.version;
    rejected(
        s.timed_transact(move |ctx, clock| {
            Ok(core::set_card_tag(
                ctx,
                &e.credential(),
                clock,
                &set(2, 3, 5, true, expected, expected_tag),
            )?)
        })
        .await,
        ErrorCode::AccessDenied,
    );
    run!(
        s,
        writer,
        core::set_card_tag,
        set(2, 3, 5, true, c.version, t.version)
    )
    .unwrap();
    let (b, w) = (a.clone(), writer.clone());
    s.transact(move |ctx| {
        ctx.revoke_access(&b.credential(), &Now, w.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        run!(
            s,
            writer,
            core::set_card_tag,
            set(2, 3, 5, true, c.version, t.version)
        ),
        ErrorCode::AccessDenied,
    );
    let c = card(&s, 3).await;
    let trashed = run!(
        s,
        a,
        cards::trash,
        req(
            3,
            TrashCard {
                space: sid(1),
                card: sid(3),
                expected: c.version
            }
        )
    )
    .unwrap()
    .result;
    let t = tag(&s, 5).await;
    rejected(
        run!(
            s,
            a,
            core::set_card_tag,
            set(4, 3, 5, false, trashed.version, t.version)
        ),
        ErrorCode::InvalidRequest,
    );
    assert_eq!(card(&s, 3).await, trashed);
    for parent in ["column", "space"] {
        s.transact(move |ctx| {
            if parent == "column" {
                ctx.set_column_lifecycle(
                    &sid(1),
                    &sid(2),
                    &format!("trash:{}:{}", sid(7), sid(8)),
                )?;
            } else {
                ctx.set_space_lifecycle(&sid(1), &format!("trash:{}:{}", sid(7), sid(8)))?;
            }
            Ok(())
        })
        .await
        .unwrap();
        let c = card(&s, 4).await;
        let t = tag(&s, 5).await;
        rejected(
            run!(
                s,
                a,
                core::set_card_tag,
                set(5, 4, 5, true, c.version, t.version)
            ),
            ErrorCode::InvalidRequest,
        );
    }
}

#[tokio::test]
async fn canonical_unicode_collisions_and_free_rename_preserve_identity_and_assignments() {
    for (first, equivalent) in [
        ("Café", "Cafe\u{301}"),
        ("Straße", "STRASSE"),
        ("ΟΣ", "ος"),
        ("Й", "И\u{306}"),
    ] {
        let (_dir, s, a) = setup().await;
        let t = run!(s, a, core::create, create(1, 5, first))
            .unwrap()
            .result;
        let c = card(&s, 3).await;
        run!(
            s,
            a,
            core::set_card_tag,
            set(2, 3, 5, true, c.version, t.version)
        )
        .unwrap();
        rejected(
            run!(s, a, core::create, create(3, 6, equivalent)),
            ErrorCode::Conflict,
        );
        run!(s, a, core::create, create(4, 6, "Other")).unwrap();
        let other = tag(&s, 6).await;
        rejected(
            run!(s, a, core::rename, rename(5, 6, equivalent, other.version)),
            ErrorCode::Conflict,
        );
        let t = tag(&s, 5).await;
        let renamed = run!(s, a, core::rename, rename(6, 5, equivalent, t.version))
            .unwrap()
            .result;
        assert_eq!(renamed.names, [equivalent]);
        assert_eq!(renamed.tag, sid(5));
        assert_eq!(card(&s, 3).await.tags, [sid(5)]);
    }
    let (_dir, s, a) = setup().await;
    let t = run!(s, a, core::create, create(1, 5, "Все"))
        .unwrap()
        .result;
    let renamed = run!(s, a, core::rename, rename(2, 5, "Всё", t.version))
        .unwrap()
        .result;
    assert_eq!(renamed.names, ["Всё"]);
    assert_eq!(renamed.tag, t.tag);
}

#[tokio::test]
async fn different_assignments_merge_but_removal_against_reassignment_is_a_conflict() {
    for conflict in [false, true] {
        let (_dir, s, a) = setup().await;
        run!(s, a, core::create, create(1, 5, "First")).unwrap();
        run!(s, a, core::create, create(2, 6, "Second")).unwrap();
        if conflict {
            let c = card(&s, 3).await;
            let t = tag(&s, 5).await;
            run!(
                s,
                a,
                core::set_card_tag,
                set(3, 3, 5, true, c.version, t.version)
            )
            .unwrap();
        }
        s.transact(move |ctx| {
            let graph = ctx.read().document(&sid(1))?;
            let card = crate::cards::object(graph, &sid(3))?;
            let map = assignments(graph, &card)?.0;
            let mut left = graph.fork().with_actor(ActorId::from(vec![31]));
            let mut right = graph.fork().with_actor(ActorId::from(vec![32]));
            if conflict {
                // A real re-assignment needs a causal return; Automerge elides
                // assigning an already equal scalar in one branch.
                let mut tx = right.transaction();
                tx.put(&map, sid(5), false)?;
                tx.commit();
            }
            let mut tx = left.transaction();
            tx.put(&map, sid(5), !conflict)?;
            tx.commit();
            let mut tx = right.transaction();
            tx.put(&map, sid(if conflict { 5 } else { 6 }), true)?;
            tx.commit();
            let mut changes = left.get_changes(&[]);
            changes.extend(right.get_changes(&[]));
            changes.reverse();
            let duplicate = changes.clone();
            changes.extend(duplicate);
            ctx.apply_changes(&sid(1), changes)
        })
        .await
        .unwrap();
        let c = card(&s, 3).await;
        if conflict {
            assert_eq!(c.state, SpaceState::Conflicted);
            rejected(
                run!(
                    s,
                    a,
                    cards::edit,
                    req(
                        4,
                        EditCard {
                            space: sid(1),
                            card: sid(3),
                            title: "Overwrite".into(),
                            description: "".into(),
                            expected: c.version
                        }
                    )
                ),
                ErrorCode::Conflict,
            );
            let t = tag(&s, 5).await;
            rejected(
                run!(s, a, core::delete, delete(5, 5, t.version)),
                ErrorCode::Conflict,
            );
        } else {
            assert_eq!(c.state, SpaceState::Active);
            assert_eq!(c.tags, [sid(5), sid(6)]);
        }
    }
}

#[tokio::test]
async fn deleted_tag_against_unseen_assignment_and_name_collision_fail_closed() {
    let (_dir, s, a) = setup().await;
    let t = run!(s, a, core::create, create(1, 5, "Work"))
        .unwrap()
        .result;
    let changes = s
        .transact(|ctx| {
            let graph = ctx.read().document(&sid(1))?;
            let card = crate::cards::object(graph, &sid(3))?;
            let map = assignments(graph, &card)?.0;
            let mut remote = graph.fork().with_actor(ActorId::from(vec![41]));
            let mut tx = remote.transaction();
            tx.put(&map, sid(5), true)?;
            tx.commit();
            Ok(remote.get_changes(&[]))
        })
        .await
        .unwrap();
    run!(s, a, core::delete, delete(2, 5, t.version)).unwrap();
    s.transact(move |ctx| ctx.apply_changes(&sid(1), changes))
        .await
        .unwrap();
    assert_eq!(tag(&s, 5).await.state, TagState::Conflicted);
    let c = card(&s, 3).await;
    assert_eq!(c.tags, [sid(5)]);
    assert_eq!(c.state, SpaceState::Conflicted);
    let t = tag(&s, 5).await;
    rejected(
        run!(
            s,
            a,
            core::set_card_tag,
            set(3, 3, 5, false, c.version, t.version)
        ),
        ErrorCode::Conflict,
    );
    let (_dir, s, a) = setup().await;
    run!(s, a, core::create, create(1, 5, "Work")).unwrap();
    let c = card(&s, 3).await;
    let t = tag(&s, 5).await;
    run!(
        s,
        a,
        core::set_card_tag,
        set(2, 3, 5, true, c.version, t.version)
    )
    .unwrap();
    run!(s, a, core::create, create(4, 6, "Other")).unwrap();
    s.transact(|ctx| {
        ctx.rename_tag(&sid(1), &sid(6), " WORK ")?;
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(tag(&s, 6).await.state, TagState::Conflicted);
    let c = card(&s, 3).await;
    assert_eq!(c.state, SpaceState::Active);
    run!(
        s,
        a,
        cards::edit,
        req(
            5,
            EditCard {
                space: sid(1),
                card: sid(3),
                title: "Allowed".into(),
                description: "".into(),
                expected: c.version
            }
        )
    )
    .unwrap();
    let c = card(&s, 3).await;
    let t = tag(&s, 5).await;
    rejected(
        run!(
            s,
            a,
            core::set_card_tag,
            set(6, 3, 5, false, c.version, t.version)
        ),
        ErrorCode::Conflict,
    );
}

#[tokio::test]
async fn deletion_sql_abort_cannot_remove_only_some_assignments() {
    let (_dir, s, a) = setup().await;
    run!(s, a, core::create, create(1, 5, "Work")).unwrap();
    for (id, cid) in [(2, 3), (3, 4)] {
        let c = card(&s, cid).await;
        let t = tag(&s, 5).await;
        run!(
            s,
            a,
            core::set_card_tag,
            set(id, cid, 5, true, c.version, t.version)
        )
        .unwrap();
    }
    let t = tag(&s, 5).await;
    let before = [card(&s, 3).await, card(&s, 4).await];
    s.transact(|ctx| {ctx.connection().execute_batch("CREATE TRIGGER reject_tag_receipt BEFORE INSERT ON command_results BEGIN SELECT RAISE(ABORT, 'injected'); END;")?;Ok(())}).await.unwrap();
    rejected(
        run!(s, a, core::delete, delete(4, 5, t.version)),
        ErrorCode::Unavailable,
    );
    assert_eq!([card(&s, 3).await, card(&s, 4).await], before);
    assert_eq!(tag(&s, 5).await, t);
    s.transact(|ctx| {
        ctx.connection()
            .execute_batch("DROP TRIGGER reject_tag_receipt")?;
        Ok(())
    })
    .await
    .unwrap();
    assert!(
        !run!(s, a, core::delete, delete(4, 5, t.version))
            .unwrap()
            .replayed
    );
}

#[tokio::test]
async fn concurrent_assignment_and_deletion_have_one_serializable_winner() {
    let (dir, s, a) = setup().await;
    let other = Storage::open(dir.path().join("tags.db")).await.unwrap();
    let t = run!(s, a, core::create, create(1, 5, "Work"))
        .unwrap()
        .result;
    let c = card(&s, 3).await;
    let (b, d) = (a.clone(), a.clone());
    let (assigned, deleted) = tokio::join!(
        s.transact(move |ctx| Ok(core::set_card_tag(
            ctx,
            &b.credential(),
            &Now,
            &set(2, 3, 5, true, c.version, t.version)
        )?)),
        other.transact(move |ctx| Ok(core::delete(
            ctx,
            &d.credential(),
            &Now,
            &delete(3, 5, t.version)
        )?))
    );
    assert_ne!(assigned.is_ok(), deleted.is_ok());
    if assigned.is_ok() {
        rejected(deleted, ErrorCode::StaleState);
        assert_eq!(card(&s, 3).await.tags, [sid(5)]);
    } else {
        rejected(assigned, ErrorCode::StaleState);
        assert!(card(&s, 3).await.tags.is_empty());
    }
    other.shutdown().await.unwrap();
}

#[tokio::test]
async fn v5_migration_preserves_legacy_graph_versions_and_receipts_and_blocks_old_writers() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("tags.db");
    let mut startup = crate::startup::Startup::new(&path);
    startup.registry = crate::schema::Registry(&crate::schema::REGISTRY.0[..5]);
    let s = startup.automatic(None).await.unwrap();
    let a = Arc::new(PendingAccess::generate().unwrap());
    let b = a.clone();
    let original = s
        .transact(move |ctx| {
            ctx.bootstrap_access(&b)?;
            Ok(spaces::create(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    1,
                    CreateSpace {
                        space: sid(1),
                        name: "Legacy".into(),
                    },
                ),
            )?)
        })
        .await
        .unwrap()
        .result;
    let bytes = s
        .transact(|ctx| {
            Ok(ctx
                .read()
                .document(&sid(1))?
                .get_changes(&[])
                .iter()
                .map(|c| c.raw_bytes().to_vec())
                .collect::<Vec<_>>())
        })
        .await
        .unwrap();
    s.shutdown().await.unwrap();
    assert!(Storage::open(&path).await.is_err());
    let s = crate::startup::Startup::new(&path)
        .automatic(Some(crate::startup::MigrationOptions {
            folder: dir.path().to_owned(),
            created_at_unix_ms: 10,
            limits: crate::migration_backup::tests::LIMITS,
        }))
        .await
        .unwrap();
    assert_eq!(
        s.transact(|ctx| Ok(ctx.space_snapshot(&sid(1))?))
            .await
            .unwrap(),
        original
    );
    assert_eq!(
        s.transact(|ctx| Ok(ctx
            .read()
            .document(&sid(1))?
            .get_changes(&[])
            .iter()
            .map(|c| c.raw_bytes().to_vec())
            .collect::<Vec<_>>()))
            .await
            .unwrap(),
        bytes
    );
    let replay = run!(
        s,
        a,
        spaces::create,
        req(
            1,
            CreateSpace {
                space: sid(1),
                name: "Legacy".into()
            }
        )
    )
    .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, original);
    run!(s, a, core::create, create(2, 5, "Work")).unwrap();
    s.shutdown().await.unwrap();
    let mut old = crate::startup::Startup::new(&path);
    old.registry = crate::schema::Registry(&crate::schema::REGISTRY.0[..5]);
    assert_eq!(
        old.automatic(None).await.err(),
        Some(StorageError::UnsupportedSchema)
    );
}
