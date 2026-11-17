use super::*;
use zedone_core::{
    cards::{CardStore, CreateCard},
    columns::{self, ColumnStore, ResolveColumnName},
    sync::SyncStore,
};

async fn seed(storage: &Storage, admin: &PendingAccess) {
    let subject = admin.subject().to_owned();
    storage
        .transact(move |tx| {
            tx.create_space(&sid(1), "Base", &subject)?;
            tx.create_column(&sid(1), &sid(2), "Column")?;
            tx.create_card(
                &CreateCard {
                    space: sid(1),
                    column: sid(2),
                    card: sid(3),
                    title: "Saved".into(),
                    description: "Original".into(),
                },
                &subject,
                &sid(4),
            )?;
            let graph = tx.read().document(&sid(1))?;
            let mut a = graph.fork().with_actor(ActorId::from(vec![21]));
            let mut b = graph.fork().with_actor(ActorId::from(vec![22]));
            for (graph, name) in [(&mut a, "First"), (&mut b, "Second")] {
                let column = crate::columns::object(graph, &sid(2))?;
                let mut t = graph.transaction();
                t.put(ROOT, "name", name)?;
                t.put(&column, "name", name)?;
                t.commit();
            }
            tx.apply_changes(&sid(1), a.get_changes(&[]))?;
            tx.apply_changes(&sid(1), b.get_changes(&[]))
        })
        .await
        .unwrap();
}
fn resolution(id: u8, name: &str, expected: StateVersion) -> CommandRequest<ResolveSpaceName> {
    req(
        id,
        ResolveSpaceName {
            space: sid(1),
            name: name.into(),
            expected,
        },
    )
}

#[tokio::test]
async fn name_conflicts_preserve_content_rights_stale_replay_restart_and_reverse_rename() {
    let (dir, db, admin) = setup().await;
    seed(&db, &admin).await;
    let initial = fresh(&db, &admin, 1, 1).await;
    let card = db
        .transact(|tx| Ok(tx.card_snapshot(&sid(1), &sid(3))?))
        .await
        .unwrap();
    assert_eq!(initial.names, ["First", "Second"]);
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    db.transact(move |tx| {
        tx.issue_access(
            &a.credential(),
            &Now,
            &m,
            &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
            None,
        )
    })
    .await
    .unwrap();
    rejected(
        run!(
            &db,
            member,
            resolve_name,
            resolution(2, "Picked", initial.version)
        ),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(
            &db,
            admin,
            rename,
            rename_req(3, 1, "Overwrite", initial.version)
        ),
        ErrorCode::Conflict,
    );
    rejected(
        run!(&db, admin, trash, trash_req(4, 1, initial.version)),
        ErrorCode::Conflict,
    );
    let secret = admin.credential().0.to_vec();
    let changed = db
        .transact(move |tx| {
            Ok(zedone_core::cards::set_completed(
                tx,
                &Credential(&secret),
                &Now,
                &req(
                    5,
                    zedone_core::cards::SetCardCompleted {
                        space: sid(1),
                        card: sid(3),
                        completed: true,
                        expected: card.version,
                    },
                ),
            )?
            .result)
        })
        .await
        .unwrap();
    assert_eq!(changed.completed, [true]);
    rejected(
        run!(
            &db,
            admin,
            resolve_name,
            resolution(6, "Stale", initial.version)
        ),
        ErrorCode::StaleState,
    );
    let current = fresh(&db, &admin, 7, 1).await;
    let resolving_version = current.version;
    let resolved = run!(
        &db,
        admin,
        resolve_name,
        resolution(8, "Second", current.version)
    )
    .unwrap();
    assert_eq!(resolved.result.space, sid(1));
    assert_eq!(resolved.result.names, ["Second"]);
    assert_eq!(resolved.result.contents, current.contents);
    assert!(
        run!(
            &db,
            admin,
            resolve_name,
            resolution(8, "Second", resolving_version)
        )
        .unwrap()
        .replayed
    );
    rejected(
        run!(
            &db,
            admin,
            resolve_name,
            resolution(9, "No conflict", resolved.result.version)
        ),
        ErrorCode::Conflict,
    );
    let column = db
        .transact(|tx| Ok(tx.column_snapshot(&sid(1), &sid(2))?))
        .await
        .unwrap();
    let secret = admin.credential().0.to_vec();
    let c = db
        .transact(move |tx| {
            Ok(columns::resolve_name(
                tx,
                &Credential(&secret),
                &Now,
                &req(
                    10,
                    ResolveColumnName {
                        space: sid(1),
                        column: sid(2),
                        name: String::new(),
                        expected: column.version,
                    },
                ),
            )?
            .result)
        })
        .await
        .unwrap();
    assert_eq!(c.names, [""]);
    assert_eq!(c.column, sid(2));
    assert_eq!(c.contents.unwrap()[0], changed);
    let current = fresh(&db, &admin, 11, 1).await;
    run!(
        &db,
        admin,
        rename,
        rename_req(12, 1, "Renamed again", current.version)
    )
    .unwrap();
    assert!(
        run!(
            &db,
            admin,
            resolve_name,
            resolution(8, "Second", resolving_version)
        )
        .unwrap()
        .replayed
    );
    db.shutdown().await.unwrap();
    let db = Storage::open(dir.path().join("spaces.db")).await.unwrap();
    let late = fresh(&db, &admin, 13, 1).await;
    assert_eq!(late.names, ["Renamed again"]);
    assert!(
        run!(
            &db,
            admin,
            resolve_name,
            resolution(8, "Second", resolving_version)
        )
        .unwrap()
        .replayed
    );
    let a = admin.clone();
    let m = member.subject().to_owned();
    db.transact(move |tx| tx.revoke_access(&a.credential(), &Now, &m))
        .await
        .unwrap();
    rejected(
        run!(
            &db,
            member,
            resolve_name,
            resolution(14, "Forbidden", late.version)
        ),
        ErrorCode::AccessDenied,
    );
}

