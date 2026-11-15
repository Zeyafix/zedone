use super::{
    spaces::{cli, sid},
    *,
};
use zedone_core::{
    invitations::InvitationVerifier,
    pairing::{self, PendingVerifier},
    wire::decode_request,
};

#[tokio::test]
async fn administrators_share_pending_inspect_cancel_and_list_across_http_ipc_cli() {
    let (dir, db, service, admin, member) = setup().await;
    let secret = key(&admin);
    let create = request(
        1,
        "invitation.create",
        json!({"invitation":sid(1),"verifier":vec![1;32]}),
    );
    assert_eq!(
        post(&service, "invitation.create", &create, &secret).await["status"],
        "ok"
    );
    let pending = request(
        2,
        "pairing.request",
        json!({"request":sid(2),"invitation":sid(1),"label":"Телефон 空間","access_verifier":vec![2;32]}),
    );
    // Trusted fixture calls the same core handler used only behind TLS.
    db.timed_transact(move |tx, clock| {
        Ok(pairing::request(
            tx,
            clock,
            &decode_request::<pairing::RequestPairing>(pending.as_bytes())?,
            InvitationVerifier([1; 32]),
            PendingVerifier([3; 32]),
        )?)
    })
    .await
    .unwrap()
    .unwrap();
    let inspect = request(3, "pairing.inspect", json!({"request":sid(2)}));
    let read = post(&service, "pairing.inspect", &inspect, &secret).await;
    assert_eq!(read["result"]["status"], "pending");
    let replay: Value =
        serde_json::from_str(&ipc::invoke(&service, "pairing.inspect", &inspect, &secret).await)
            .unwrap();
    assert_eq!(replay["result"], read["result"]);
    assert_eq!(replay["replayed"], true);
    assert_eq!(cli(&dir, &secret, "pairing.inspect", &inspect), replay);
    let cancel = request(
        4,
        "pairing.cancel",
        json!({"request":sid(2),"expected":read["result"]["version"]}),
    );
    let cancelled = cli(&dir, &secret, "pairing.cancel", &cancel);
    assert_eq!(cancelled["result"]["status"], "cancelled");
    let replay = post(&service, "pairing.cancel", &cancel, &secret).await;
    assert_eq!(replay["result"], cancelled["result"]);
    assert_eq!(replay["replayed"], true);
    let list = request(5, "pairing.list", json!({"after":null}));
    let listed = post(&service, "pairing.list", &list, &secret).await;
    assert_eq!(listed["result"]["entries"], json!([cancelled["result"]]));
    let replay = cli(&dir, &secret, "pairing.list", &list);
    assert_eq!(replay["result"], listed["result"]);
    assert_eq!(replay["replayed"], true);
    for (op, body) in [
        ("pairing.inspect", &inspect),
        ("pairing.cancel", &cancel),
        ("pairing.list", &list),
    ] {
        assert_eq!(
            post(&service, op, body, &key(&member)).await["error"]["code"],
            "access_denied"
        );
    }
    let stale = request(
        6,
        "pairing.cancel",
        json!({"request":sid(2),"expected":read["result"]["version"]}),
    );
    assert_eq!(
        post(&service, "pairing.cancel", &stale, &secret).await["error"]["code"],
        "stale_state"
    );
    for op in ["pairing.request", "pairing.confirm"] {
        assert_eq!(
            post(&service, op, &request(7, op, json!({})), &secret).await["error"]["code"],
            "invalid_request"
        );
    }
    assert!(!listed.to_string().contains("verifier"));
    db.shutdown().await.unwrap();
}

#[tokio::test]
async fn confirmation_rejection_and_unlink_share_http_ipc_cli_receipts_and_authority() {
    let (dir, db, service, admin, member) = setup().await;
    let secret = key(&admin);
    for reject in [false, true] {
        let n = if reject { 20 } else { 10 };
        let create = request(
            n,
            "invitation.create",
            json!({"invitation":sid(n),"verifier":vec![n;32]}),
        );
        assert_eq!(
            post(&service, "invitation.create", &create, &secret).await["status"],
            "ok"
        );
        let pending = request(
            n + 1,
            "pairing.request",
            json!({"request":sid(n),"invitation":sid(n),"label":"Phone","access_verifier":vec![n;32]}),
        );
        let snapshot = db
            .timed_transact(move |tx, clock| {
                Ok(pairing::request(
                    tx,
                    clock,
                    &decode_request::<pairing::RequestPairing>(pending.as_bytes())?,
                    InvitationVerifier([n; 32]),
                    PendingVerifier([n; 32]),
                )?)
            })
            .await
            .unwrap()
            .unwrap()
            .result;
        let op = if reject {
            "pairing.reject"
        } else {
            "pairing.confirm"
        };
        let p = if reject {
            json!({"request":sid(n),"expected":snapshot.version})
        } else {
            json!({"request":sid(n),"expected":snapshot.version,"scope":{"kind":"administration"},"expires_at_unix_ms":null})
        };
        let body = request(n + 2, op, p.clone());
        assert_eq!(
            post(&service, op, &body, &key(&member)).await["error"]["code"],
            "access_denied"
        );
        let completed = cli(&dir, &secret, op, &body);
        assert_eq!(
            completed["result"]["status"],
            if reject { "rejected" } else { "confirmed" }
        );
        let replay: Value =
            serde_json::from_str(&ipc::invoke(&service, op, &body, &secret).await).unwrap();
        assert_eq!(replay["result"], completed["result"]);
        assert_eq!(replay["replayed"], true);
        assert_eq!(post(&service, op, &body, &secret).await, replay);
        if !reject {
            let unlink = request(
                n + 3,
                "pairing.unlink",
                json!({"request":sid(n),"expected":completed["result"]["version"]}),
            );
            assert_eq!(
                post(&service, "pairing.unlink", &unlink, &key(&member)).await["error"]["code"],
                "access_denied"
            );
            let result = post(&service, "pairing.unlink", &unlink, &secret).await;
            assert_eq!(result["result"]["status"], "unlinked");
            let replay = cli(&dir, &secret, "pairing.unlink", &unlink);
            assert_eq!(replay["result"], result["result"]);
            assert_eq!(replay["replayed"], true);
            let stale = request(n + 4, op, p);
            assert_eq!(
                post(&service, op, &stale, &secret).await["error"]["code"],
                "stale_state"
            );
        }
        let mut changed: Value = serde_json::from_str(&body).unwrap();
        changed["parameters"]["request"] = json!(sid(99));
        assert_eq!(
            post(&service, op, &changed.to_string(), &secret).await["error"]["code"],
            "command_mismatch"
        );
    }
    db.shutdown().await.unwrap();
}
