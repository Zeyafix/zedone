use super::*;
use zedone_core::access::SpacePermission;

pub(super) fn sid(n: u8) -> String {
    format!("{n:032x}")
}
pub(super) async fn setup_spaces() -> (TempDir, Storage, Service, Arc<PendingAccess>) {
    let dir = TempDir::new().unwrap();
    let storage = Storage::open(dir.path().join("device.db")).await.unwrap();
    let admin = Arc::new(PendingAccess::generate().unwrap());
    let a = admin.clone();
    storage
        .timed_transact(move |ctx, _| ctx.bootstrap_access(&a))
        .await
        .unwrap();
    let service = Service::new(storage.clone());
    (dir, storage, service, admin)
}
pub(super) fn cli(dir: &TempDir, secret: &str, operation: &str, body: &str) -> Value {
    let file = dir.path().join("space-command.key");
    std::fs::write(&file, secret).unwrap();
    let mut child = Command::new(
        std::env::var("ZEDONE_TEST_CLI").unwrap_or_else(|_| env!("CARGO_BIN_EXE_zedone").into()),
    )
    .arg("--database")
    .arg(dir.path().join("device.db"))
    .arg("--key-file")
    .arg(file)
    .args(["--operation", operation])
    .stdin(Stdio::piped())
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()
    .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(body.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.stderr.is_empty());
    let response: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(output.status.success(), response["status"] == "ok");
    response
}
async fn read(service: &Service, secret: &str, id: u8, space: u8) -> Value {
    post(
        service,
        "space.read",
        &request(id, "space.read", json!({"space":sid(space)})),
        secret,
    )
    .await
}

#[tokio::test]
async fn the_whole_lifecycle_uses_one_committed_result_across_http_ipc_and_real_cli() {
    let (dir, storage, service, admin) = setup_spaces().await;
    let secret = key(&admin);
    let creation = request(
        1,
        "space.create",
        json!({"space":sid(1),"name":"Ёж 空間 😀"}),
    );
    let created = post(&service, "space.create", &creation, &secret).await;
    assert_eq!(created["status"], "ok");
    assert_eq!(created["result"]["names"], json!(["Ёж 空間 😀"]));
    let duplicate: Value =
        serde_json::from_str(&ipc::invoke(&service, "space.create", &creation, &secret).await)
            .unwrap();
    assert_eq!(duplicate["replayed"], true);
    assert_eq!(duplicate["result"], created["result"]);
    assert_eq!(cli(&dir, &secret, "space.create", &creation), duplicate);
    let rename = request(
        2,
        "space.rename",
        json!({"space":sid(1),"name":"Renamed","expected":created["result"]["version"]}),
    );
    let renamed = cli(&dir, &secret, "space.rename", &rename);
    assert_eq!(renamed["result"]["names"], json!(["Renamed"]));
    let repeat = post(&service, "space.rename", &rename, &secret).await;
    assert_eq!(repeat["replayed"], true);
    assert_eq!(repeat["result"], renamed["result"]);
    let trash = request(
        3,
        "space.trash",
        json!({"space":sid(1),"expected":renamed["result"]["version"]}),
    );
    let trashed = post(&service, "space.trash", &trash, &secret).await;
    assert_eq!(trashed["result"]["state"], "trashed");
    let restore = request(
        4,
        "space.restore",
        json!({"space":sid(1),"expected":trashed["result"]["version"]}),
    );
    let restored: Value =
        serde_json::from_str(&ipc::invoke(&service, "space.restore", &restore, &secret).await)
            .unwrap();
    assert_eq!(restored["result"]["state"], "active");
    assert_eq!(
        cli(&dir, &secret, "space.trash", &trash)["result"],
        trashed["result"]
    );
    assert_eq!(
        read(&service, &secret, 5, 1).await["result"],
        restored["result"]
    );
    let listed = cli(
        &dir,
        &secret,
        "space.list",
        &request(6, "space.list", json!({"include_trashed":false})),
    );
    assert_eq!(listed["result"]["spaces"], json!([restored["result"]]));
    let reading = request(7, "space.read", json!({"space":sid(1)}));
    let original: Value =
        serde_json::from_str(&ipc::invoke(&service, "space.read", &reading, &secret).await)
            .unwrap();
    let replay = cli(&dir, &secret, "space.read", &reading);
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["result"], original["result"]);
    // The CLI can reopen the same installation after the host worker closes.
    storage.shutdown().await.unwrap();
    assert_eq!(
        cli(&dir, &secret, "space.create", &creation)["result"],
        created["result"]
    );
    let current = cli(
        &dir,
        &secret,
        "space.read",
        &request(8, "space.read", json!({"space":sid(1)})),
    );
    assert_eq!(current["result"], restored["result"]);
}

