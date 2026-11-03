use super::*;

fn prepared() -> (tempfile::TempDir, Store) {
    let directory = tempfile::tempdir().unwrap();
    run(directory.path(), "containers-prepare").unwrap();
    let store = Store::open(&directory.path().join("automerge-containers.sqlite3")).unwrap();
    (directory, store)
}

fn identities(doc: &Automerge) -> Vec<(String, Option<String>)> {
    NODES
        .iter()
        .map(|node| {
            let view = view(doc, *node).unwrap();
            (view.id, view.parent)
        })
        .collect()
}

#[test]
fn names_and_competing_choices_survive_process_phases_without_replacing_objects() {
    let (directory, store) = prepared();
    let original = identities(&store.read("A").unwrap().doc);
    drop(store);
    for phase in ["containers-resolve", "containers-resume"] {
        run(directory.path(), phase).unwrap();
    }
    let store = Store::open(&directory.path().join("automerge-containers.sqlite3")).unwrap();
    for id in ["A", "B"] {
        let replica = store.read(id).unwrap();
        assert_eq!(identities(&replica.doc), original);
        assert!(!view(&replica.doc, Node::Card).unwrap().blocked());
    }
    assert!(run(directory.path(), "containers-prepare").is_err());
}

#[test]
fn parent_name_conflicts_allow_child_edits_but_do_not_override_child_conflicts() {
    let (_directory, mut store) = prepared();
    for (id, text) in [("A", "Текст A"), ("B", "Текст B")] {
        let mut replica = store.read(id).unwrap();
        for node in [Node::Space, Node::Left] {
            assert!(change(&mut replica.doc, node, "name", "Blocked", None).is_err());
        }
        change(
            &mut replica.doc,
            Node::Right,
            "name",
            "Свободная колонка",
            None,
        )
        .unwrap();
        change(&mut replica.doc, Node::Card, "text", text, None).unwrap();
        store.write(id, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    let before = a.fingerprint().unwrap();
    assert!(change(&mut a.doc, Node::Card, "name", "Blocked", None).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    let observed = view(&a.doc, Node::Card).unwrap();
    change(&mut a.doc, Node::Card, "text", "Текст A", Some(&observed)).unwrap();
    change(&mut a.doc, Node::Card, "name", "Доступная карточка", None).unwrap();
    assert!(view(&a.doc, Node::Space).unwrap().blocked());
    assert!(view(&a.doc, Node::Left).unwrap().blocked());
}

#[test]
fn equal_names_never_merge_siblings_or_change_parent_ids() {
    let directory = tempfile::tempdir().unwrap();
    let mut store = Store::open(&directory.path().join("probe.sqlite3")).unwrap();
    seed(&mut store).unwrap();
    let original = store.read("A").unwrap();
    let ids = identities(&original.doc);
    assert_ne!(ids[1].0, ids[2].0);
    assert_eq!(ids[1].1, Some(ids[0].0.clone()));
    assert_eq!(ids[2].1, Some(ids[0].0.clone()));
    assert_eq!(ids[3].1, Some(ids[1].0.clone()));
    for id in ["A", "B"] {
        let mut replica = store.read(id).unwrap();
        change(
            &mut replica.doc,
            Node::Left,
            "name",
            "Совпавшая правка",
            None,
        )
        .unwrap();
        store.write(id, &replica).unwrap();
    }
    exchange(&mut store).unwrap();
    let mut a = store.read("A").unwrap();
    assert_eq!(view(&a.doc, Node::Left).unwrap().name.len(), 2);
    assert!(!view(&a.doc, Node::Left).unwrap().blocked());
    change(&mut a.doc, Node::Left, "name", "Одинаковое имя", None).unwrap();
    store.write("A", &a).unwrap();
    exchange(&mut store).unwrap();
    for id in ["A", "B"] {
        let replica = store.read(id).unwrap();
        expect(&replica.doc, Node::Left, &["Одинаковое имя"]).unwrap();
        expect(&replica.doc, Node::Right, &["Одинаковое имя"]).unwrap();
        assert_eq!(identities(&replica.doc), ids);
    }
}

#[test]
fn child_edit_preserves_choice_but_new_parent_assignment_invalidates_it() {
    let (_directory, mut store) = prepared();
    let mut a = store.read("A").unwrap();
    let observed = view(&a.doc, Node::Left).unwrap();
    let original_choice = observed.clone();
    let conflict_heads = a.doc.get_heads();
    let container = Node::Left.object(&a.doc).unwrap();
    let wrong_object = view(&a.doc, Node::Space).unwrap();
    let before = a.fingerprint().unwrap();
    assert!(change(&mut a.doc, Node::Left, "name", "Wrong", Some(&wrong_object)).is_err());
    assert!(change(&mut a.doc, Node::Left, "text", "Wrong", None).is_err());
    assert!(change(&mut a.doc, Node::Right, "name", "Wrong", Some(&observed)).is_err());
    assert_eq!(a.fingerprint().unwrap(), before);
    change(
        &mut a.doc,
        Node::Card,
        "text",
        "Новая правка содержимого",
        None,
    )
    .unwrap();
    change(&mut a.doc, Node::Left, "name", "Решение", Some(&observed)).unwrap();
    assert_eq!(
        a.doc
            .get_all_at(&container, "name", &conflict_heads)
            .unwrap()
            .len(),
        2
    );
    let current = a.fingerprint().unwrap();
    assert!(change(&mut a.doc, Node::Left, "name", "Stale", Some(&observed)).is_err());
    assert_eq!(a.fingerprint().unwrap(), current);
    // A decision not received at the time of choosing must remain a variant.
    let mut b = store.read("B").unwrap();
    let observed = view(&b.doc, Node::Left).unwrap();
    change(
        &mut b.doc,
        Node::Left,
        "name",
        "Позднее решение",
        Some(&observed),
    )
    .unwrap();
    change(&mut b.doc, Node::Left, "name", "Поздняя правка", None).unwrap();
    store.write("A", &a).unwrap();
    store.write("B", &b).unwrap();
    exchange(&mut store).unwrap();
    for id in ["A", "B"] {
        expect(
            &store.read(id).unwrap().doc,
            Node::Left,
            &["Решение", "Поздняя правка"],
        )
        .unwrap();
    }
    let mut a = store.read("A").unwrap();
    let before = a.fingerprint().unwrap();
    assert!(
        change(
            &mut a.doc,
            Node::Left,
            "name",
            "Stale",
            Some(&original_choice)
        )
        .is_err()
    );
    assert_eq!(a.fingerprint().unwrap(), before);
}

#[test]
fn duplicate_reordered_changes_preserve_all_container_variants_ids_and_links() {
    let (directory, mut store) = prepared();
    run(directory.path(), "containers-resolve").unwrap();
    exchange(&mut store).unwrap();
    let original = store.read("A").unwrap();
    let expected = snapshot(&original.doc).unwrap();
    let changes = original.doc.get_changes(&[]);
    for reverse in [false, true] {
        for offset in 0..changes.len() {
            let mut delivery = changes.clone();
            delivery.rotate_left(offset);
            if reverse {
                delivery.reverse();
            }
            let mut target = Replica::new();
            for change in delivery {
                target.doc.apply_changes([change.clone(), change]).unwrap();
            }
            assert!(target.doc.get_missing_deps(&[]).is_empty());
            assert_eq!(target.doc.get_heads(), original.doc.get_heads());
            assert_eq!(target.doc.get_changes(&[]).len(), changes.len());
            assert_eq!(snapshot(&target.doc).unwrap(), expected);
            store.write("A", &target).unwrap();
            assert_eq!(snapshot(&store.read("A").unwrap().doc).unwrap(), expected);
        }
    }
}

#[test]
fn failed_save_retains_hierarchy_and_variants_on_both_replicas_after_reopen() {
    let (directory, mut store) = prepared();
    let before = store.read("A").unwrap().fingerprint().unwrap();
    let other = store.read("B").unwrap().fingerprint().unwrap();
    let mut candidate = store.read("A").unwrap();
    let observed = view(&candidate.doc, Node::Space).unwrap();
    change(
        &mut candidate.doc,
        Node::Space,
        "name",
        "Не сохранится",
        Some(&observed),
    )
    .unwrap();
    change(
        &mut candidate.doc,
        Node::Card,
        "text",
        "Не сохранится",
        None,
    )
    .unwrap();
    store
        .0
        .execute_batch(
            "CREATE TRIGGER reject_container AFTER UPDATE ON replicas
        BEGIN SELECT RAISE(ABORT, 'synthetic container failure'); END;",
        )
        .unwrap();
    assert!(store.write("A", &candidate).is_err());
    drop(store);
    let store = Store::open(&directory.path().join("automerge-containers.sqlite3")).unwrap();
    assert_eq!(store.read("A").unwrap().fingerprint().unwrap(), before);
    assert_eq!(store.read("B").unwrap().fingerprint().unwrap(), other);
}
