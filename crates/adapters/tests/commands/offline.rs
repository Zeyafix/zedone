use super::cards::{creation, setup_cards};
use super::spaces::{cli, sid};
use super::*;

// Rotate the FIRST execution too: CLI/IPC must not only replay HTTP receipts.
struct Board {
    dir: TempDir,
    storage: Storage,
    service: Service,
    secret: String,
    id: u8,
    first: usize,
    receipts: Vec<(String, String, Value)>,
}
impl Board {
    async fn new(first: usize) -> Self {
        let (dir, storage, service, admin) = setup_cards().await;
        let secret = key(&admin);
        let created = post(&service, "card.create", &creation(), &secret).await;
        assert_eq!(created["status"], "ok");
        Self {
            dir,
            storage,
            service,
            secret,
            id: 2,
            first,
            receipts: vec![],
        }
    }
    async fn deliver(&self, entry: usize, op: &str, body: &str) -> Value {
        match entry % 3 {
            0 => post(&self.service, op, body, &self.secret).await,
            1 => serde_json::from_str(&ipc::invoke(&self.service, op, body, &self.secret).await)
                .unwrap(),
            _ => cli(&self.dir, &self.secret, op, body),
        }
    }
    async fn call(&mut self, op: &str, p: Value) -> Value {
        let body = request(self.id, op, p);
        self.id += 1;
        let response = self.deliver(self.first, op, &body).await;
        self.first += 1;
        assert_eq!(response["status"], "ok", "{op}: {response}");
        assert_eq!(response["replayed"], false, "{op}: {response}");
        self.receipts.push((op.into(), body, response.clone()));
        response["result"].clone()
    }
    async fn read(&mut self, kind: &str) -> Value {
        let p = match kind {
            "card" => json!({"space":sid(1),"card":sid(3)}),
            "column" => json!({"space":sid(1),"column":sid(2)}),
            "space" => json!({"space":sid(1)}),
            _ => unreachable!(),
        };
        self.call(&format!("{kind}.read"), p).await
    }
    async fn history(&mut self) -> Value {
        self.call(
            "card.history",
            json!({"space":sid(1),"card":sid(3),"limit":100,"after":null,"expected":null}),
        )
        .await
    }
    async fn change(&mut self, op: &str, card: &Value, extra: Value) -> Value {
        let mut p = json!({"space":sid(1),"card":sid(3),"expected":card["version"]});
        p.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        self.call(op, p).await
    }
    async fn stale(&mut self, op: &str, p: Value) {
        let body = request(self.id, op, p);
        self.id += 1;
        for entry in 0..3 {
            assert_eq!(
                self.deliver(entry, op, &body).await,
                json!({"status":"error","error":{"code":"stale_state"}}),
                "{op}"
            );
        }
    }
    async fn restart(&mut self) {
        self.storage.clone().shutdown().await.unwrap();
        self.storage = Storage::open(self.dir.path().join("device.db"))
            .await
            .unwrap();
        self.storage.recover_editing_sessions().await.unwrap();
        self.service = Service::new(self.storage.clone());
    }
    async fn replay_all(&self) {
        for (op, body, original) in &self.receipts {
            let mut expected = original.clone();
            expected["replayed"] = json!(true);
            for entry in 0..3 {
                assert_eq!(self.deliver(entry, op, body).await, expected, "{op}");
            }
        }
    }
}

