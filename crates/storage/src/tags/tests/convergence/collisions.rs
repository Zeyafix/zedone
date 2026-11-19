use super::*;
use zedone_core::tag_names::{self, ResolveTagNames, TagNameChoice};

async fn collision(creation: bool) -> (TempDir, Storage, Storage, Arc<PendingAccess>) {
    let (dir, left, admin) = setup().await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    assign(&left, &admin, 2, 3, 5, true).await;
    if !creation {
        run!(left, admin, core::create, create(3, 6, "Личное")).unwrap();
        assign(&left, &admin, 4, 3, 6, true).await;
    }
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    let t = tag(&left, 5).await;
    run!(left, admin, core::rename, rename(5, 5, "Важное", t.version)).unwrap();
    if creation {
        run!(right, admin, core::create, create(6, 6, " ВАЖНОЕ ")).unwrap();
        assign(&right, &admin, 7, 3, 6, true).await;
    } else {
        let t = tag(&right, 6).await;
        run!(
            right,
            admin,
            core::rename,
            rename(6, 6, " ВАЖНОЕ ", t.version)
        )
        .unwrap();
    }
    exchange(&left, &right, creation).await;
    (dir, left, right, admin)
}

async fn decision(db: &Storage, request: u8, names: [&str; 2]) -> CommandRequest<ResolveTagNames> {
    let mut choices = Vec::new();
    for (id, name) in [5, 6].into_iter().zip(names) {
        choices.push(TagNameChoice {
            tag: sid(id),
            name: name.into(),
            expected: tag(db, id).await.version,
        });
    }
    req(
        request,
        ResolveTagNames {
            space: sid(1),
            tag: sid(5),
            names: choices,
        },
    )
}

#[tokio::test]
async fn rename_collisions_keep_ids_links_editing_blocks_and_atomic_unique_names() {
    for creation in [false, true] {
        let (dir, left, right, admin) = collision(creation).await;
        let before = card(&left, 3).await;
        assert_eq!(before.tags, [sid(5), sid(6)]);
        assert_eq!(before.state, SpaceState::Active);
        for id in [5, 6] {
            let t = tag(&left, id).await;
            assert_eq!(t.name_group, [sid(5), sid(6)]);
            assert_eq!(t.state, TagState::Conflicted);
            for present in [false, true] {
                rejected(
                    run!(
                        left,
                        admin,
                        core::set_card_tag,
                        set(10, 3, id, present, before.version, t.version)
                    ),
                    ErrorCode::Conflict,
                );
            }
        }
        let original = listing(&left).await;
        let mut incomplete = decision(&left, 11, ["Все", "Всё"]).await;
        incomplete.parameters.names.pop();
        rejected(
            run!(left, admin, tag_names::resolve, incomplete),
            ErrorCode::InvalidRequest,
        );
        rejected(
            run!(
                left,
                admin,
                tag_names::resolve,
                decision(&left, 12, ["Straße", " STRASSE "]).await
            ),
            ErrorCode::InvalidRequest,
        );
        assert_eq!(listing(&left).await, original);
        run!(left, admin, core::create, create(13, 7, "Остальное")).unwrap();
        assign(&left, &admin, 14, 3, 7, true).await;
        assign(&left, &admin, 15, 3, 7, false).await;
        let c = card(&left, 3).await;
        run!(
            left,
            admin,
            cards::edit,
            req(
                16,
                EditCard {
                    space: sid(1),
                    card: sid(3),
                    title: "Новый заголовок".into(),
                    description: "Новый текст".into(),
                    expected: c.version
                }
            )
        )
        .unwrap();
        let edited = card(&left, 3).await;
        let request = decision(&left, 17, ["Все", "Всё"]).await;
        let saved = serde_json::to_vec(&request).unwrap();
        let stale = decision(&left, 18, ["Другое", "Третье"]).await;
        run!(left, admin, tag_names::resolve, request).unwrap();
        rejected(
            run!(left, admin, tag_names::resolve, stale),
            ErrorCode::StaleState,
        );
        exchange(&left, &right, !creation).await;
        for db in [&left, &right] {
            assert_eq!(tag(db, 5).await.names, ["Все"]);
            assert_eq!(tag(db, 6).await.names, ["Всё"]);
            assert_eq!(tag(db, 5).await.state, TagState::Active);
            assert_eq!(tag(db, 6).await.state, TagState::Active);
            let c = card(db, 3).await;
            assert_eq!(c.titles, edited.titles);
            assert_eq!(c.descriptions, edited.descriptions);
            assert_eq!(c.tags, before.tags);
            assert_eq!(c.positions, before.positions);
        }
        assign(&left, &admin, 19, 3, 5, false).await;
        assign(&left, &admin, 20, 3, 5, true).await;
        let t = tag(&left, 5).await;
        run!(left, admin, core::rename, rename(21, 5, "Назад", t.version)).unwrap();
        let after = listing(&left).await;
        assert!(
            run!(
                left,
                admin,
                tag_names::resolve,
                serde_json::from_slice::<CommandRequest<ResolveTagNames>>(&saved).unwrap()
            )
            .unwrap()
            .replayed
        );
        assert_eq!(listing(&left).await, after);
        exchange(&left, &right, creation).await;
        right.shutdown().await.unwrap();
        let reopened = Storage::open(dir.path().join("right.db")).await.unwrap();
        assert_eq!(listing(&reopened).await, after);
    }
}