#[tokio::test]
async fn independent_name_resolutions_reconflict_after_exchange_in_both_orders_and_repeats() {
    let (dir, left, admin) = setup().await;
    seed(&left, &admin).await;
    let page = left
        .transact(|tx| crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![]))
        .await
        .unwrap();
    let right = Storage::open(dir.path().join("right.db")).await.unwrap();
    let owner = admin.clone();
    right
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let selection = tx.read().sync_selection(&sid(1), &sid(9))?;
            tx.set_sync_selected(&sid(1), &sid(9), true, selection.version)?;
            tx.receive_sync_page(&sid(1), &sid(9), owner.subject(), &page)
        })
        .await
        .unwrap();
    let initial = fresh(&left, &admin, 1, 1).await;
    assert_eq!(fresh(&right, &admin, 1, 1).await.version, initial.version);
    run!(
        &left,
        admin,
        resolve_name,
        resolution(2, "Left choice", initial.version)
    )
    .unwrap();
    run!(
        &right,
        admin,
        resolve_name,
        resolution(3, "Right choice", initial.version)
    )
    .unwrap();
    for (db, name, id) in [(&left, "Left column", 4), (&right, "Right column", 5)] {
        let c = db
            .transact(|tx| Ok(tx.column_snapshot(&sid(1), &sid(2))?))
            .await
            .unwrap();
        let secret = admin.credential().0.to_vec();
        db.transact(move |tx| {
            Ok(columns::resolve_name(
                tx,
                &Credential(&secret),
                &Now,
                &req(
                    id,
                    ResolveColumnName {
                        space: sid(1),
                        column: sid(2),
                        name: name.into(),
                        expected: c.version,
                    },
                ),
            )?)
        })
        .await
        .unwrap();
    }
    let l = left
        .transact(|tx| crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![]))
        .await
        .unwrap();
    let r = right
        .transact(|tx| crate::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![]))
        .await
        .unwrap();
    for (db, page) in [(&left, r), (&right, l)] {
        db.transact(move |tx| {
            tx.push_sync(&sid(1), "exchange", &vec![], &page.changes)?;
            tx.push_sync(&sid(1), "exchange", &vec![], &page.changes)?;
            Ok(())
        })
        .await
        .unwrap();
    }
    let current = fresh(&left, &admin, 6, 1).await;
    assert_eq!(current, fresh(&right, &admin, 6, 1).await);
    assert_eq!(current.names, ["Left choice", "Right choice"]);
    assert_eq!(
        current.contents.as_ref().unwrap().columns[0].names,
        ["Left column", "Right column"]
    );
    let before = current.version;
    let (a, b) = tokio::join!(
        async { run!(&left, admin, resolve_name, resolution(7, "Final", before)) },
        async {
            run!(
                &left,
                admin,
                resolve_name,
                resolution(8, "Losing stale", before)
            )
        }
    );
    assert_eq!([a.is_ok(), b.is_ok()].into_iter().filter(|v| *v).count(), 1);
    assert!([a, b].into_iter().any(|v| matches!(
        v,
        Err(StorageError::Rejected(CommandError {
            code: ErrorCode::StaleState
        }))
    )));
}

#[tokio::test]
async fn resolution_failures_roll_back_graph_and_receipt_at_commit_boundaries() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
    ] {
        let (_dir, db, admin) = setup().await;
        seed(&db, &admin).await;
        let before = fresh(&db, &admin, 1, 1).await;
        let secret = admin.credential().0.to_vec();
        let request = resolution(2, "Choice", before.version);
        let failure = db
            .transact_checked(
                move |tx| {
                    Ok(core::resolve_name(
                        tx,
                        &Credential(&secret),
                        &Now,
                        &request,
                    )?)
                },
                move |at| {
                    if at == stage {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await;
        assert_eq!(failure.unwrap_err(), StorageError::Unavailable);
        assert_eq!(fresh(&db, &admin, 3, 1).await, before);
        assert!(
            !run!(
                &db,
                admin,
                resolve_name,
                resolution(2, "Choice", before.version)
            )
            .unwrap()
            .replayed
        );
    }
}
