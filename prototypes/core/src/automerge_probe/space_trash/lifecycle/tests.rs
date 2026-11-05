use super::*;

fn prepared() -> (tempfile::TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let mut store = Store::open(&dir.path().join("lifecycle.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    (dir, store)
}

fn restore_initial(store: &mut Store) {
    let before = store.read("A").unwrap().fingerprint().unwrap();
    restore(store, "initial", &before).unwrap();
}

// Compare user data, not identities of newly created field assignments/history
// slots. Entity identity, trash groups and field multiplicity ARE preserved.
fn semantic(doc: &Automerge) -> Value {
    let nodes = view(doc).unwrap();
    let content: BTreeMap<_, _> = nodes
        .iter()
        .map(|(k, n)| {
            let fields: BTreeMap<_, _> = registers(n)
                .into_iter()
                .map(|(f, v)| {
                    let mut values: Vec<_> = v.iter().map(|v| v.value.clone()).collect();
                    values.sort();
                    (f, values)
                })
                .collect();
            (
                k,
                json!({"id": n.id, "kind": n.kind, "parent": n.parent,
            "fields": fields, "location": location(&nodes, k)}),
            )
        })
        .collect();
    let versions: Vec<_> = history::versions(doc)
        .unwrap()
        .iter()
        .map(|v| json!([v.card, v.title, v.text]))
        .collect();
    json!({"nodes": content, "history": versions})
}

#[test]
fn snapshot_is_complete_stable_and_excludes_other_space_and_local_metadata() {
    let (_dir, mut store) = prepared();
    store
        .0
        .execute_batch(
            "CREATE TABLE other_space(data TEXT); INSERT INTO other_space VALUES ('НЕ В СНИМКЕ');
        CREATE TABLE permissions(data TEXT); INSERT INTO permissions VALUES ('LOCAL RIGHTS');",
        )
        .unwrap();
    let snapshot = read_snapshot(&store, "initial").unwrap();
    let a = store.read("A").unwrap();
    assert_eq!(a.doc.get_heads(), snapshot.load().unwrap().get_heads());
    assert_eq!(semantic(&a.doc), semantic(&snapshot.load().unwrap()));
    assert!(Snapshot::capture(&fixture(true).unwrap().doc).is_err());
    let json = serde_json::to_string(&snapshot).unwrap();
    assert!(!json.contains("peer") && !json.contains("LOCAL RIGHTS"));
    restore_initial(&mut store);
    assert_eq!(
        store
            .0
            .query_row("SELECT data FROM other_space", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "НЕ В СНИМКЕ"
    );
    assert_eq!(
        store
            .0
            .query_row("SELECT data FROM permissions", [], |r| r
                .get::<_, String>(0))
            .unwrap(),
        "LOCAL RIGHTS"
    );
    assert_eq!(semantic(&a.doc), semantic(&store.read("A").unwrap().doc));
}

#[test]
fn restoring_fields_history_trash_groups_and_deleted_tag_is_a_new_action() {
    let (_dir, mut store) = prepared();
    let original = store.read("A").unwrap();
    let mut a = store.read("A").unwrap();
    history::save_pair(&mut a.doc, "k0", "Позже", "Сохранено позже").unwrap();
    let nodes = view(&a.doc).unwrap();
    set_container(&mut a.doc, "c0", &scope(&nodes, "c0"), "trash").unwrap();
    purge::purge(&mut a.doc, "s0", &["c0"], false).unwrap();
    let nodes = view(&a.doc).unwrap();
    let mut tx = a.doc.transaction();
    put(&mut tx, &nodes["tag"].id, "life", "purged").unwrap();
    tx.commit();
    let heads = a.doc.get_heads();
    let changes = a.doc.get_changes(&[]).len();
    store.write("A", &a).unwrap();
    restore_initial(&mut store);
    let restored = store.read("A").unwrap();
    assert_eq!(semantic(&original.doc), semantic(&restored.doc));
    assert_ne!(restored.doc.get_heads(), heads);
    assert!(restored.doc.get_changes(&[]).len() > changes);
    assert_eq!(restored.doc.get_actor(), a.doc.get_actor());
    assert_eq!(restored.peer.encode(), a.peer.encode());
    assert_eq!(
        semantic(
            &read_snapshot(&store, "before-restore")
                .unwrap()
                .load()
                .unwrap()
        ),
        semantic(&a.doc)
    );
}

#[test]
fn snapshot_conflicts_survive_restoration_and_still_protect_their_history() {
    let (_dir, mut store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    history::save_pair(&mut a.doc, "k0", "Первый", "Первый текст").unwrap();
    history::save_pair(&mut b.doc, "k0", "Второй", "Второй текст").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    save_snapshot(&mut store, "conflict", &a.doc).unwrap();
    let before = semantic(&a.doc);
    let nodes = view(&a.doc).unwrap();
    let mut tx = a.doc.transaction();
    put(&mut tx, &nodes["k0"].id, "title", "Решено").unwrap();
    put(&mut tx, &nodes["k0"].id, "text", "Решено").unwrap();
    tx.commit();
    store.write("A", &a).unwrap();
    restore(&mut store, "conflict", &a.fingerprint().unwrap()).unwrap();
    let mut result = store.read("A").unwrap();
    assert_eq!(semantic(&result.doc), before);
    let nodes = view(&result.doc).unwrap();
    assert!(nodes["k0"].blocked());
    let plan = history::preview(&result.doc, "s0", None).unwrap();
    history::clear(&mut result.doc, &plan).unwrap();
    let titles: Vec<_> = history::versions(&result.doc)
        .unwrap()
        .iter()
        .map(|v| v.title.clone())
        .collect();
    assert_eq!(titles.len(), 2);
    assert!(titles.contains(&"Первый".into()) && titles.contains(&"Второй".into()));
}

#[test]
fn safety_copy_reverses_restore_and_known_new_objects_are_removed() {
    let (_dir, mut store) = prepared();
    let mut a = store.read("A").unwrap();
    create(&mut a.doc, "new-a", "c0").unwrap();
    history::save_pair(&mut a.doc, "new-a", "Новая", "Новая история").unwrap();
    let before = semantic(&a.doc);
    store.write("A", &a).unwrap();
    restore_initial(&mut store);
    assert_eq!(
        location(&view(&store.read("A").unwrap().doc).unwrap(), "new-a"),
        "purged"
    );
    let observed = store.read("A").unwrap().fingerprint().unwrap();
    restore(&mut store, "before-restore", &observed).unwrap();
    assert_eq!(semantic(&store.read("A").unwrap().doc), before);
}

#[test]
fn pause_survives_restart_and_late_edits_and_creations_survive_resume() {
    let (dir, mut store) = prepared();
    let mut b = store.read("B").unwrap();
    history::save_pair(&mut b.doc, "k0", "Независимо", "Не потерять").unwrap();
    create(&mut b.doc, "new-b", "c0").unwrap();
    store.write("B", &b).unwrap();
    let peer = b.fingerprint().unwrap();
    restore_initial(&mut store);
    drop(store);
    let mut store = Store::open(&dir.path().join("lifecycle.sqlite3")).unwrap();
    assert!(paused(&store).unwrap());
    assert!(sync(&mut store).is_err());
    assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), peer);
    resume(&mut store).unwrap();
    sync(&mut store).unwrap();
    let nodes = view(&store.read("A").unwrap().doc).unwrap();
    assert_eq!(nodes["k0"].fields["title"].len(), 1);
    assert!(!nodes["k0"].blocked());
    assert!(
        nodes["k0"].fields["title"]
            .iter()
            .any(|v| v.value == "Независимо")
    );
    assert_eq!(location(&nodes, "new-b"), "board");
    assert_eq!(
        history::versions(&store.read("A").unwrap().doc)
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn corrupt_future_foreign_and_stale_snapshots_fail_without_writes() {
    let (_dir, mut store) = prepared();
    let observed = store.read("A").unwrap().fingerprint().unwrap();
    let good = read_snapshot(&store, "initial").unwrap();
    let mut bad = good.clone();
    bad.document.truncate(bad.document.len() / 2);
    let mut future = good.clone();
    future.format = 2;
    let mut wrong_heads = good.clone();
    wrong_heads.heads.clear();
    let foreign = Snapshot::capture(&fixture(false).unwrap().doc).unwrap();
    for invalid in [bad, future, wrong_heads, foreign] {
        store
            .0
            .execute(
                "INSERT OR REPLACE INTO snapshots VALUES ('invalid', ?1)",
                [invalid.encode().unwrap()],
            )
            .unwrap();
        assert!(restore(&mut store, "invalid", &observed).is_err());
        assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), observed);
        assert!(!paused(&store).unwrap());
        assert!(read_snapshot(&store, "before-restore").is_err());
    }
    let mut a = store.read("A").unwrap();
    history::save_pair(&mut a.doc, "k0", "Новая", "Новая").unwrap();
    store.write("A", &a).unwrap();
    assert!(restore(&mut store, "initial", &observed).is_err());
    assert_eq!(
        store.read("A").unwrap().fingerprint().unwrap(),
        a.fingerprint().unwrap()
    );
}

#[test]
fn snapshot_envelope_rejects_truncation_trailing_bytes_and_overflowing_headers() {
    let snapshot = Snapshot::capture(&fixture(false).unwrap().doc).unwrap();
    let bytes = snapshot.encode().unwrap();
    for length in [0, 3, 4, bytes.len() - 1] {
        assert!(Snapshot::decode(&bytes[..length]).is_err());
    }
    let mut trailing = bytes.clone();
    trailing.push(0);
    assert!(Snapshot::decode(&trailing).is_err());
    let mut header = bytes;
    header[..4].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(Snapshot::decode(&header).is_err());
}

#[test]
fn deleting_one_snapshot_preserves_the_others_and_the_live_replicas() {
    let (_dir, mut store) = prepared();
    let a = store.read("A").unwrap();
    save_snapshot(&mut store, "second", &a.doc).unwrap();
    delete_snapshot(&mut store, "initial").unwrap();
    assert!(read_snapshot(&store, "initial").is_err());
    assert!(delete_snapshot(&mut store, "initial").is_err());
    assert_eq!(
        semantic(&read_snapshot(&store, "second").unwrap().load().unwrap()),
        semantic(&a.doc)
    );
    assert_eq!(
        store.read("A").unwrap().fingerprint().unwrap(),
        a.fingerprint().unwrap()
    );
}

#[test]
fn restoration_changes_only_different_fields_so_compatible_offline_edits_merge() {
    let (_dir, mut store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    let n = view(&a.doc).unwrap()["k0"].clone();
    edit(&mut a.doc, "k0", &n, "title", "Позже локально", false).unwrap();
    let n = view(&b.doc).unwrap()["k0"].clone();
    edit(&mut b.doc, "k0", &n, "text", "Независимый текст", false).unwrap();
    store.write("A", &a).unwrap();
    store.write("B", &b).unwrap();
    restore_initial(&mut store);
    resume(&mut store).unwrap();
    sync(&mut store).unwrap();
    let nodes = view(&store.read("A").unwrap().doc).unwrap();
    assert!(!nodes["k0"].blocked());
    assert_eq!(nodes["k0"].fields["title"][0].value, "Снимок");
    assert_eq!(nodes["k0"].fields["text"][0].value, "Независимый текст");
}

#[test]
fn snapshot_restoration_preserves_received_changes_with_missing_dependencies() {
    for dependency_already_here in [false, true] {
        let (_dir, store) = prepared();
        let base = store.read("A").unwrap();
        let mut writer = base.doc.fork();
        let heads = writer.get_heads();
        history::save_pair(&mut writer, "k0", "Первая", "Первый текст").unwrap();
        history::save_pair(&mut writer, "k0", "Вторая", "Не потерять из снимка").unwrap();
        let changes = writer.get_changes(&heads);
        let mut partial = base.doc.clone();
        partial.apply_changes([changes[1].clone()]).unwrap();
        assert!(!partial.get_missing_deps(&[]).is_empty());
        let snapshot = Snapshot::capture(&partial).unwrap();
        let mut current = base.doc.clone();
        if dependency_already_here {
            current.apply_changes([changes[0].clone()]).unwrap();
        }
        let mut result = restored(&current, &snapshot).unwrap();
        if !dependency_already_here {
            assert!(!result.get_missing_deps(&[]).is_empty());
            result.apply_changes([changes[0].clone()]).unwrap();
        }
        assert!(result.get_missing_deps(&[]).is_empty());
        assert!(
            view(&result).unwrap()["k0"].fields["title"]
                .iter()
                .any(|v| v.value == "Вторая")
        );
        assert!(
            history::versions(&result)
                .unwrap()
                .iter()
                .any(|v| v.text == "Не потерять из снимка")
        );
    }
}

#[test]
fn failures_at_each_sql_stage_roll_back_graph_safety_and_pause_together() {
    for (name, clause) in [
        ("copy", "BEFORE INSERT ON snapshots"),
        ("document", "AFTER UPDATE ON replicas"),
        ("pause", "AFTER UPDATE ON lifecycle"),
    ] {
        let (_dir, mut store) = prepared();
        restore_initial(&mut store);
        resume(&mut store).unwrap();
        let safety: Vec<u8> = store
            .0
            .query_row(
                "SELECT payload FROM snapshots WHERE name='before-restore'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let a = store.read("A").unwrap().fingerprint().unwrap();
        let b = store.read("B").unwrap().fingerprint().unwrap();
        store
            .0
            .execute_batch(&format!(
                "CREATE TRIGGER fail_{name} {clause} BEGIN SELECT RAISE(ABORT,'injected'); END;"
            ))
            .unwrap();
        assert!(restore(&mut store, "initial", &a).is_err());
        assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), a);
        assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), b);
        assert!(!paused(&store).unwrap());
        assert_eq!(
            store
                .0
                .query_row(
                    "SELECT payload FROM snapshots WHERE name='before-restore'",
                    [],
                    |r| r.get::<_, Vec<u8>>(0)
                )
                .unwrap(),
            safety
        );
    }
}