#[tokio::test]
async fn rename_collision_equal_different_and_late_decisions_keep_whole_group() {
    for creation in [false, true] {
        for same in [false, true] {
            let (_dir, left, right, admin) = collision(creation).await;
            run!(
                left,
                admin,
                tag_names::resolve,
                decision(&left, 30, ["Все", "Всё"]).await
            )
            .unwrap();
            run!(
                right,
                admin,
                tag_names::resolve,
                decision(
                    &right,
                    31,
                    if same {
                        ["Все", "Всё"]
                    } else {
                        ["Дом", "Дела"]
                    }
                )
                .await
            )
            .unwrap();
            if !same {
                let t = tag(&left, 5).await;
                run!(
                    left,
                    admin,
                    core::rename,
                    rename(32, 5, "Позднее", t.version)
                )
                .unwrap();
            }
            exchange(&left, &right, creation).await;
            assert_eq!(listing(&left).await, listing(&right).await);
            for id in [5, 6] {
                assert_eq!(
                    tag(&left, id).await.state,
                    if same {
                        TagState::Active
                    } else {
                        TagState::Conflicted
                    }
                );
                assert_eq!(tag(&left, id).await.name_group, [sid(5), sid(6)]);
            }
            if !same {
                run!(
                    left,
                    admin,
                    tag_names::resolve,
                    decision(&left, 33, ["Итог", "Другой итог"]).await
                )
                .unwrap();
                exchange(&left, &right, !creation).await;
                assert_eq!(tag(&right, 5).await.state, TagState::Active);
            }
            assert_eq!(card(&right, 3).await.tags, [sid(5), sid(6)]);
        }
    }
}

#[tokio::test]
async fn name_collision_resolution_writes_every_alias_and_preserves_assignments() {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    run!(right, admin, core::create, create(2, 7, "РАБОТА")).unwrap();
    assign(&left, &admin, 3, 3, 5, true).await;
    assign(&right, &admin, 4, 4, 7, true).await;
    exchange(&left, &right, true).await;
    let t = tag(&left, 7).await;
    run!(left, admin, core::rename, rename(5, 7, "Важное", t.version)).unwrap();
    run!(right, admin, core::create, create(6, 6, "ВАЖНОЕ")).unwrap();
    assign(&right, &admin, 7, 3, 6, true).await;
    exchange(&left, &right, false).await;
    assert_eq!(tag(&left, 7).await.name_group, [sid(5), sid(6)]);
    let mut request = decision(&left, 8, ["Все", "Всё"]).await;
    request.parameters.tag = sid(7);
    run!(left, admin, tag_names::resolve, request).unwrap();
    exchange(&left, &right, true).await;
    for db in [&left, &right] {
        assert_eq!(tag(db, 5).await, tag(db, 7).await);
        assert_eq!(tag(db, 7).await.names, ["Все"]);
        assert_eq!(tag(db, 7).await.state, TagState::Active);
        assert_eq!(card(db, 3).await.tags, [sid(5), sid(6)]);
        assert_eq!(card(db, 4).await.tags, [sid(5)]);
    }
}

#[tokio::test]
async fn deleted_bridge_ancestor_does_not_block_live_name_choices_or_return_old_links() {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    assign(&left, &admin, 2, 3, 5, true).await;
    let t = tag(&left, 5).await;
    run!(left, admin, core::rename, rename(3, 5, "Личное", t.version)).unwrap();
    let t = tag(&left, 5).await;
    run!(left, admin, core::delete, delete(4, 5, t.version)).unwrap();
    run!(left, admin, core::create, create(5, 6, "Работа")).unwrap();
    assign(&left, &admin, 6, 3, 6, true).await;
    run!(right, admin, core::create, create(7, 7, "РАБОТА")).unwrap();
    assign(&right, &admin, 8, 3, 7, true).await;
    exchange(&left, &right, true).await;
    assert_eq!(tag(&left, 5).await.state, TagState::Deleted);
    assert_eq!(card(&left, 3).await.tags, [sid(6), sid(7)]);
    let mut choices = Vec::new();
    for (id, name) in [(6, "Все"), (7, "Всё")] {
        choices.push(TagNameChoice {
            tag: sid(id),
            name: name.into(),
            expected: tag(&left, id).await.version,
        });
    }
    assert_eq!(tag(&left, 6).await.name_group, [sid(6), sid(7)]);
    run!(
        left,
        admin,
        tag_names::resolve,
        req(
            9,
            ResolveTagNames {
                space: sid(1),
                tag: sid(6),
                names: choices
            }
        )
    )
    .unwrap();
    exchange(&left, &right, false).await;
    for db in [&left, &right] {
        assert_eq!(tag(db, 5).await.state, TagState::Deleted);
        assert_eq!(tag(db, 6).await.state, TagState::Active);
        assert_eq!(tag(db, 7).await.state, TagState::Active);
        assert_eq!(card(db, 3).await.tags, [sid(6), sid(7)]);
    }
    right.shutdown().await.unwrap();
    let reopened = Storage::open(dir.path().join("right.db")).await.unwrap();
    assert_eq!(listing(&left).await, listing(&reopened).await);
}
