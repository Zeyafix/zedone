use super::spaces::{cli, sid};
use super::*;

#[tokio::test]
async fn catalog_permissions_delete_replay_expiry_revocation_and_http_ipc_cli() {
    let (dir, db, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    db.daily_tick(true).await.unwrap();
    let folder = service
        .select_backup_folder(&secret, None, true)
        .await
        .unwrap();
    let body = request(51, "backups.catalog", json!({"folder":folder.id}));
    let http = post(&service, "backups.catalog", &body, &secret).await;
    assert_eq!(http["status"], "ok");
    assert_eq!(http["result"]["copies"].as_array().unwrap().len(), 1);
    assert_eq!(
        serde_json::from_str::<Value>(
            &ipc::invoke(&service, "backups.catalog", &body, &secret).await
        )
        .unwrap(),
        http
    );
    assert_eq!(cli(&dir, &secret, "backups.catalog", &body), http);
    let copy = &http["result"]["copies"][0];
    let limited = Arc::new(PendingAccess::generate().unwrap());
    let l = limited.clone();
    let a = admin.clone();
    db.timed_transact(move |ctx, clock| {
        ctx.issue_access(
            &a.credential(),
            clock,
            &l,
            &GrantScope::Spaces([(sid(1), SpacePermission::Read)].into_iter().collect()),
            None,
        )
    })
    .await
    .unwrap();
    assert_eq!(
        post(&service, "backups.catalog", &body, &key(&limited)).await["error"]["code"],
        "access_denied"
    );
    let readonly = service
        .select_backup_folder(
            &key(&limited),
            Some(
                dir.path()
                    .join("device.db.backups")
                    .to_str()
                    .unwrap()
                    .into(),
            ),
            false,
        )
        .await
        .unwrap();
    let denied = request(
        52,
        "backups.catalog.delete",
        json!({"folder":readonly.id,"copy":copy["id"],"expected":copy["digest"]}),
    );
    assert_eq!(
        post(&service, "backups.catalog.delete", &denied, &key(&limited)).await["error"]["code"],
        "access_denied"
    );
    let invalid = request(
        53,
        "backups.catalog.read",
        json!({"folder":folder.id,"copy":"../local-session.json","expected":copy["digest"]}),
    );
    assert_eq!(
        post(&service, "backups.catalog.read", &invalid, &secret).await["error"]["code"],
        "invalid_request"
    );
    let remove = request(
        54,
        "backups.catalog.delete",
        json!({"folder":folder.id,"copy":copy["id"],"expected":copy["digest"]}),
    );
    let result = post(&service, "backups.catalog.delete", &remove, &secret).await;
    assert_eq!(result["status"], "ok");
    let replay = cli(&dir, &secret, "backups.catalog.delete", &remove);
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], result["result"]);
    let read = post(&service, "backups.catalog", &body, &secret).await;
    assert_eq!(read["result"]["copies"].as_array().unwrap().len(), 0);
    let forgot = request(55, "backups.folder.forget", json!({"folder":folder.id}));
    assert_eq!(
        post(&service, "backups.folder.forget", &forgot, &secret).await["status"],
        "ok"
    );
    assert_eq!(
        post(&service, "backups.catalog", &body, &secret).await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        post(&service, "backups.catalog.delete", &remove, &secret).await["error"]["code"],
        "access_denied"
    );
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn location_switch_is_receipted_and_stale_requests_cannot_switch_back() {
    let (dir, db, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    db.daily_tick(true).await.unwrap();
    let target = dir.path().join("target");
    std::fs::create_dir(&target).unwrap();
    let folder = service
        .select_backup_folder(&secret, Some(target.to_str().unwrap().into()), true)
        .await
        .unwrap();
    let current = post(
        &service,
        "backups.location.inspect",
        &request(61, "backups.location.inspect", json!({})),
        &secret,
    )
    .await;
    let change = request(
        62,
        "backups.location.change",
        json!({"folder":folder.id,"expected":current["result"]["version"]}),
    );
    let changed = post(&service, "backups.location.change", &change, &secret).await;
    assert_eq!(changed["status"], "ok");
    let replay = cli(&dir, &secret, "backups.location.change", &change);
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], changed["result"]);
    let again = request(
        63,
        "backups.location.change",
        json!({"folder":folder.id,"expected":current["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "backups.location.change", &again, &secret).await["error"]["code"],
        "stale_state"
    );
    assert_eq!(std::fs::read_dir(&target).unwrap().count(), 1);
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn purging_a_space_keeps_its_copies_until_separate_authorized_deletion() {
    let (dir, db, service, admin) = super::cards::setup_cards().await;
    let secret = key(&admin);
    db.daily_tick(true).await.unwrap();
    let folder = service
        .select_backup_folder(&secret, None, true)
        .await
        .unwrap();
    let before = post(
        &service,
        "backups.catalog",
        &request(71, "backups.catalog", json!({"folder":folder.id})),
        &secret,
    )
    .await;
    let space = post(
        &service,
        "space.read",
        &request(72, "space.read", json!({"space":sid(1)})),
        &secret,
    )
    .await;
    assert_eq!(
        post(
            &service,
            "space.trash",
            &request(
                73,
                "space.trash",
                json!({"space":sid(1),"expected":space["result"]["version"]})
            ),
            &secret
        )
        .await["status"],
        "ok"
    );
    let preview = post(
        &service,
        "trash.inspect",
        &request(
            74,
            "trash.inspect",
            json!({"space":sid(1),"selection":{"kind":"space"}}),
        ),
        &secret,
    )
    .await;
    assert_eq!(post(&service,"trash.purge",&request(75,"trash.purge",json!({"space":sid(1),"selection":{"kind":"space"},"expected":preview["result"]["version"]})),&secret).await["status"],"ok");
    db.daily_tick(true).await.unwrap();
    let after = post(
        &service,
        "backups.catalog",
        &request(76, "backups.catalog", json!({"folder":folder.id})),
        &secret,
    )
    .await;
    assert_eq!(after["result"], before["result"]);
    let c = &after["result"]["copies"][0];
    let delete = request(
        77,
        "backups.catalog.delete",
        json!({"folder":folder.id,"copy":c["id"],"expected":c["digest"]}),
    );
    assert_eq!(
        cli(&dir, &secret, "backups.catalog.delete", &delete)["status"],
        "ok"
    );
    assert_eq!(
        post(
            &service,
            "backups.catalog",
            &request(78, "backups.catalog", json!({"folder":folder.id})),
            &secret
        )
        .await["result"]["copies"],
        json!([])
    );
    db.shutdown().await.unwrap();
}
