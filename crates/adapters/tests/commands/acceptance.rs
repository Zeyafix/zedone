use super::*;

#[tokio::test]
async fn reads_history_settlement_and_unrelated_changes_keep_prepared_versions_valid() {
    let mut b = Board::new(0).await;
    let original = b.read("card").await;
    let session = format!("{:02x}", b.id).repeat(16);
    b.change("card.begin_editing", &original, json!({})).await;
    let saved = b
        .change(
            "card.autosave",
            &original,
            json!({"session":session,"title":"New","description":"New"}),
        )
        .await;
    let column = b.read("column").await;
    let space = b.read("space").await;
    b.change("card.finish_editing", &saved, json!({"session":session}))
        .await;
    let history = b.history().await;
    assert_eq!(history["versions"].as_array().unwrap().len(), 2);
    b.call("card.list", json!({"space":sid(1),"include_trashed":true}))
        .await;
    b.call("space.create", json!({"space":sid(9),"name":"Unrelated"}))
        .await;
    assert_eq!(b.read("card").await, saved);
    assert_eq!(b.read("column").await, column);
    assert_eq!(b.read("space").await, space);
    b.change("card.set_completed", &saved, json!({"completed":true}))
        .await;
    b.storage.shutdown().await.unwrap();
}

#[tokio::test]
async fn competing_adapter_changes_require_a_new_intention_for_the_loser() {
    let mut b = Board::new(0).await;
    let original = b.read("card").await;
    let peer = Storage::open(b.dir.path().join("device.db")).await.unwrap();
    let other = Service::new(peer.clone());
    let edit = request(
        200,
        "card.edit",
        json!({"space":sid(1),"card":sid(3),"title":"Later","description":"Saved","expected":original["version"]}),
    );
    let complete = request(
        201,
        "card.set_completed",
        json!({"space":sid(1),"card":sid(3),"completed":true,"expected":original["version"]}),
    );
    let (http, ipc) = tokio::join!(
        post(&b.service, "card.edit", &edit, &b.secret),
        ipc::invoke(&other, "card.set_completed", &complete, &b.secret)
    );
    let ipc: Value = serde_json::from_str(&ipc).unwrap();
    let (winner, win_op, win_body, lose_op, lose_body) = if http["status"] == "ok" {
        assert_eq!(ipc["error"]["code"], "stale_state");
        (http, "card.edit", edit, "card.set_completed", complete)
    } else {
        assert_eq!(http["error"]["code"], "stale_state");
        assert_eq!(ipc["status"], "ok");
        (ipc, "card.set_completed", complete, "card.edit", edit)
    };
    b.stale(
        lose_op,
        serde_json::from_str::<Value>(&lose_body).unwrap()["parameters"].clone(),
    )
    .await;
    assert_eq!(
        cli(&b.dir, &b.secret, lose_op, &lose_body)["error"]["code"],
        "stale_state"
    );
    assert_eq!(b.read("card").await, winner["result"]);
    // A new ID and an explicit fresh condition make the losing action valid.
    let mut p = serde_json::from_str::<Value>(&lose_body).unwrap()["parameters"].clone();
    p["expected"] = winner["result"]["version"].clone();
    let latest = b.call(lose_op, p).await;
    peer.shutdown().await.unwrap();
    b.restart().await;
    let mut expected = winner;
    expected["replayed"] = json!(true);
    for entry in 0..3 {
        assert_eq!(b.deliver(entry, win_op, &win_body).await, expected);
        assert_eq!(
            b.deliver(entry, lose_op, &lose_body).await["error"]["code"],
            "stale_state"
        );
    }
    assert_eq!(b.read("card").await, latest);
    b.storage.shutdown().await.unwrap();
}

