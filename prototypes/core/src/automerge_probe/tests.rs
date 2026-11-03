use super::*;

#[test]
fn restart_retains_history_identity_and_peer_then_resumes_dropped_message() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "prepare").unwrap();
    run(directory.path(), "resume").unwrap();
    assert!(run(directory.path(), "prepare").is_err());
}

#[test]
fn failed_write_keeps_document_actor_and_peer_together() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "prepare").unwrap();
    let path = directory.path().join("automerge-probe.sqlite3");
    let mut store = Store::open(&path).unwrap();
    let before = store.read("A").unwrap().fingerprint().unwrap();
    let mut candidate = store.read("A").unwrap();
    candidate.set_title("Must never be committed").unwrap();
    candidate.peer = State::new();
    store
        .0
        .execute_batch(
            "CREATE TRIGGER reject_save AFTER UPDATE ON replicas
        BEGIN SELECT RAISE(ABORT, 'synthetic write failure'); END;",
        )
        .unwrap();
    assert!(store.write("A", &candidate).is_err());
    assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), before);
    store.0.execute_batch("DROP TRIGGER reject_save;").unwrap();
    drop(store);
    assert_eq!(
        Store::open(&path)
            .unwrap()
            .read("A")
            .unwrap()
            .fingerprint()
            .unwrap(),
        before
    );
    run(directory.path(), "resume").unwrap();
}

#[test]
fn corrupt_document_or_peer_is_rejected_without_reinitialising_data() {
    for column in ["document", "peer"] {
        let directory = tempfile::tempdir().unwrap();
        run(directory.path(), "prepare").unwrap();
        let path = directory.path().join("automerge-probe.sqlite3");
        let store = Store::open(&path).unwrap();
        let other = store.read("B").unwrap().fingerprint().unwrap();
        store
            .0
            .execute(
                &format!("UPDATE replicas SET {column} = x'ff' WHERE id = 'A'"),
                [],
            )
            .unwrap();
        drop(store);
        let store = Store::open(&path).unwrap();
        assert!(store.read("A").is_err());
        assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), other);
        let raw: Vec<u8> = store
            .0
            .query_row(
                &format!("SELECT {column} FROM replicas WHERE id = 'A'"),
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(raw, [255]);
    }
}

#[test]
fn locked_database_never_confirms_the_candidate_write() {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "prepare").unwrap();
    let path = directory.path().join("automerge-probe.sqlite3");
    let mut store = Store::open(&path).unwrap();
    store.0.busy_timeout(Duration::from_millis(10)).unwrap();
    let mut candidate = store.read("A").unwrap();
    let before = candidate.fingerprint().unwrap();
    candidate.set_title("Blocked").unwrap();
    let lock = Connection::open(&path).unwrap();
    lock.execute_batch("BEGIN EXCLUSIVE").unwrap();
    assert!(store.write("A", &candidate).is_err());
    lock.execute_batch("ROLLBACK").unwrap();
    assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), before);
}
