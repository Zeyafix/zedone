use super::*;
use zedone_core::{spaces::SpaceState, sync::SyncStore};
mod collisions;
mod deletion;
mod names;
mod resolution;

async fn copy(db: &Storage, path: std::path::PathBuf, admin: &Arc<PendingAccess>) -> Storage {
    let page = db
        .transact(|tx| crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![]))
        .await
        .unwrap();
    let remote = Storage::open(path).await.unwrap();
    let owner = admin.clone();
    remote
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let selection = tx.read().sync_selection(&sid(1), &sid(99))?;
            tx.set_sync_selected(&sid(1), &sid(99), true, selection.version)?;
            tx.receive_sync_page(&sid(1), &sid(99), owner.subject(), &page)
        })
        .await
        .unwrap();
    remote
}

async fn deliver(from: &Storage, to: &Storage, reverse: bool) {
    let mut page = from
        .transact(|tx| crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![]))
        .await
        .unwrap();
    if reverse {
        page.changes.reverse();
    }
    to.transact(move |tx| {
        for _ in 0..2 {
            tx.push_sync(&sid(1), "test", &vec![], &page.changes)?;
        }
        Ok(())
    })
    .await
    .unwrap();
}

async fn exchange(left: &Storage, right: &Storage, reverse: bool) {
    deliver(left, right, reverse).await;
    deliver(right, left, !reverse).await;
}

async fn assign(
    db: &Storage,
    admin: &PendingAccess,
    request: u8,
    card_id: u8,
    tag_id: u8,
    present: bool,
) {
    let c = card(db, card_id).await;
    let t = tag(db, tag_id).await;
    run!(
        db,
        admin,
        core::set_card_tag,
        set(request, card_id, tag_id, present, c.version, t.version)
    )
    .unwrap();
}

async fn listing(db: &Storage) -> Vec<TagSnapshot> {
    db.transact(|tx| tx.tags(&sid(1)).map_err(Into::into))
        .await
        .unwrap()
}

#[tokio::test]
async fn independent_creations_converge_preserve_links_spelling_aliases_and_restart() {
    for (left_name, right_name, merged) in [
        ("РАБОТА", "Работа", true),
        ("Всё", "\u{a0}ВСЁ\u{2003}", true),
        ("Все", "Всё", false),
        ("Café", "Cafe\u{301}", true),
        ("Straße", "STRASSE", true),
    ] {
        for reverse in [false, true] {
            let (dir, left, admin) = setup().await;
            let right = copy(&left, dir.path().join("right.db"), &admin).await;
            run!(left, admin, core::create, create(1, 5, left_name)).unwrap();
            run!(right, admin, core::create, create(2, 6, right_name)).unwrap();
            assign(&left, &admin, 3, 3, 5, true).await;
            assign(&right, &admin, 4, 4, 6, true).await;
            assign(&right, &admin, 5, 3, 6, true).await;
            let before = card(&left, 3).await;
            exchange(&left, &right, reverse).await;
            let tags = listing(&left).await;
            assert_eq!(tags, listing(&right).await);
            assert_eq!(tags.len(), if merged { 1 } else { 2 });
            assert!(tags.iter().all(|tag| tag.state == TagState::Active));
            assert!(tags.iter().all(|tag| tag.names.len() == 1
                && [left_name, right_name].contains(&tag.names[0].as_str())));
            for db in [&left, &right] {
                let c = card(db, 3).await;
                assert_eq!(c.state, SpaceState::Active);
                assert_eq!(
                    c.tags,
                    if merged {
                        vec![sid(5)]
                    } else {
                        vec![sid(5), sid(6)]
                    }
                );
                assert_eq!(c.titles, before.titles);
                assert_eq!(c.descriptions, before.descriptions);
                assert_eq!(c.columns, before.columns);
                assert_eq!(c.completed, before.completed);
                assert_eq!(card(db, 4).await.tags, [sid(if merged { 5 } else { 6 })]);
                if merged {
                    assert_eq!(tag(db, 5).await, tag(db, 6).await);
                }
            }
            let snapshot = left
                .transact(|tx| Ok(tx.space_snapshot(&sid(1))?))
                .await
                .unwrap();
            assert_eq!(snapshot.contents.unwrap().tags, tags);
            right.shutdown().await.unwrap();
            let right = Storage::open(dir.path().join("right.db")).await.unwrap();
            assert_eq!(listing(&right).await, tags);
            assert_eq!(card(&right, 3).await, card(&left, 3).await);
        }
    }
}

