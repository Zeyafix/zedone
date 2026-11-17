use super::spaces::{cli, setup_spaces, sid};
use super::*;
use zedone_core::{
    cards::{CardStore, CreateCard, EditCard},
    columns::ColumnStore,
    spaces::SpaceStore,
};

#[tokio::test]
async fn text_resolution_uses_identical_http_ipc_cli_receipts_validation_and_revocation() {
    let (dir, db, service, admin) = setup_spaces().await;
    let owner = admin.clone();
    let page = db
        .transact(move |tx| {
            tx.create_space(&sid(1), "Board", owner.subject())?;
            tx.create_column(&sid(1), &sid(2), "Column")?;
            tx.create_card(
                &CreateCard {
                    space: sid(1),
                    column: sid(2),
                    card: sid(3),
                    title: "Base".into(),
                    description: "Base text".into(),
                },
                owner.subject(),
                &sid(4),
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
            let selection = tx.read().sync_selection(&sid(1), &sid(9))?;
            tx.set_sync_selected(&sid(1), &sid(9), true, selection.version)?;
            tx.receive_sync_page(&sid(1), &sid(9), owner.subject(), &page)?;
            let before = tx.card_snapshot(&sid(1), &sid(3))?;
            tx.edit_card(
                &EditCard {
                    space: sid(1),
                    card: sid(3),
                    title: "Remote".into(),
                    description: "Remote text".into(),
                    expected: before.version,
                },
                owner.subject(),
                &sid(5),
            )?;
            zedone_storage::sync_engine::page(tx.read().document(&sid(1))?, &sid(1), &vec![])
        })
        .await
        .unwrap();
    let owner = admin.clone();
    db.transact(move |tx| {
        let before = tx.card_snapshot(&sid(1), &sid(3))?;
        tx.edit_card(
            &EditCard {
                space: sid(1),
                card: sid(3),
                title: "Local".into(),
                description: "Local text".into(),
                expected: before.version,
            },
            owner.subject(),
            &sid(6),
        )?;
        tx.apply_changes(
            &sid(1),
            zedone_storage::sync_engine::decode_changes(&page.changes)?,
        )
    })
    .await
    .unwrap();
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (owner, user) = (admin.clone(), member.clone());
    db.timed_transact(move |tx, clock| {
        tx.issue_access(
            &owner.credential(),
            clock,
            &user,
            &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
            None,
        )
    })
    .await
    .unwrap();
    let secret = key(&admin);
    let read = request(10, "card.read", json!({"space":sid(1),"card":sid(3)}));
    let before = post(&service, "card.read", &read, &secret).await;
    let parameters = json!({"space":sid(1),"card":sid(3),"title":"Chosen","description":"Merged 😀","expected":before["result"]["version"]});
    let denied = request(11, "card.resolve_text", parameters.clone());
    for key in [key(&member), "00".repeat(32)] {
        let result = post(&service, "card.resolve_text", &denied, &key).await;
        assert_eq!(result["error"]["code"], "access_denied");
        assert_eq!(cli(&dir, &key, "card.resolve_text", &denied), result);
        assert_eq!(
            serde_json::from_str::<Value>(
                &ipc::invoke(&service, "card.resolve_text", &denied, &key).await
            )
            .unwrap(),
            result
        );
    }
    for (id, field, value) in [
        (12, "title", json!(" \n")),
        (13, "unexpected", json!(true)),
        (14, "expected", json!([0])),
    ] {
        let mut invalid = parameters.clone();
        invalid[field] = value;
        let body = request(id, "card.resolve_text", invalid);
        let result = post(&service, "card.resolve_text", &body, &secret).await;
        assert_eq!(result["error"]["code"], "invalid_request");
        assert_eq!(cli(&dir, &secret, "card.resolve_text", &body), result);
        assert_eq!(
            serde_json::from_str::<Value>(
                &ipc::invoke(&service, "card.resolve_text", &body, &secret).await
            )
            .unwrap(),
            result
        );
    }
    let body = request(15, "card.resolve_text", parameters.clone());
    let first = post(&service, "card.resolve_text", &body, &secret).await;
    assert_eq!(first["status"], "ok");
    assert_eq!(first["result"]["titles"], json!(["Chosen"]));
    let replay: Value =
        serde_json::from_str(&ipc::invoke(&service, "card.resolve_text", &body, &secret).await)
            .unwrap();
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], first["result"]);
    assert_eq!(cli(&dir, &secret, "card.resolve_text", &body), replay);
    assert_eq!(
        post(
            &service,
            "card.resolve_text",
            &request(16, "card.resolve_text", parameters.clone()),
            &secret
        )
        .await["error"]["code"],
        "stale_state"
    );
    let history = post(
        &service,
        "card.history",
        &request(
            17,
            "card.history",
            json!({"space":sid(1),"card":sid(3),"after":null,"expected":null,"limit":100}),
        ),
        &secret,
    )
    .await;
    assert_eq!(history["result"]["versions"].as_array().unwrap().len(), 4);
    let second_admin = PendingAccess::generate().unwrap();
    let owner = admin.clone();
    db.timed_transact(move |tx, clock| {
        tx.issue_access(
            &owner.credential(),
            clock,
            &second_admin,
            &GrantScope::Administration,
            None,
        )
    })
    .await
    .unwrap();
    let access = post(
        &service,
        "access.inspect",
        &inspect(18, admin.subject()),
        &secret,
    )
    .await;
    assert_eq!(
        post(
            &service,
            "access.revoke",
            &request(
                19,
                "access.revoke",
                json!({"subject":admin.subject(),"expected":access["result"]["version"]})
            ),
            &secret
        )
        .await["status"],
        "ok"
    );
    assert_eq!(
        post(&service, "card.resolve_text", &body, &secret).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        cli(&dir, &secret, "card.resolve_text", &body)["error"]["code"],
        "access_denied"
    );
}
