use super::{column_object, invalid, map};
use crate::StorageResult;
use automerge::{
    Automerge, ObjId, ROOT, ReadDoc,
    legacy::{Key, ObjectId, OpId},
};
use std::collections::{HashMap, HashSet};

fn decoded_id(object: &ObjId) -> ObjectId {
    match object {
        ObjId::Root => ObjectId::Root,
        ObjId::Id(counter, actor, _) => ObjectId::Id(OpId::new(*counter, actor)),
    }
}

pub(super) fn has_independent_edit(graph: &Automerge, card: &ObjId) -> StorageResult<bool> {
    let deletions = graph
        .get_all(card, "lifecycle")?
        .into_iter()
        .map(|(_, operation)| graph.hash_for_opid(&operation).ok_or_else(invalid))
        .collect::<StorageResult<Vec<_>>>()?;
    let unseen = graph.get_changes_meta(&deletions);
    if unseen.is_empty() {
        return Ok(false);
    }

    let mut children = HashMap::<_, Vec<_>>::new();
    for change in &unseen {
        for &dependency in &change.deps {
            children.entry(dependency).or_default().push(change.hash);
        }
    }
    let mut pending = deletions;
    let mut descendants = HashSet::new();
    while let Some(hash) = pending.pop() {
        if descendants.insert(hash) {
            pending.extend(children.remove(&hash).into_iter().flatten());
        }
    }

    let assignments = decoded_id(&map(graph, card, "tags")?);
    let card = decoded_id(card);
    for change in unseen {
        if descendants.contains(&change.hash) {
            continue;
        }
        let change = graph
            .get_change_by_hash(&change.hash)
            .ok_or_else(invalid)?
            .decode();
        let tag_deletion = change.operations.iter().any(|operation| {
            matches!(&operation.key, Key::Map(field) if field.as_str() == "deleted")
                && operation.primitive_value() == Some(automerge::ScalarValue::Boolean(true))
        });
        if change.operations.iter().any(|operation| {
            operation.obj == assignments && !tag_deletion
                || operation.obj == card
                    && matches!(&operation.key, Key::Map(field) if matches!(
                        field.as_str(),
                        "title" | "description" | "completed" | "placement" | "column" | "position"
                    ))
        }) {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(super) fn has_container_group(graph: &Automerge, lifecycle: &[String]) -> StorageResult<bool> {
    let matches_group = |object: &ObjId| -> StorageResult<bool> {
        Ok(graph
            .get_all(object, "lifecycle")?
            .iter()
            .any(|(value, _)| {
                value.as_str().is_some_and(|value| {
                    value != "active" && lifecycle.iter().any(|state| state == value)
                })
            }))
    };
    if matches_group(&ROOT)? {
        return Ok(true);
    }
    for root in graph.keys(ROOT) {
        if let Some(column) = root.strip_prefix("column:")
            && matches_group(&column_object(graph, column)?)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}
