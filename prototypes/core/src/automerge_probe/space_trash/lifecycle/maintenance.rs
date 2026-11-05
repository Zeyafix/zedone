//! Lossless graph packing and SQLite free-page reclamation, not history pruning.
use super::*;

fn pack(store: &mut Store) -> Result<Value> {
    let tx = store
        .0
        .transaction_with_behavior(TransactionBehavior::Immediate)?;
    let rows = {
        let mut statement =
            tx.prepare("SELECT id,document,actor,peer FROM replicas ORDER BY id")?;
        statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Vec<u8>>(1)?,
                    r.get::<_, Vec<u8>>(2)?,
                    r.get::<_, Vec<u8>>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut before_bytes = 0;
    let mut after_bytes = 0;
    for (id, bytes, actor, peer) in rows {
        let mut doc = Automerge::load(&bytes)?;
        doc.set_actor(actor.clone().into());
        State::decode(&peer)?;
        // Default SaveOptions retains queued changes with missing dependencies.
        let compact = doc.save_and_verify()?;
        let mut checked = Automerge::load(&compact)?;
        checked.set_actor(actor.into());
        if doc.get_heads() != checked.get_heads()
            || doc
                .get_changes(&[])
                .iter()
                .map(|c| c.hash())
                .collect::<Vec<_>>()
                != checked
                    .get_changes(&[])
                    .iter()
                    .map(|c| c.hash())
                    .collect::<Vec<_>>()
            || doc.get_missing_deps(&[]) != checked.get_missing_deps(&[])
            || compact != checked.save()
        {
            return Err("Packing changed graph identity or pending delivery".into());
        }
        before_bytes += bytes.len();
        after_bytes += compact.len();
        // Do not rewrite actor, peer, snapshots, pause, settings or other spaces.
        tx.execute(
            "UPDATE replicas SET document=?1 WHERE id=?2",
            params![compact, id],
        )?;
    }
    tx.commit()?;
    Ok(json!({"before_blob_bytes": before_bytes, "after_blob_bytes": after_bytes}))
}

fn reclaim(store: &Store) -> Result<()> {
    // VACUUM is transactional inside SQLite and cannot be nested in pack's tx.
    // If it fails, the packed graph remains valid; report failure and allow retry.
    store.0.execute_batch("VACUUM;")?;
    let integrity: String = store
        .0
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err("SQLite integrity check failed after reclamation".into());
    }
    Ok(())
}

pub(super) fn compact(store: &mut Store) -> Result<Value> {
    let report = pack(store)?;
    reclaim(store)?;
    Ok(report)
}

// Construct a real persisted base + incremental suffix, not redundant JSON.
fn incremental_edits(store: &mut Store, count: usize) -> Result<()> {
    let mut a = store.read("A")?;
    let mut stream = a.doc.save();
    for i in 0..count {
        let heads = a.doc.get_heads();
        history::save_pair(
            &mut a.doc,
            "k0",
            &format!("Версия {i}"),
            &format!("Текст {i} 😀"),
        )?;
        stream.extend(a.doc.save_after(&heads));
    }
    store
        .0
        .execute("UPDATE replicas SET document=?1 WHERE id='A'", [stream])?;
    Ok(())
}

