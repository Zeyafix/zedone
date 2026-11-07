use super::cards::{creation, setup_cards};
use super::spaces::{cli, sid};
use super::*;

#[tokio::test]
async fn search_has_one_strict_contract_and_receipts_on_http_ipc_cli_and_restart() {
    let (dir, storage, service, admin) = setup_cards().await;
    let secret = key(&admin);
    let card = post(&service, "card.create", &creation(), &secret).await["result"].clone();
    for (i, scope) in [
        json!({"kind":"space","space":sid(1)}),
        json!({"kind":"all"}),
    ]
    .into_iter()
    .enumerate()
    {
        let body = request(
            10 + i as u8,
            "card.search",
            json!({"query":"\u{a0}ЁЖ\u{2003}","scope":scope}),
        );
        let result = post(&service, "card.search", &body, &secret).await;
        assert_eq!(result["status"], "ok");
        assert_eq!(
            result["result"]["spaces"][0]["hits"],
            json!([{"card":card,"placement_conflict":false}])
        );
        let mut replay = result.clone();
        replay["replayed"] = json!(true);
        assert_eq!(cli(&dir, &secret, "card.search", &body), replay);
        assert_eq!(
            serde_json::from_str::<Value>(
                &ipc::invoke(&service, "card.search", &body, &secret).await
            )
            .unwrap(),
            replay
        );
    }
    for params in [
        json!({"query":"","scope":{"kind":"space"}}),
        json!({"query":"","scope":{"kind":"space","space":"invalid"}}),
        json!({"query":"","scope":{"kind":"all","space":sid(1)}}),
        json!({"query":"","scope":{"kind":"all"},"include_trashed":true}),
        json!({"query":null,"scope":{"kind":"all"}}),
        json!({"query":"","scope":{"kind":"unknown"}}),
    ] {
        let body = request(20, "card.search", params);
        let expected = json!({"status":"error","error":{"code":"invalid_request"}});
        assert_eq!(
            post(&service, "card.search", &body, &secret).await,
            expected
        );
        assert_eq!(cli(&dir, &secret, "card.search", &body), expected);
        assert_eq!(
            serde_json::from_str::<Value>(
                &ipc::invoke(&service, "card.search", &body, &secret).await
            )
            .unwrap(),
            expected
        );
    }
    storage.shutdown().await.unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let service = Service::new(storage.clone());
    let body = request(
        11,
        "card.search",
        json!({"query":"\u{a0}ЁЖ\u{2003}","scope":{"kind":"all"}}),
    );
    assert_eq!(
        post(&service, "card.search", &body, &secret).await["replayed"],
        true
    );
    storage.shutdown().await.unwrap();
}
