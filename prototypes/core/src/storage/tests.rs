use super::*;

fn database() -> (tempfile::TempDir, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let db = connect(directory.path().join("synthetic.sqlite3")).unwrap();
    (directory, db)
}

#[test]
fn exact_profile_round_trips_after_reopen_and_seed_is_idempotent() {
    let (directory, mut db) = database();
    let expected = synthetic(Space::A);
    assert_eq!(expected.cards.len(), 30);
    assert_eq!(expected.cards[0].title, "Карточка 0001");
    for (index, card) in expected.cards.iter().enumerate() {
        assert_eq!(card.column, index as u32 / 10);
        let lines = if index < 24 {
            5
        } else if index < 29 {
            100
        } else {
            200
        };
        assert_eq!(card.text.lines().count(), lines);
        assert!(!card.text.ends_with('\n'));
        assert_eq!(
            card.text.lines().next().unwrap(),
            "Заметка 0001: ёж, ЕЖ, cafe\u{301}, 😀."
        );
    }
    for _ in 0..2 {
        assert_eq!(
            run(&mut db, StorageRequest::Seed { space: Space::A })
                .unwrap()
                .snapshot,
            Some(expected.clone())
        );
    }
    drop(db);
    let mut reopened = connect(directory.path().join("synthetic.sqlite3")).unwrap();
    assert_eq!(
        run(&mut reopened, StorageRequest::Snapshot { space: Space::A })
            .unwrap()
            .snapshot,
        Some(expected)
    );
}

#[test]
fn search_is_literal_cyrillic_case_insensitive_and_does_not_join_fields() {
    let (_directory, mut db) = database();
    let mut snapshot = synthetic(Space::A);
    snapshot.cards.truncate(3);
    snapshot.cards[0].title = "ЁЛКА C++ план* %_? \\".into();
    snapshot.cards[0].text = "Купить  молоко\tсегодня\nЖУК cafe\u{301} 😀".into();
    snapshot.cards[1].title = "ЕЛКА купить".into();
    snapshot.cards[1].text = "молоко".into();
    snapshot.cards[2].title = "планирование".into();
    snapshot.cards[2].text.clear();
    run(&mut db, StorageRequest::Restore { snapshot }).unwrap();
    for (query, ids) in [
        ("  ёлка  ", vec![1]),
        ("елка", vec![2]),
        ("жук", vec![1]),
        ("купить молоко", vec![]),
        ("купить  молоко", vec![1]),
        ("молоко\tсегодня", vec![1]),
        ("сегодня\nжук", vec![1]),
        ("c++", vec![1]),
        ("план*", vec![1]),
        ("%_?", vec![1]),
        ("\\", vec![1]),
        ("cafe\u{301}", vec![1]),
        ("😀", vec![1]),
        ("' OR 1=1 --", vec![]),
        (" \t\n ", vec![1, 2, 3]),
    ] {
        let found = run(
            &mut db,
            StorageRequest::Read {
                space: Space::A,
                query: query.into(),
            },
        )
        .unwrap()
        .snapshot
        .unwrap();
        assert_eq!(
            found.cards.iter().map(|card| card.id).collect::<Vec<_>>(),
            ids,
            "{query:?}"
        );
    }
}

#[test]
fn snapshot_restore_and_delete_preserve_other_space_and_rollback_bad_write() {
    let (_directory, mut db) = database();
    let a = run(&mut db, StorageRequest::Seed { space: Space::A })
        .unwrap()
        .snapshot
        .unwrap();
    let mut b = synthetic(Space::B);
    b.cards[0].text = "B_ONLY_SENTINEL".into();
    run(
        &mut db,
        StorageRequest::Restore {
            snapshot: b.clone(),
        },
    )
    .unwrap();
    let serialized = serde_json::to_string(&a).unwrap();
    assert!(!serialized.contains("B_ONLY_SENTINEL"));
    assert!(!serialized.contains("\"space\":\"B\""));
    let mut invalid = a.clone();
    invalid.cards[1].column = 99; // Fails after destructive statements and first insert.
    assert!(run(&mut db, StorageRequest::Restore { snapshot: invalid }).is_err());
    assert_eq!(load(&db, Space::A).unwrap(), Some(a.clone()));
    for _ in 0..2 {
        run(&mut db, StorageRequest::Delete { space: Space::A }).unwrap();
    }
    assert_eq!(load(&db, Space::A).unwrap(), None);
    run(
        &mut db,
        StorageRequest::Restore {
            snapshot: serde_json::from_str(&serialized).unwrap(),
        },
    )
    .unwrap();
    assert_eq!(load(&db, Space::A).unwrap(), Some(a));
    assert_eq!(load(&db, Space::B).unwrap(), Some(b));
}

#[test]
fn logical_snapshot_keeps_one_revision_across_concurrent_commit() {
    let (directory, mut db) = database();
    db.pragma_update(None, "journal_mode", "WAL").unwrap();
    let before = synthetic(Space::A);
    run(
        &mut db,
        StorageRequest::Restore {
            snapshot: before.clone(),
        },
    )
    .unwrap();
    let mut writer = connect(directory.path().join("synthetic.sqlite3")).unwrap();
    let tx = db.transaction().unwrap();
    // Pin the read version before another connection commits all related tables.
    tx.query_row("SELECT id FROM prototype_spaces WHERE id = 'A'", [], |_| {
        Ok(())
    })
    .unwrap();
    let mut after = before.clone();
    after.columns[0] = "New column".into();
    after.cards[0].title = "New card".into();
    run(
        &mut writer,
        StorageRequest::Restore {
            snapshot: after.clone(),
        },
    )
    .unwrap();
    assert_eq!(load(&tx, Space::A).unwrap(), Some(before));
    tx.commit().unwrap();
    assert_eq!(load(&db, Space::A).unwrap(), Some(after));
}

#[tokio::test(flavor = "current_thread")]
async fn locked_database_does_not_block_async_runtime_and_failed_write_is_unchanged() {
    let (directory, mut db) = database();
    let expected = run(&mut db, StorageRequest::Seed { space: Space::A })
        .unwrap()
        .snapshot;
    db.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let storage = Storage::new(directory.path().into());
    let task = tokio::spawn(async move {
        storage
            .execute(StorageRequest::Delete { space: Space::A })
            .await
    });
    let started = std::time::Instant::now();
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert!(
        started.elapsed() < Duration::from_secs(1),
        "SQLite blocked the single async thread"
    );
    assert!(
        !task.is_finished(),
        "The storage call must be waiting for the real database lock"
    );
    assert!(task.await.unwrap().is_err()); // Real SQLITE_BUSY, no injected success.
    db.execute_batch("ROLLBACK").unwrap();
    assert_eq!(load(&db, Space::A).unwrap(), expected);
}

#[test]
fn contract_rejects_unknown_actions_spaces_and_missing_query() {
    for input in [
        r#"{"action":"seed","space":"C"}"#,
        r#"{"action":"read","space":"A"}"#,
        r#"{"action":"sql","space":"A"}"#,
        r#"{"action":"seed","space":"A","path":"other.sqlite3"}"#,
    ] {
        assert!(serde_json::from_str::<StorageRequest>(input).is_err());
    }
}