#[test]
fn restoration_reopens_snapshot_late_creation_conflict_without_reviving_neighbors() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let mut b = store.read("B").unwrap();
    let nodes = view(&a.doc).unwrap();
    set_container(&mut a.doc, "c0", &scope(&nodes, "c0"), "trash").unwrap();
    purge::purge(&mut a.doc, "s0", &["c0"], false).unwrap();
    create(&mut b.doc, "new-b", "c0").unwrap();
    a.doc.merge(&mut b.doc).unwrap();
    assert_eq!(location(&view(&a.doc).unwrap(), "new-b"), "conflicts");
    let snapshot = Snapshot::capture(&a.doc).unwrap();
    let plan = purge::late_preview(&a.doc, "s0").unwrap();
    purge::resolve(&mut a.doc, &plan, true).unwrap();
    let result = restored(&a.doc, &snapshot).unwrap();
    assert_eq!(location(&view(&result).unwrap(), "new-b"), "conflicts");
    assert_eq!(semantic(&result), semantic(&snapshot.load().unwrap()));
}

#[test]
fn competing_restorations_and_reverse_repeated_delivery_keep_both_results() {
    let (_dir, store) = prepared();
    let mut a = store.read("A").unwrap();
    let first = Snapshot::capture(&a.doc).unwrap();
    history::save_pair(&mut a.doc, "k0", "Второй снимок", "Другой").unwrap();
    let second = Snapshot::capture(&a.doc).unwrap();
    history::save_pair(&mut a.doc, "k0", "Нынешнее", "Нынешнее").unwrap();
    let mut b = a.doc.fork();
    a.doc = restored(&a.doc, &first).unwrap();
    b = restored(&b, &second).unwrap();
    a.doc.merge(&mut b).unwrap();
    let variants = &view(&a.doc).unwrap()["k0"].fields["title"];
    assert_eq!(variants.len(), 2);
    let changes: Vec<_> = a.doc.get_changes(&[]).into_iter().rev().collect();
    let mut rebuilt = Automerge::new();
    for change in changes {
        rebuilt.apply_changes([change.clone(), change]).unwrap();
    }
    assert_eq!(view(&rebuilt).unwrap(), view(&a.doc).unwrap());
    assert_eq!(
        history::snapshot(&rebuilt).unwrap(),
        history::snapshot(&a.doc).unwrap()
    );
}