impl Board {
    async fn reject_old_changes(
        &mut self,
        card: &Value,
        tag: &Value,
        text_version: &Value,
    ) -> Vec<(&'static str, String)> {
        let mut rejected = vec![];
        for (op, extra) in [
            ("card.edit", json!({"title":"Old","description":"Old"})),
            ("card.set_completed", json!({"completed":true})),
            ("card.set_completed", json!({"completed":false})),
            (
                "card.set_tag",
                json!({"tag":sid(4),"present":true,"expected_tag":tag["version"]}),
            ),
            (
                "card.set_tag",
                json!({"tag":sid(4),"present":false,"expected_tag":tag["version"]}),
            ),
            ("card.move", json!({"column":sid(2),"before":null})),
            ("card.move", json!({"column":sid(6),"before":null})),
            ("card.trash", json!({})),
            ("card.restore", json!({})),
            ("card.begin_editing", json!({})),
            (
                "card.autosave",
                json!({"session":"05".repeat(16),"title":"Old","description":"Old"}),
            ),
            ("card.finish_editing", json!({"session":"05".repeat(16)})),
            ("card.restore_text", json!({"text_version":text_version})),
        ] {
            let mut p = json!({"space":sid(1),"card":sid(3),"expected":card["version"]});
            p.as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            rejected.push((op, request(self.id, op, p.clone())));
            self.stale(op, p).await;
        }
        rejected
    }
}

#[tokio::test]
async fn every_card_field_invalidates_every_mutation_at_all_entries_even_after_undo_and_restart() {
    for (index, field) in [
        "title",
        "description",
        "text",
        "assign",
        "unassign",
        "complete",
        "uncomplete",
        "column",
        "order",
        "trash",
        "restore",
    ]
    .into_iter()
    .enumerate()
    {
        let mut b = Board::new(index % 3).await;
        b.call(
            "column.create",
            json!({"space":sid(1),"column":sid(6),"name":"Other"}),
        )
        .await;
        b.call("card.create", json!({"space":sid(1),"column":sid(2),"card":sid(5),"title":"Neighbor","description":""})).await;
        let mut tag = b
            .call(
                "tag.create",
                json!({"space":sid(1),"tag":sid(4),"name":"Work"}),
            )
            .await;
        let mut original = b.read("card").await;
        let preparation = match field {
            "unassign" => Some((
                "card.set_tag",
                json!({"tag":sid(4),"present":true,"expected_tag":tag["version"]}),
            )),
            "uncomplete" => Some(("card.set_completed", json!({"completed":true}))),
            "restore" => Some(("card.trash", json!({}))),
            _ => None,
        };
        if let Some((op, extra)) = preparation {
            original = b.change(op, &original, extra).await;
            tag = b
                .call("tag.read", json!({"space":sid(1),"tag":sid(4)}))
                .await;
        }
        let column = b.read("column").await;
        let space = b.read("space").await;
        let history = b.history().await;
        let title = original["titles"][0].clone();
        let description = original["descriptions"][0].clone();
        let (op, change, mut undo) = match field {
            "title" => (
                "card.edit",
                json!({"title":"New","description":description}),
                json!({"title":title,"description":description}),
            ),
            "description" => (
                "card.edit",
                json!({"title":title,"description":"New"}),
                json!({"title":title,"description":description}),
            ),
            "text" => (
                "card.edit",
                json!({"title":"New","description":"New"}),
                json!({"title":title,"description":description}),
            ),
            "assign" | "unassign" => (
                "card.set_tag",
                json!({"tag":sid(4),"present":field == "assign","expected_tag":tag["version"]}),
                json!({"tag":sid(4),"present":field != "assign","expected_tag":tag["version"]}),
            ),
            "complete" | "uncomplete" => (
                "card.set_completed",
                json!({"completed":field == "complete"}),
                json!({"completed":field != "complete"}),
            ),
            "column" => (
                "card.move",
                json!({"column":sid(6),"before":null}),
                json!({"column":sid(2),"before":sid(5)}),
            ),
            "order" => (
                "card.move",
                json!({"column":sid(2),"before":null}),
                json!({"column":sid(2),"before":sid(5)}),
            ),
            "trash" => ("card.trash", json!({}), json!({})),
            "restore" => ("card.restore", json!({}), json!({})),
            _ => unreachable!(),
        };
        let changed = b.change(op, &original, change).await;
        assert_ne!(changed["version"], original["version"], "{field}");
        if matches!(field, "column" | "order") {
            assert_ne!(changed["positions"], original["positions"], "{field}");
        }
        let current_column = b.read("column").await;
        let current_space = b.read("space").await;
        let current_history = b.history().await;
        let rejected = b
            .reject_old_changes(&original, &tag, &history["versions"][0]["id"])
            .await;
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
        assert_eq!(b.read("card").await, changed, "{field}");
        assert_eq!(b.read("column").await, current_column, "{field}");
        assert_eq!(b.read("space").await, current_space, "{field}");
        assert_eq!(b.history().await, current_history, "{field}");
        // A freshly prepared reverse succeeds; identical visible fields do not
        // revive any rejected ID or its original causal condition.
        let reverse = match field {
            "trash" => "card.restore",
            "restore" => "card.trash",
            _ => op,
        };
        if op == "card.set_tag" {
            let fresh_tag = b
                .call("tag.read", json!({"space":sid(1),"tag":sid(4)}))
                .await;
            undo["expected_tag"] = fresh_tag["version"].clone();
        }
        let returned = b.change(reverse, &changed, undo).await;
        assert_ne!(returned["version"], original["version"], "{field}");
        for name in [
            "titles",
            "descriptions",
            "tags",
            "completed",
            "columns",
            "state",
        ] {
            assert_eq!(returned[name], original[name], "{field}/{name}");
        }
        let listed = b
            .call("card.list", json!({"space":sid(1),"include_trashed":true}))
            .await;
        let order: Vec<_> = listed["cards"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["card"].clone())
            .collect();
        assert_eq!(order, [json!(sid(3)), json!(sid(5))], "{field}");
        let returned_history = b.history().await;
        let returned_column = b.read("column").await;
        let returned_space = b.read("space").await;
        b.restart().await;
        for (op, body) in rejected {
            for entry in 0..3 {
                assert_eq!(
                    b.deliver(entry, op, &body).await,
                    json!({"status":"error","error":{"code":"stale_state"}}),
                    "{field}/{op}"
                );
            }
        }
        b.replay_all().await;
        assert_eq!(b.read("card").await, returned, "{field}");
        assert_eq!(b.read("column").await, returned_column, "{field}");
        assert_eq!(b.read("space").await, returned_space, "{field}");
        assert_eq!(b.history().await, returned_history, "{field}");
        b.storage.shutdown().await.unwrap();
    }
}

