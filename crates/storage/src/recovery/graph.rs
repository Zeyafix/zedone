use crate::{
    StorageError, StorageResult,
    snapshot::{SpaceSnapshot, import::model},
};
use automerge::{
    Automerge, Change, ObjId, ObjType, ROOT, ReadDoc, ScalarValue, Value, transaction::Transactable,
};
use std::collections::BTreeSet;
use zedone_core::error::{CommandError, ErrorCode};

fn invalid() -> StorageError {
    CommandError {
        code: ErrorCode::InvalidRequest,
    }
    .into()
}
fn commit(
    graph: &mut Automerge,
    work: impl FnOnce(&mut automerge::transaction::Transaction<'_>) -> StorageResult<()>,
) -> StorageResult<()> {
    let mut tx = graph.transaction();
    work(&mut tx)?;
    tx.commit();
    Ok(())
}
fn scalars(graph: &Automerge, object: &ObjId, field: &str) -> StorageResult<Vec<ScalarValue>> {
    let mut result = graph
        .get_all(object, field)?
        .into_iter()
        .map(|(v, _)| match v {
            Value::Scalar(v) => Ok(v.into_owned()),
            _ => Err(invalid()),
        })
        .collect::<StorageResult<Vec<_>>>()?;
    result.sort_by_key(|v| format!("{v:?}"));
    result.dedup();
    Ok(result)
}
fn assign(
    graph: &mut Automerge,
    object: &ObjId,
    field: &str,
    desired: Vec<ScalarValue>,
) -> StorageResult<()> {
    if scalars(graph, object, field)? == desired {
        return Ok(());
    }
    if desired.is_empty() {
        return commit(graph, |tx| {
            tx.delete(object, field)?;
            Ok(())
        });
    }
    let base = graph.clone();
    for (index, value) in desired.into_iter().enumerate() {
        if index == 0 {
            commit(graph, |tx| {
                tx.delete(object, field)?;
                tx.put(object, field, value)?;
                Ok(())
            })?;
        } else {
            let mut branch = base.fork();
            commit(&mut branch, |tx| {
                tx.delete(object, field)?;
                tx.put(object, field, value)?;
                Ok(())
            })?;
            graph.merge(&mut branch)?;
        }
    }
    Ok(())
}
fn map_child(graph: &mut Automerge, parent: &ObjId, field: &str) -> StorageResult<ObjId> {
    let existing = graph.get_all(parent, field)?;
    if existing.len() == 1 && existing[0].0 == Value::Object(ObjType::Map) {
        return Ok(existing[0].1.clone());
    }
    let mut tx = graph.transaction();
    let result = tx.put_object(parent, field, ObjType::Map)?;
    tx.commit();
    Ok(result)
}
fn copy_map(
    graph: &mut Automerge,
    source: &Automerge,
    target: &ObjId,
    original: &ObjId,
    depth: usize,
) -> StorageResult<()> {
    if depth > 4 {
        return Err(invalid());
    }
    let keys: BTreeSet<_> = source.keys(original).chain(graph.keys(target)).collect();
    for key in keys {
        let values = source.get_all(original, &key)?;
        if values.is_empty() {
            if *target == ROOT && key.starts_with("card:")
                || *target == ROOT && key.starts_with("column:")
            {
                let child = map_child(graph, target, &key)?;
                assign(graph, &child, "lifecycle", vec!["purged".into()])?;
                if key.starts_with("column:") {
                    assign(graph, &child, "purge_boundary", vec![true.into()])?;
                }
            } else if *target == ROOT && key.starts_with("tag:") {
                let child = map_child(graph, target, &key)?;
                assign(graph, &child, "deleted", vec![true.into()])?;
            } else {
                commit(graph, |tx| {
                    tx.delete(target, &key)?;
                    Ok(())
                })?;
            }
        } else if values[0].0 == Value::Object(ObjType::Map) {
            if values.len() != 1 {
                return Err(invalid());
            }
            let child = map_child(graph, target, &key)?;
            if key == "history" && depth == 1 {
                let versions = crate::cards::history(source, original)?;
                let current: Vec<_> = graph.keys(&child).collect();
                commit(graph, |tx| {
                    for key in &current {
                        tx.delete(&child, key)?;
                    }
                    Ok(())
                })?;
                for version in versions {
                    let mut bytes = [0; 16];
                    getrandom::fill(&mut bytes).map_err(|_| StorageError::Unavailable)?;
                    let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                    let entry = format!("{}:{id}", version.subject);
                    commit(graph, |tx| {
                        let object = tx.put_object(&child, &entry, ObjType::Map)?;
                        tx.put(&object, "sequence", version.sequence)?;
                        tx.put(&object, "title", version.title.as_str())?;
                        tx.put(&object, "description", version.description.as_str())?;
                        Ok(())
                    })?;
                }
            } else {
                copy_map(graph, source, &child, &values[0].1, depth + 1)?;
            }
        } else {
            assign(graph, target, &key, scalars(source, original, &key)?)?;
        }
    }
    Ok(())
}
fn marker(graph: &mut Automerge, object: &ObjId, field: &str, value: bool) -> StorageResult<()> {
    if !value && graph.get_all(object, field)?.is_empty() {
        return Ok(());
    }
    commit(graph, |tx| {
        tx.delete(object, field)?;
        tx.put(object, field, value)?;
        Ok(())
    })
}
fn preserve_conflicts(graph: &mut Automerge, source: &Automerge, space: &str) -> StorageResult<()> {
    use crate::recovery_flags::{CONFLICT, RETIRED_TAG};
    use zedone_core::spaces::SpaceState;
    let desired = crate::spaces::snapshot(source, space, true)?;
    marker(
        graph,
        &ROOT,
        CONFLICT,
        desired.state == SpaceState::Conflicted && !desired.deletion_groups.is_empty(),
    )?;
    let contents = desired.contents.ok_or_else(invalid)?;
    for column in contents.columns {
        let object = map_child(graph, &ROOT, &format!("column:{}", column.column))?;
        if column.deletion_groups.contains(&"purged".into())
            && column.state == SpaceState::Conflicted
        {
            assign(
                graph,
                &object,
                "lifecycle",
                vec!["active".into(), "purged".into()],
            )?;
        }
        marker(
            graph,
            &object,
            CONFLICT,
            column.state == SpaceState::Conflicted && !column.deletion_groups.is_empty(),
        )?;
    }
    for card in &contents.cards {
        let object = crate::cards::object(graph, &card.card)?;
        if card.deletion_groups.contains(&"purged".into()) && card.state == SpaceState::Conflicted {
            assign(
                graph,
                &object,
                "lifecycle",
                vec!["active".into(), "purged".into()],
            )?;
        }
        marker(graph, &object, CONFLICT, card.deletion_conflict)?;
    }
    let tags = crate::tags::snapshots(source, space, &crate::spaces::domain_projections(source)?)?;
    let roots: Vec<_> = graph
        .keys(ROOT)
        .filter(|key| key.starts_with("tag:"))
        .collect();
    for key in roots {
        let id = key.strip_prefix("tag:").ok_or_else(invalid)?;
        let object = map_child(graph, &ROOT, &key)?;
        if let Some(tag) = tags.get(id) {
            let source_object = source.get(ROOT, &key)?.ok_or_else(invalid)?.1;
            let retired =
                crate::recovery_flags::active(source, &source_object, RETIRED_TAG, "deleted")?;
            marker(graph, &object, RETIRED_TAG, retired)?;
            marker(graph, &object, CONFLICT, tag.deletion_conflict)?;
        } else {
            marker(graph, &object, CONFLICT, false)?;
            marker(graph, &object, RETIRED_TAG, true)?;
        }
    }
    for card in &contents.cards {
        let source_object = crate::cards::object(source, &card.card)?;
        let desired = crate::tags::convergence::merged_assignments(source, &source_object, &tags)?;
        let object = crate::cards::object(graph, &card.card)?;
        let current_tags =
            crate::tags::snapshots(graph, space, &crate::spaces::domain_projections(graph)?)?;
        let actual = crate::tags::convergence::merged_assignments(graph, &object, &current_tags)?;
        let assignments = crate::cards::map(graph, &object, "tags")?;
        for (tag, values) in desired {
            if actual.get(&tag) != Some(&values) {
                commit(graph, |tx| {
                    tx.delete(&assignments, &tag)?;
                    Ok(())
                })?;
                assign(
                    graph,
                    &assignments,
                    &tag,
                    values.into_iter().map(ScalarValue::Boolean).collect(),
                )?;
            }
        }
    }
    Ok(())
}
pub(super) fn plan(current: &Automerge, snapshot: &SpaceSnapshot) -> StorageResult<Vec<Change>> {
    let mut source = Automerge::new();
    source
        .apply_changes(snapshot.changes().iter().cloned())
        .map_err(|_| invalid())?;
    model::validate(&source, &snapshot.metadata().space)?;
    let mut schema = model::Schema::default();
    for change in source.get_changes(&[]) {
        let mut decoded = change.decode();
        if !decoded.extra_bytes.is_empty() || decoded.author.is_some() {
            return Err(invalid());
        }
        schema.visit(&mut decoded, &mut |id| Ok(id.into()))?;
    }
    schema.check_references()?;
    let initial = source.get_changes(&[]).first().ok_or_else(invalid)?.hash();
    if current.get_change_by_hash(&initial).is_none() {
        return Err(invalid());
    }
    let before = current.get_heads();
    let mut result = current.clone();
    result
        .apply_changes(snapshot.changes().iter().cloned())
        .map_err(|_| invalid())?;
    copy_map(&mut result, &source, &ROOT, &ROOT, 0)?;
    preserve_conflicts(&mut result, &source, &snapshot.metadata().space)?;
    commit(&mut result, |tx| {
        tx.delete(ROOT, "format")?;
        tx.put(ROOT, "format", "zedone.space.v1")?;
        Ok(())
    })?;
    model::validate(&result, &snapshot.metadata().space)?;
    let changes = result.get_changes(&before);
    if changes.len() > zedone_core::snapshots::MAX_ARCHIVE_CHANGES
        || changes.iter().map(|c| c.raw_bytes().len()).sum::<usize>()
            > zedone_core::snapshots::MAX_ARCHIVE_BYTES
    {
        return Err(StorageError::Unavailable);
    }
    let mut all = snapshot.changes().to_vec();
    all.extend(changes);
    Ok(all)
}