#[tokio::test]
async fn alias_rename_remove_replay_delete_and_recreate_do_not_resurrect_old_links() {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(left, admin, core::create, create(1, 5, "РАБОТА")).unwrap();
    run!(right, admin, core::create, create(2, 6, "Работа")).unwrap();
    assign(&left, &admin, 3, 3, 5, true).await;
    assign(&right, &admin, 4, 4, 6, true).await;
    exchange(&left, &right, true).await;
    let c = card(&left, 3).await;
    let t = tag(&left, 6).await;
    let original_card_version = c.version;
    let present = set(5, 3, 6, true, c.version, t.version);
    run!(left, admin, core::set_card_tag, present).unwrap();
    assign(&left, &admin, 6, 3, 6, false).await;
    run!(
        left,
        admin,
        core::set_card_tag,
        set(5, 3, 6, true, original_card_version, t.version)
    )
    .unwrap();
    assert!(card(&left, 3).await.tags.is_empty());
    let old = tag(&left, 6).await;
    let renamed = run!(
        left,
        admin,
        core::rename,
        rename(7, 6, "работа", old.version)
    )
    .unwrap()
    .result;
    assert_eq!(renamed.names, ["работа"]);
    assert_eq!(renamed.tag, sid(5));
    let renamed = run!(
        left,
        admin,
        core::rename,
        rename(8, 6, "Личное", renamed.version)
    )
    .unwrap()
    .result;
    rejected(
        run!(
            left,
            admin,
            core::rename,
            rename(9, 5, "Stale", old.version)
        ),
        ErrorCode::StaleState,
    );
    deliver(&right, &left, true).await;
    assert_eq!(tag(&left, 5).await, renamed);
    assert!(card(&left, 3).await.tags.is_empty());
    assert_eq!(card(&left, 4).await.tags, [sid(5)]);
    exchange(&left, &right, false).await;
    assert_eq!(listing(&left).await, listing(&right).await);
    assign(&right, &admin, 10, 3, 6, true).await;
    exchange(&left, &right, true).await;
    let t = tag(&left, 6).await;
    run!(left, admin, core::delete, delete(11, 6, t.version)).unwrap();
    exchange(&left, &right, true).await;
    for db in [&left, &right] {
        assert_eq!(tag(db, 5).await.state, TagState::Deleted);
        assert_eq!(tag(db, 5).await, tag(db, 6).await);
        assert!(card(db, 3).await.tags.is_empty());
        assert!(card(db, 4).await.tags.is_empty());
    }
    run!(left, admin, core::create, create(12, 7, "Личное")).unwrap();
    assert_eq!(tag(&left, 7).await.tag, sid(7));
    assert!(card(&left, 3).await.tags.is_empty());
}

#[tokio::test]
async fn late_bridge_preserves_three_ids_and_assignments_and_blocks_alias_writes() {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    assign(&left, &admin, 2, 3, 5, true).await;
    let t = tag(&left, 5).await;
    run!(left, admin, core::rename, rename(3, 5, "Личное", t.version)).unwrap();
    run!(left, admin, core::create, create(4, 6, "Работа")).unwrap();
    assign(&left, &admin, 5, 3, 6, true).await;
    run!(right, admin, core::create, create(6, 7, "РАБОТА")).unwrap();
    assign(&right, &admin, 7, 3, 7, true).await;
    exchange(&left, &right, true).await;
    assert_eq!(listing(&left).await, listing(&right).await);
    assert_eq!(listing(&left).await.len(), 3);
    for id in [5, 6, 7] {
        let t = tag(&left, id).await;
        assert_eq!(t.tag, sid(id));
        assert_eq!(t.state, TagState::Conflicted);
        let c = card(&left, 3).await;
        assert_eq!(c.tags, [sid(5), sid(6), sid(7)]);
        assert_eq!(c.state, SpaceState::Active);
        rejected(
            run!(
                left,
                admin,
                core::set_card_tag,
                set(8, 3, id, false, c.version, t.version)
            ),
            ErrorCode::Conflict,
        );
        assert_eq!(card(&left, 3).await, c);
    }
    right.shutdown().await.unwrap();
    let right = Storage::open(dir.path().join("right.db")).await.unwrap();
    assert_eq!(listing(&left).await, listing(&right).await);
}

