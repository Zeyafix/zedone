use super::cards::{creation, setup_cards};
use super::spaces::{cli, sid};
use super::*;

async fn all_entries(
    service: &Service,
    dir: &TempDir,
    secret: &str,
    operation: &str,
    body: &str,
) -> Value {
    let response = post(service, operation, body, secret).await;
    let repeated =
        serde_json::from_str::<Value>(&ipc::invoke(service, operation, body, secret).await)
            .unwrap();
    if response["status"] == "ok" {
        assert_eq!(repeated["replayed"], true);
        assert_eq!(response["result"], repeated["result"]);
    } else {
        assert_eq!(response, repeated);
    }
    assert_eq!(cli(dir, secret, operation, body), repeated);
    response
}
#[tokio::test]
async fn moves_and_undo_use_the_same_http_ipc_cli_contract_and_original_receipts() {
    let (dir, s, service, a) = setup_cards().await;
    let secret = key(&a);
    let created = post(&service, "card.create", &creation(), &secret).await;
    let col = post(
        &service,
        "column.create",
        &request(
            2,
            "column.create",
            json!({"space":sid(1),"column":sid(4),"name":"Other"}),
        ),
        &secret,
    )
    .await;
    let movement = request(
        3,
        "card.move",
        json!({"space":sid(1),"card":sid(3),"column":sid(4),"before":null,"expected":created["result"]["version"]}),
    );
    let moved = all_entries(&service, &dir, &secret, "card.move", &movement).await;
    assert_eq!(moved["result"]["columns"], json!([sid(4)]));
    let undo = request(
        4,
        "card.move",
        json!({"space":sid(1),"card":sid(3),"column":sid(2),"before":null,"expected":moved["result"]["version"]}),
    );
    let back = all_entries(&service, &dir, &secret, "card.move", &undo).await;
    let stale = request(
        5,
        "card.move",
        json!({"space":sid(1),"card":sid(3),"column":sid(4),"before":null,"expected":created["result"]["version"]}),
    );
    assert_eq!(
        all_entries(&service, &dir, &secret, "card.move", &stale).await["error"]["code"],
        "stale_state"
    );
    let read = post(
        &service,
        "column.read",
        &request(6, "column.read", json!({"space":sid(1),"column":sid(4)})),
        &secret,
    )
    .await;
    assert_ne!(read["result"]["version"], col["result"]["version"]);
    let column_move = request(
        7,
        "column.move",
        json!({"space":sid(1),"column":sid(4),"before":sid(2),"expected":read["result"]["version"]}),
    );
    let reordered = all_entries(&service, &dir, &secret, "column.move", &column_move).await;
    let reverse = request(
        8,
        "column.move",
        json!({"space":sid(1),"column":sid(4),"before":null,"expected":reordered["result"]["version"]}),
    );
    all_entries(&service, &dir, &secret, "column.move", &reverse).await;
    for (id, operation, params) in [
        (
            9,
            "card.move",
            json!({"space":sid(1),"card":sid(3),"column":sid(4),"before":42,"expected":back["result"]["version"]}),
        ),
        (
            10,
            "column.move",
            json!({"space":sid(1),"column":sid(4),"before":null,"expected":read["result"]["version"],"rank":0}),
        ),
    ] {
        assert_eq!(
            all_entries(
                &service,
                &dir,
                &secret,
                operation,
                &request(id, operation, params)
            )
            .await["error"]["code"],
            "invalid_request"
        );
    }
    s.shutdown().await.unwrap();
    assert_eq!(
        cli(&dir, &secret, "card.move", &movement)["result"],
        moved["result"]
    );
    assert_eq!(
        cli(&dir, &secret, "column.move", &column_move)["result"],
        reordered["result"]
    );
}

#[tokio::test]
async fn read_only_and_revoked_access_block_moves_and_replays_on_every_entry() {
    let (dir, s, service, a) = setup_cards().await;
    let secret = key(&a);
    let card = post(&service, "card.create", &creation(), &secret).await;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let writer = Arc::new(PendingAccess::generate().unwrap());
    let (b, m, w) = (a.clone(), member.clone(), writer.clone());
    s.timed_transact(move |ctx, clock| {
        for (key, permission) in [
            (&m, SpacePermission::Read),
            (&w, SpacePermission::ReadWrite),
        ] {
            ctx.issue_access(
                &b.credential(),
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
    let read = post(
        &service,
        "column.read",
        &request(2, "column.read", json!({"space":sid(1),"column":sid(2)})),
        &secret,
    )
    .await;
    let cases = [
        (
            "card.move",
            request(
                3,
                "card.move",
                json!({"space":sid(1),"card":sid(3),"column":sid(2),"before":null,"expected":card["result"]["version"]}),
            ),
        ),
        (
            "column.move",
            request(
                4,
                "column.move",
                json!({"space":sid(1),"column":sid(2),"before":null,"expected":read["result"]["version"]}),
            ),
        ),
    ];
    for (op, body) in &cases {
        assert_eq!(
            all_entries(&service, &dir, &key(&member), op, body).await["error"]["code"],
            "access_denied"
        );
        assert_eq!(
            all_entries(&service, &dir, &key(&writer), op, body).await["status"],
            "ok"
        );
    }
    let (b, w) = (a.clone(), writer.clone());
    s.timed_transact(move |ctx, clock| {
        ctx.revoke_access(&b.credential(), clock, w.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    for (op, body) in &cases {
        assert_eq!(
            all_entries(&service, &dir, &key(&writer), op, body).await["error"]["code"],
            "access_denied"
        );
    }
}