#[tokio::test]
async fn readonly_subject_can_create_without_changing_old_rights_and_revocation_blocks_every_entry()
{
    let (dir, storage, service, admin) = setup_spaces().await;
    let base = post(
        &service,
        "space.create",
        &request(
            1,
            "space.create",
            json!({"space":sid(1),"name":"Shared read"}),
        ),
        &key(&admin),
    )
    .await;
    let member = Arc::new(PendingAccess::generate().unwrap());
    let (a, m) = (admin.clone(), member.clone());
    storage
        .timed_transact(move |ctx, clock| {
            ctx.issue_access(
                &a.credential(),
                clock,
                &m,
                &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
                None,
            )
        })
        .await
        .unwrap();
    let secret = key(&member);
    let creation = request(2, "space.create", json!({"space":sid(2),"name":"Mine"}));
    let mine = cli(&dir, &secret, "space.create", &creation);
    assert_eq!(mine["status"], "ok");
    assert_eq!(
        post(
            &service,
            "space.rename",
            &request(
                3,
                "space.rename",
                json!({"space":sid(1),"name":"No","expected":base["result"]["version"]})
            ),
            &secret
        )
        .await["error"]["code"],
        "access_denied"
    );
    assert_eq!(
        read(&service, &key(&admin), 4, 2).await["error"]["code"],
        "access_denied"
    );
    let inspected = post(
        &service,
        "access.inspect",
        &inspect(5, member.subject()),
        &key(&admin),
    )
    .await;
    let revoke = request(
        6,
        "access.revoke",
        json!({"subject":member.subject(),"expected":inspected["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "access.revoke", &revoke, &key(&admin)).await["status"],
        "ok"
    );
    assert_eq!(
        post(&service, "space.create", &creation, &secret).await["error"]["code"],
        "access_denied"
    );
    let denied: Value =
        serde_json::from_str(&ipc::invoke(&service, "space.create", &creation, &secret).await)
            .unwrap();
    assert_eq!(denied["error"]["code"], "access_denied");
    assert_eq!(
        cli(&dir, &secret, "space.create", &creation)["error"]["code"],
        "access_denied"
    );
}

#[tokio::test]
async fn space_decoders_reject_forged_authority_bad_ids_duplicate_fields_and_operation_mismatch() {
    let (_dir, _storage, service, admin) = setup_spaces().await;
    for parameters in [
        json!({"space":sid(1),"name":"x","subject":admin.subject()}),
        json!({"space":sid(1),"name":"x","permissions":"read_write"}),
        json!({"space":"0".repeat(32),"name":"x"}),
        json!({"space":"A".repeat(32),"name":"x"}),
        json!({"space":"../outside","name":"x"}),
    ] {
        assert_eq!(
            post(
                &service,
                "space.create",
                &request(1, "space.create", parameters),
                &key(&admin)
            )
            .await["error"]["code"],
            "invalid_request"
        );
    }
    let valid = request(1, "space.create", json!({"space":sid(1),"name":"x"}));
    for operation in ["%FF", "%C3%28", "space.unknown"] {
        assert_eq!(
            post(&service, operation, &valid, &key(&admin)).await["error"]["code"],
            "invalid_request"
        );
    }
    let duplicate = valid.replace("\"name\":", "\"name\":\"hidden\",\"name\":");
    assert_eq!(
        post(&service, "space.create", &duplicate, &key(&admin)).await["error"]["code"],
        "invalid_request"
    );
    assert_eq!(
        post(&service, "space.read", &valid, &key(&admin)).await["error"]["code"],
        "invalid_request"
    );
    assert_eq!(
        post(
            &service,
            "space.list",
            &request(
                2,
                "space.list",
                json!({"include_trashed":false,"subject":admin.subject()})
            ),
            &key(&admin)
        )
        .await["error"]["code"],
        "invalid_request"
    );
    let actual = post(
        &service,
        "space.list",
        &request(3, "space.list", json!({"include_trashed":true})),
        &key(&admin),
    )
    .await;
    assert_eq!(actual["result"]["spaces"], json!([]));
}