#[tokio::test]
async fn new_description_invalidates_every_old_card_intention_and_both_container_deletions() {
    let mut b = Board::new(0).await;
    let tag = b
        .call(
            "tag.create",
            json!({"space":sid(1),"tag":sid(4),"name":"Work"}),
        )
        .await;
    let card = b.read("card").await;
    let column = b.read("column").await;
    let space = b.read("space").await;
    let history = b.history().await;
    let edited = b
        .change(
            "card.edit",
            &card,
            json!({"title":"Ёж 空間 😀","description":"New description"}),
        )
        .await;
    let before_column = b.read("column").await;
    let before_space = b.read("space").await;
    let before_history = b.history().await;
    // All payloads are valid. Only the shared original card version is stale.
    for (op, extra) in [
        ("card.edit", json!({"title":"Old","description":"Old"})),
        ("card.set_completed", json!({"completed":true})),
        (
            "card.set_tag",
            json!({"tag":sid(4),"present":true,"expected_tag":tag["version"]}),
        ),
        ("card.move", json!({"column":sid(2),"before":null})),
        ("card.trash", json!({})),
        ("card.restore", json!({})),
        ("card.begin_editing", json!({})),
        (
            "card.autosave",
            json!({"session":"05".repeat(16),"title":"Old","description":"Old"}),
        ),
        ("card.finish_editing", json!({"session":"05".repeat(16)})),
        (
            "card.restore_text",
            json!({"text_version":history["versions"][0]["id"]}),
        ),
    ] {
        let mut p = json!({"space":sid(1),"card":sid(3),"expected":card["version"]});
        p.as_object_mut()
            .unwrap()
            .extend(extra.as_object().unwrap().clone());
        b.stale(op, p).await;
    }
    b.stale(
        "column.trash",
        json!({"space":sid(1),"column":sid(2),"expected":column["version"]}),
    )
    .await;
    b.stale(
        "space.trash",
        json!({"space":sid(1),"expected":space["version"]}),
    )
    .await;
    assert_eq!(b.read("card").await, edited);
    assert_eq!(b.read("column").await, before_column);
    assert_eq!(b.read("space").await, before_space);
    assert_eq!(b.history().await, before_history);
    // Returning to identical text cannot revive an unexecuted old intention.
    let returned = b
        .change(
            "card.edit",
            &edited,
            json!({"title":"Ёж 空間 😀","description":"<b>raw</b>\n**text**"}),
        )
        .await;
    assert_eq!(returned["descriptions"], card["descriptions"]);
    assert_ne!(returned["version"], card["version"]);
    b.restart().await;
    b.stale(
        "card.set_completed",
        json!({"space":sid(1),"card":sid(3),"completed":true,"expected":card["version"]}),
    )
    .await;
    assert_eq!(b.read("card").await, returned);
    // Read-only history and receipts do not invalidate current intentions.
    let history = b.history().await;
    let done = b
        .change("card.set_completed", &returned, json!({"completed":true}))
        .await;
    assert_eq!(done["completed"], json!([true]));
    assert_eq!(b.history().await, history);
    b.storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn mixed_offline_lifecycle_restarts_and_replays_without_undoing_later_actions() {
    // Each adapter starts the same scenario; new commands and retries both run
    // through all entries, with no listener, remote peer or external service.
    for first in 0..3 {
        let mut b = Board::new(first).await;
        b.call(
            "column.create",
            json!({"space":sid(1),"column":sid(5),"name":"Done"}),
        )
        .await;
        let tag = b
            .call(
                "tag.create",
                json!({"space":sid(1),"tag":sid(4),"name":"Работа"}),
            )
            .await;
        let mut card = b.read("card").await;
        for (op, extra) in [
            (
                "card.set_tag",
                json!({"tag":sid(4),"present":true,"expected_tag":tag["version"]}),
            ),
            ("card.set_completed", json!({"completed":true})),
            ("card.move", json!({"column":sid(5),"before":null})),
            (
                "card.edit",
                json!({"title":"Current","description":"Сохранено 😀"}),
            ),
        ] {
            card = b.change(op, &card, extra).await;
        }
        let column = b.read("column").await;
        // The empty source column can be trashed/restored independently.
        let removed = b
            .call(
                "column.trash",
                json!({"space":sid(1),"column":sid(2),"expected":column["version"]}),
            )
            .await;
        b.call(
            "column.restore",
            json!({"space":sid(1),"column":sid(2),"expected":removed["version"]}),
        )
        .await;
        let space = b.read("space").await;
        let deleted = b
            .call(
                "space.trash",
                json!({"space":sid(1),"expected":space["version"]}),
            )
            .await;
        b.call(
            "space.restore",
            json!({"space":sid(1),"expected":deleted["version"]}),
        )
        .await;
        card = b.read("card").await;
        assert_eq!(card["titles"], json!(["Current"]));
        assert_eq!(card["tags"], json!([sid(4)]));
        assert_eq!(card["state"], "active");
        assert_eq!(card["completed"], json!([true]));
        assert_eq!(card["columns"], json!([sid(5)]));
        let tag = b
            .call("tag.read", json!({"space":sid(1),"tag":sid(4)}))
            .await;
        for (op, extra) in [
            (
                "card.set_tag",
                json!({"tag":sid(4),"present":false,"expected_tag":tag["version"]}),
            ),
            ("card.set_completed", json!({"completed":false})),
            ("card.move", json!({"column":sid(2),"before":null})),
        ] {
            card = b.change(op, &card, extra).await;
        }
        assert_eq!(card["tags"], json!([]));
        assert_eq!(card["completed"], json!([false]));
        assert_eq!(card["columns"], json!([sid(2)]));
        let history = b.history().await;
        assert_eq!(history["versions"].as_array().unwrap().len(), 2);
        let column = b.read("column").await;
        let space = b.read("space").await;
        b.restart().await;
        b.replay_all().await;
        assert_eq!(b.read("card").await, card);
        assert_eq!(b.read("column").await, column);
        assert_eq!(b.read("space").await, space);
        assert_eq!(b.history().await, history);
        let found = b
            .call(
                "card.search",
                json!({"query":"сохранено","scope":{"kind":"all"}}),
            )
            .await;
        assert_eq!(
            found["spaces"][0]["hits"],
            json!([{"card":card,"placement_conflict":false}])
        );
        b.storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn unavailable_host_preserves_original_request_for_cli_reopen_and_concurrent_delivery() {
    let mut b = Board::new(0).await;
    let card = b.read("card").await;
    let parameters = json!({"space":sid(1),"card":sid(3),"title":"After outage","description":"Saved once","expected":card["version"]});
    let body = request(200, "card.edit", parameters.clone());
    b.storage.clone().shutdown().await.unwrap();
    for entry in 0..2 {
        assert_eq!(
            b.deliver(entry, "card.edit", &body).await,
            json!({"status":"error","error":{"code":"unavailable"}})
        );
    }
    // CLI opens its own admitted worker using exactly the undelivered request.
    let original = b.deliver(2, "card.edit", &body).await;
    assert_eq!(original["status"], "ok");
    assert_eq!(original["replayed"], false);
    b.storage = Storage::open(b.dir.path().join("device.db")).await.unwrap();
    b.service = Service::new(b.storage.clone());
    let peer = Storage::open(b.dir.path().join("device.db")).await.unwrap();
    let other = Service::new(peer.clone());
    let (http, ipc) = tokio::join!(
        post(&b.service, "card.edit", &body, &b.secret),
        ipc::invoke(&other, "card.edit", &body, &b.secret)
    );
    let mut expected = original.clone();
    expected["replayed"] = json!(true);
    assert_eq!(http, expected);
    assert_eq!(serde_json::from_str::<Value>(&ipc).unwrap(), expected);
    peer.shutdown().await.unwrap();
    let later = b
        .change(
            "card.edit",
            &original["result"],
            json!({"title":"Later","description":"Keep this"}),
        )
        .await;
    b.restart().await;
    for entry in 0..3 {
        assert_eq!(b.deliver(entry, "card.edit", &body).await, expected);
        let mut changed = parameters.clone();
        changed["expected"] = later["version"].clone();
        assert_eq!(
            b.deliver(entry, "card.edit", &request(200, "card.edit", changed))
                .await,
            json!({"status":"error","error":{"code":"command_mismatch"}})
        );
    }
    assert_eq!(b.read("card").await, later);
    assert_eq!(b.history().await["versions"].as_array().unwrap().len(), 3);
    b.storage.shutdown().await.unwrap();
}
