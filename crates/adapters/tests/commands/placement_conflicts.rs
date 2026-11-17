use super::*;
use super::{
    ordering::all_entries,
    spaces::{setup_spaces, sid},
};
use zedone_core::{
    cards::{CardStore, CreateCard},
    columns::ColumnStore,
    ordering::{CardPlacement, OrderStore, Rank},
    spaces::SpaceStore,
};

#[tokio::test]
async fn saved_placement_choices_share_http_ipc_cli_validation_replay_and_access_rules() {
    let (dir, db, service, admin) = setup_spaces().await;
    let owner = admin.clone();
    let initial = db
        .transact(move |tx| {
            tx.create_space(&sid(1), "Board", owner.subject())?;
            for column in [2, 4] {
                tx.create_column(&sid(1), &sid(column), "Column")?;
            }
            tx.create_card(
                &CreateCard {
                    space: sid(1),
                    column: sid(2),
                    card: sid(3),
                    title: "Base".into(),
                    description: "Body".into(),
                },
                owner.subject(),
                &sid(3),
            )?;
            zedone_storage::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
        })
        .await
        .unwrap();
    let remote = Storage::open(dir.path().join("remote.db")).await.unwrap();
    let owner = admin.clone();
    let page = remote
        .transact(move |tx| {
            tx.bootstrap_access(&owner)?;
            let selected = tx.read().sync_selection(&sid(1), &sid(9))?;
            tx.set_sync_selected(&sid(1), &sid(9), true, selected.version)?;
            tx.receive_sync_page(&sid(1), &sid(9), owner.subject(), &initial)?;
            tx.place_card(
                &sid(1),
                &sid(3),
                &CardPlacement {
                    column: sid(4),
                    rank: Rank::Legacy(1),
                },
            )?;
            zedone_storage::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
        })
        .await
        .unwrap();
    db.transact(move |tx| {
        tx.place_card(
            &sid(1),
            &sid(3),
            &CardPlacement {
                column: sid(2),
                rank: Rank::Legacy(1),
            },
        )?;
        tx.apply_changes(
            &sid(1),
            zedone_storage::sync_engine::decode_changes(&page.changes)?,
        )
    })
    .await
    .unwrap();
    let secret = key(&admin);
    let inspection = all_entries(
        &service,
        &dir,
        &secret,
        "card.inspect_placements",
        &request(
            10,
            "card.inspect_placements",
            json!({"space":sid(1),"card":sid(3)}),
        ),
    )
    .await;
    assert_eq!(inspection["result"]["choices"].as_array().unwrap().len(), 2);
    let card = &inspection["result"]["card"];
    let parameters = json!({"space":sid(1),"card":sid(3),"placement":card["placements"][0],"expected":card["version"]});
    let reader = Arc::new(PendingAccess::generate().unwrap());
    let writer = Arc::new(PendingAccess::generate().unwrap());
    let (a, r, w) = (admin.clone(), reader.clone(), writer.clone());
    db.timed_transact(move |tx, clock| {
        for (key, permission) in [
            (&r, SpacePermission::Read),
            (&w, SpacePermission::ReadWrite),
        ] {
            tx.issue_access(
                &a.credential(),
                clock,
                key,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), permission)])),
                None,
            )?;
        }
        Ok(())
    })
    .await
    .unwrap();
    assert_eq!(
        all_entries(
            &service,
            &dir,
            &key(&reader),
            "card.inspect_placements",
            &request(
                11,
                "card.inspect_placements",
                json!({"space":sid(1),"card":sid(3)})
            )
        )
        .await["status"],
        "ok"
    );
    assert_eq!(
        all_entries(
            &service,
            &dir,
            &key(&reader),
            "card.resolve_placement",
            &request(12, "card.resolve_placement", parameters.clone())
        )
        .await["error"]["code"],
        "access_denied"
    );
    for (id, field, value, code) in [
        (
            13,
            "placement",
            json!({"column":sid(4),"rank":999}),
            "invalid_request",
        ),
        (14, "expected", json!(vec![0; 32]), "stale_state"),
        (15, "extra", json!(true), "invalid_request"),
    ] {
        let mut invalid = parameters.clone();
        invalid[field] = value;
        assert_eq!(
            all_entries(
                &service,
                &dir,
                &secret,
                "card.resolve_placement",
                &request(id, "card.resolve_placement", invalid)
            )
            .await["error"]["code"],
            code
        );
    }
    let body = request(16, "card.resolve_placement", parameters.clone());
    let chosen = all_entries(
        &service,
        &dir,
        &key(&writer),
        "card.resolve_placement",
        &body,
    )
    .await;
    assert_eq!(chosen["result"]["state"], "active");
    let movement = request(
        17,
        "card.move",
        json!({"space":sid(1),"card":sid(3),"column":sid(4),"before":null,"expected":chosen["result"]["version"]}),
    );
    let back = all_entries(&service, &dir, &secret, "card.move", &movement).await;
    assert_eq!(back["result"]["titles"], card["titles"]);
    let replay = all_entries(
        &service,
        &dir,
        &key(&writer),
        "card.resolve_placement",
        &body,
    )
    .await;
    assert_eq!(replay["result"], chosen["result"]);
    assert_eq!(replay["replayed"], true);
    let mut mismatch = parameters.clone();
    mismatch["placement"] = card["placements"][1].clone();
    assert_eq!(
        all_entries(
            &service,
            &dir,
            &key(&writer),
            "card.resolve_placement",
            &request(16, "card.resolve_placement", mismatch)
        )
        .await["error"]["code"],
        "command_mismatch"
    );
    let (a, w) = (admin.clone(), writer.clone());
    db.timed_transact(move |tx, clock| tx.revoke_access(&a.credential(), clock, w.subject()))
        .await
        .unwrap();
    assert_eq!(
        all_entries(
            &service,
            &dir,
            &key(&writer),
            "card.resolve_placement",
            &body
        )
        .await["error"]["code"],
        "access_denied"
    );
    db.shutdown().await.unwrap();
    assert_eq!(
        super::spaces::cli(&dir, &secret, "card.move", &movement)["result"],
        back["result"]
    );
}