#[tokio::test]
async fn container_changes_reject_old_deletions_without_partial_effects_on_every_entry() {
    for (index, field) in [
        "add",
        "trash",
        "restore",
        "move_in",
        "move_out",
        "replace",
        "column_name",
        "column_order",
        "new_column",
        "trash_column",
        "restore_column",
        "space_name",
        "new_tag",
        "tag_name",
        "delete_tag",
    ]
    .into_iter()
    .enumerate()
    {
        let mut b = Board::new(index % 3).await;
        let other = b
            .call(
                "column.create",
                json!({"space":sid(1),"column":sid(6),"name":"Other"}),
            )
            .await;
        let mut tag = b
            .call(
                "tag.create",
                json!({"space":sid(1),"tag":sid(4),"name":"Work"}),
            )
            .await;
        let mut card = b.read("card").await;
        if matches!(field, "tag_name" | "delete_tag") {
            card = b
                .change(
                    "card.set_tag",
                    &card,
                    json!({"tag":sid(4),"present":true,"expected_tag":tag["version"]}),
                )
                .await;
            tag = b
                .call("tag.read", json!({"space":sid(1),"tag":sid(4)}))
                .await;
        }
        match field {
            "restore" => {
                card = b.change("card.trash", &card, json!({})).await;
            }
            "move_in" => {
                card = b
                    .change("card.move", &card, json!({"column":sid(6),"before":null}))
                    .await;
            }
            "restore_column" => {
                b.call(
                    "column.trash",
                    json!({"space":sid(1),"column":sid(6),"expected":other["version"]}),
                )
                .await;
            }
            _ => {}
        }
        let column = b.read("column").await;
        let space = b.read("space").await;
        match field {
            "add" | "replace" => {
                if field == "replace" {
                    b.change("card.trash", &card, json!({})).await;
                }
                b.call("card.create", json!({"space":sid(1),"column":sid(2),"card":sid(8),"title":"New member","description":"Saved"})).await;
            }
            "trash" | "restore" => {
                b.change(&format!("card.{field}"), &card, json!({})).await;
            }
            "move_in" | "move_out" => {
                b.change(
                    "card.move",
                    &card,
                    json!({"column":sid(if field == "move_in" {2} else {6}),"before":null}),
                )
                .await;
            }
            "column_name" => {
                b.call("column.rename", json!({"space":sid(1),"column":sid(2),"name":"Renamed","expected":column["version"]})).await;
            }
            "column_order" => {
                b.call("column.move", json!({"space":sid(1),"column":sid(2),"before":null,"expected":column["version"]})).await;
            }
            "new_column" => {
                b.call(
                    "column.create",
                    json!({"space":sid(1),"column":sid(7),"name":"New"}),
                )
                .await;
            }
            "trash_column" | "restore_column" => {
                let current = b
                    .call("column.read", json!({"space":sid(1),"column":sid(6)}))
                    .await;
                let op = if field == "trash_column" {
                    "column.trash"
                } else {
                    "column.restore"
                };
                b.call(
                    op,
                    json!({"space":sid(1),"column":sid(6),"expected":current["version"]}),
                )
                .await;
            }
            "space_name" => {
                b.call(
                    "space.rename",
                    json!({"space":sid(1),"name":"Renamed","expected":space["version"]}),
                )
                .await;
            }
            "new_tag" => {
                b.call(
                    "tag.create",
                    json!({"space":sid(1),"tag":sid(7),"name":"New"}),
                )
                .await;
            }
            "tag_name" => {
                b.call(
                    "tag.rename",
                    json!({"space":sid(1),"tag":sid(4),"name":"Renamed","expected":tag["version"]}),
                )
                .await;
            }
            "delete_tag" => {
                b.call(
                    "tag.delete",
                    json!({"space":sid(1),"tag":sid(4),"expected":tag["version"]}),
                )
                .await;
            }
            _ => unreachable!(),
        }
        let current_card = b.read("card").await;
        let current_column = b.read("column").await;
        let current_space = b.read("space").await;
        let history = b.history().await;
        let column_changed = matches!(
            field,
            "add"
                | "trash"
                | "restore"
                | "move_in"
                | "move_out"
                | "replace"
                | "column_name"
                | "column_order"
                | "tag_name"
                | "delete_tag"
        );
        if column_changed {
            b.stale(
                "column.trash",
                json!({"space":sid(1),"column":sid(2),"expected":column["version"]}),
            )
            .await;
        } else {
            assert_eq!(
                current_column["version"], column["version"],
                "{field}: unrelated change"
            );
        }
        b.stale(
            "space.trash",
            json!({"space":sid(1),"expected":space["version"]}),
        )
        .await;
        assert_eq!(b.read("card").await, current_card, "{field}");
        assert_eq!(b.read("column").await, current_column, "{field}");
        assert_eq!(b.read("space").await, current_space, "{field}");
        assert_eq!(b.history().await, history, "{field}");
        let (kind, fresh) = if column_changed {
            ("column", current_column)
        } else {
            ("space", current_space)
        };
        let mut p = json!({"space":sid(1),"expected":fresh["version"]});
        if kind == "column" {
            p["column"] = json!(sid(2));
        }
        let trashed = b.call(&format!("{kind}.trash"), p.clone()).await;
        p["expected"] = trashed["version"].clone();
        b.call(&format!("{kind}.restore"), p).await;
        let restored_column = b.read("column").await;
        let restored_space = b.read("space").await;
        let restored_card = b.read("card").await;
        let restored_history = b.history().await;
        b.restart().await;
        b.replay_all().await;
        assert_eq!(b.read("column").await, restored_column, "{field}");
        assert_eq!(b.read("space").await, restored_space, "{field}");
        assert_eq!(b.read("card").await, restored_card, "{field}");
        assert_eq!(b.history().await, restored_history, "{field}");
        b.storage.shutdown().await.unwrap();
    }
}