pub(super) fn run(directory: &Path, phase: &str) -> Result<()> {
    let mut store = Store::open(&directory.join("automerge-compaction.sqlite3"))?;
    let expected = directory.join("compaction-expected.json");
    match phase {
        "lifecycle-compact-prepare" => {
            seed(&mut store)?;
            incremental_edits(&mut store, 24)?;
            let mut a = store.read("A")?;
            let nodes = view(&a.doc)?;
            set_container(&mut a.doc, "c0", &scope(&nodes, "c0"), "trash")?;
            purge::purge(&mut a.doc, "s0", &["c0"], false)?;
            store.write("A", &a)?;
            let mut b = store.read("B")?;
            history::save_pair(&mut b.doc, "k0", "Поздняя", "Не потерять после сокращения")?;
            create(&mut b.doc, "new-b", "c0")?;
            store.write("B", &b)?;
            std::fs::write(&expected, serde_json::to_vec(&a.fingerprint()?)?)?;
        }
        "lifecycle-compact-run" => {
            let report = compact(&mut store)?;
            assert_eq!(
                store.read("A")?.fingerprint()?,
                serde_json::from_slice::<Value>(&std::fs::read(&expected)?)?
            );
            println!("{report}");
        }
        "lifecycle-compact-resume" => {
            assert_eq!(
                store.read("A")?.fingerprint()?,
                serde_json::from_slice::<Value>(&std::fs::read(&expected)?)?
            );
            sync(&mut store)?;
            let a = store.read("A")?;
            let nodes = view(&a.doc)?;
            assert_eq!(location(&nodes, "k0"), "conflicts");
            assert_eq!(location(&nodes, "new-b"), "conflicts");
            assert_eq!(location(&nodes, "k1"), "purged");
            assert!(
                nodes["k0"].fields["title"]
                    .iter()
                    .any(|v| v.value == "Поздняя")
            );
            read_snapshot(&store, "initial")?;
            compact(&mut store)?;
        }
        _ => return Err("Unknown compaction phase".into()),
    }
    println!("P02.06 {phase}: passed");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prepared() -> (tempfile::TempDir, Store) {
        let dir = tempfile::tempdir().unwrap();
        let mut store = Store::open(&dir.path().join("pack.sqlite3")).unwrap();
        seed(&mut store).unwrap();
        (dir, store)
    }

    #[test]
    fn incremental_stream_shrinks_without_losing_graph_snapshots_or_local_state() {
        let (dir, mut store) = prepared();
        incremental_edits(&mut store, 48).unwrap();
        let a = store.read("A").unwrap().fingerprint().unwrap();
        let b = store.read("B").unwrap().fingerprint().unwrap();
        let snapshot = read_snapshot(&store, "initial").unwrap().document;
        let report = compact(&mut store).unwrap();
        assert!(report["after_blob_bytes"].as_u64() < report["before_blob_bytes"].as_u64());
        assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), a);
        assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), b);
        assert_eq!(read_snapshot(&store, "initial").unwrap().document, snapshot);
        assert!(!paused(&store).unwrap());
        let second = compact(&mut store).unwrap();
        assert_eq!(second["before_blob_bytes"], second["after_blob_bytes"]);
        drop(store);
        let store = Store::open(&dir.path().join("pack.sqlite3")).unwrap();
        assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), a);
    }

    #[test]
    fn failures_and_read_only_vacuum_leave_valid_retryable_state() {
        let (dir, mut store) = prepared();
        incremental_edits(&mut store, 8).unwrap();
        let before: Vec<Vec<u8>> = store
            .0
            .prepare("SELECT document FROM replicas ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        store
            .0
            .execute_batch(
                "CREATE TRIGGER fail_pack AFTER UPDATE ON replicas
            WHEN new.id='B' BEGIN SELECT RAISE(ABORT,'injected'); END;",
            )
            .unwrap();
        assert!(pack(&mut store).is_err());
        let after: Vec<Vec<u8>> = store
            .0
            .prepare("SELECT document FROM replicas ORDER BY id")
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(after, before);
        store.0.execute_batch("DROP TRIGGER fail_pack").unwrap();
        compact(&mut store).unwrap();
        let read_only = Store(
            Connection::open_with_flags(
                dir.path().join("pack.sqlite3"),
                rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap(),
        );
        assert!(reclaim(&read_only).is_err());
        assert_eq!(
            store.read("A").unwrap().fingerprint().unwrap(),
            read_only.read("A").unwrap().fingerprint().unwrap()
        );
        drop(read_only);
        compact(&mut store).unwrap();
    }

    #[test]
    fn pending_out_of_order_changes_are_retained_until_dependencies_arrive() {
        let (_dir, mut store) = prepared();
        let mut a = store.read("A").unwrap();
        let base = a.doc.get_heads();
        history::save_pair(&mut a.doc, "k0", "Первый", "Первый").unwrap();
        history::save_pair(&mut a.doc, "k0", "Второй", "Второй").unwrap();
        let changes = a.doc.get_changes(&base);
        let mut b = store.read("B").unwrap();
        b.doc.apply_changes([changes[1].clone()]).unwrap();
        assert!(!b.doc.get_missing_deps(&[]).is_empty());
        store.write("B", &b).unwrap();
        compact(&mut store).unwrap();
        let mut loaded = store.read("B").unwrap();
        assert_eq!(
            loaded.doc.get_missing_deps(&[]),
            b.doc.get_missing_deps(&[])
        );
        loaded.doc.apply_changes([changes[0].clone()]).unwrap();
        assert_eq!(view(&loaded.doc).unwrap(), view(&a.doc).unwrap());
        assert_eq!(
            history::snapshot(&loaded.doc).unwrap(),
            history::snapshot(&a.doc).unwrap()
        );
    }

    #[test]
    fn sqlite_free_pages_are_reclaimed_while_snapshots_and_pause_survive() {
        let (dir, mut store) = prepared();
        store.0.execute_batch("CREATE TABLE scratch(blob BLOB); INSERT INTO scratch VALUES (zeroblob(1048576)); DROP TABLE scratch;").unwrap();
        let observed = store.read("A").unwrap().fingerprint().unwrap();
        restore(&mut store, "initial", &observed).unwrap();
        let size = std::fs::metadata(dir.path().join("pack.sqlite3"))
            .unwrap()
            .len();
        compact(&mut store).unwrap();
        assert!(
            std::fs::metadata(dir.path().join("pack.sqlite3"))
                .unwrap()
                .len()
                < size
        );
        assert!(paused(&store).unwrap());
        read_snapshot(&store, "before-restore").unwrap();
    }
}