#[tokio::test]
async fn rename_against_creation_is_not_an_independent_same_name_birth() {
    let (dir, left, admin) = setup().await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    assign(&left, &admin, 2, 3, 5, true).await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    let t = tag(&left, 5).await;
    run!(left, admin, core::rename, rename(3, 5, "Важное", t.version)).unwrap();
    run!(right, admin, core::create, create(4, 6, "ВАЖНОЕ")).unwrap();
    assign(&right, &admin, 5, 3, 6, true).await;
    exchange(&left, &right, true).await;
    let tags = listing(&left).await;
    assert_eq!(tags.len(), 2);
    assert!(tags.iter().all(|t| t.state == TagState::Conflicted));
    assert_eq!(card(&left, 3).await.tags, [sid(5), sid(6)]);
    assert_eq!(card(&left, 3).await.state, SpaceState::Active);
}

#[tokio::test]
async fn aliases_keep_unseen_removal_against_reassignment_as_a_card_conflict() {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    run!(left, admin, core::create, create(1, 5, "Работа")).unwrap();
    run!(right, admin, core::create, create(2, 6, "РАБОТА")).unwrap();
    assign(&left, &admin, 3, 3, 5, true).await;
    exchange(&left, &right, false).await;
    assign(&left, &admin, 4, 3, 5, false).await;
    assign(&right, &admin, 5, 3, 6, false).await;
    assign(&right, &admin, 6, 3, 6, true).await;
    exchange(&left, &right, true).await;
    for db in [&left, &right] {
        let c = card(db, 3).await;
        assert_eq!(c.state, SpaceState::Conflicted);
        assert_eq!(c.tags, [sid(5)]);
        let t = tag(db, 6).await;
        rejected(
            run!(
                db,
                admin,
                core::set_card_tag,
                set(7, 3, 6, false, c.version, t.version)
            ),
            ErrorCode::Conflict,
        );
        assert_eq!(card(db, 3).await, c);
    }
    assert_eq!(card(&left, 3).await, card(&right, 3).await);
}

#[tokio::test]
async fn three_concurrent_births_choose_one_id_without_losing_aliases_on_partial_delivery() {
    let (dir, left, admin) = setup().await;
    let right = copy(&left, dir.path().join("right.db"), &admin).await;
    let third = copy(&left, dir.path().join("third.db"), &admin).await;
    for (db, id) in [(&left, 7), (&right, 6), (&third, 5)] {
        run!(db, admin, core::create, create(id, id, "РАБОТА")).unwrap();
        assign(db, &admin, id + 10, 3, id, true).await;
    }
    exchange(&left, &right, true).await;
    assert_eq!(listing(&left).await[0].tag, sid(6));
    let before = tag(&left, 7).await;
    exchange(&right, &third, false).await;
    exchange(&left, &third, true).await;
    exchange(&left, &right, false).await;
    for db in [&left, &right, &third] {
        assert_eq!(listing(db).await.len(), 1);
        assert_eq!(card(db, 3).await.tags, [sid(5)]);
        assert_eq!(tag(db, 5).await, tag(db, 7).await);
        assert_eq!(tag(db, 5).await, tag(db, 6).await);
    }
    rejected(
        run!(
            left,
            admin,
            core::rename,
            rename(30, 7, "Stale", before.version)
        ),
        ErrorCode::StaleState,
    );
    let current = tag(&left, 7).await;
    run!(
        left,
        admin,
        core::rename,
        rename(31, 7, "Личное", current.version)
    )
    .unwrap();
    exchange(&left, &right, false).await;
    exchange(&right, &third, true).await;
    assert_eq!(listing(&left).await, listing(&third).await);
}
