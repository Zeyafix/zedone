use super::{column_object, invalid, map};
use crate::StorageResult;
use automerge::{
    Automerge, ObjId, ROOT, ReadDoc,
    legacy::{Change, Key, ObjectId, OpId},
};
use std::collections::{HashMap, HashSet};

fn decoded_id(object: &ObjId) -> ObjectId {
    match object {
        ObjId::Root => ObjectId::Root,
        ObjId::Id(counter, actor, _) => ObjectId::Id(OpId::new(*counter, actor)),
    }
}

pub(crate) fn has_independent_change(
    graph: &Automerge,
    object: &ObjId,
    lifecycle: Option<&str>,
    matches: impl Fn(&Change) -> bool,
) -> StorageResult<bool> {
    let boundary = graph
        .get_all(object, "lifecycle")?
        .into_iter()
        .filter(|(value, _)| lifecycle.is_none_or(|state| value.as_str() == Some(state)))
        .map(|(_, operation)| graph.hash_for_opid(&operation).ok_or_else(invalid))
        .collect::<StorageResult<Vec<_>>>()?;
    find_independent_change(graph, boundary, |hash| {
        let change = graph.get_change_by_hash(hash).ok_or_else(invalid)?.decode();
        Ok(matches(&change))
    })
}

fn find_independent_change(
    graph: &Automerge,
    boundary: Vec<automerge::ChangeHash>,
    mut matches: impl FnMut(&automerge::ChangeHash) -> StorageResult<bool>,
) -> StorageResult<bool> {
    if boundary.is_empty() {
        return Ok(false);
    }
    let unseen = graph.get_changes_meta(&boundary);
    if unseen.is_empty() {
        return Ok(false);
    }

    let mut children = HashMap::<_, Vec<_>>::new();
    for change in &unseen {
        for &dependency in &change.deps {
            children.entry(dependency).or_default().push(change.hash);
        }
    }
    let mut pending = boundary;
    let mut descendants = HashSet::new();
    while let Some(hash) = pending.pop() {
        if descendants.insert(hash) {
            pending.extend(children.remove(&hash).into_iter().flatten());
        }
    }

    for change in unseen {
        if descendants.contains(&change.hash) {
            continue;
        }
        if matches(&change.hash)? {
            return Ok(true);
        }
    }
    Ok(false)
}

pub(crate) const PURGE_BOUNDARY: &str = "purge_boundary";

pub(crate) fn activated_across_purge(
    graph: &Automerge,
    descendant: &ObjId,
    ancestor: &ObjId,
) -> StorageResult<bool> {
    let boundary = graph
        .get_all(ancestor, PURGE_BOUNDARY)?
        .into_iter()
        .map(|(value, op)| {
            if value.as_bool() != Some(true) {
                return Err(invalid());
            }
            graph.hash_for_opid(&op).ok_or_else(invalid)
        })
        .collect::<StorageResult<Vec<_>>>()?;
    if boundary.is_empty() {
        return Ok(false);
    }
    let activations = graph
        .get_all(descendant, "lifecycle")?
        .into_iter()
        .filter(|(value, _)| value.as_str() == Some("active"))
        .map(|(_, op)| graph.hash_for_opid(&op).ok_or_else(invalid))
        .collect::<StorageResult<HashSet<_>>>()?;
    find_independent_change(graph, boundary, |hash| Ok(activations.contains(hash)))
}

pub(super) fn has_independent_edit(
    graph: &Automerge,
    card: &ObjId,
    lifecycle: Option<&str>,
) -> StorageResult<bool> {
    let assignments = decoded_id(&map(graph, card, "tags")?);
    let id = decoded_id(card);
    has_independent_change(graph, card, lifecycle, |change| {
        card_changed(change, &id, &assignments)
    })
}

pub(crate) fn has_independent_column_change(
    graph: &Automerge,
    column: &ObjId,
    cards: &[zedone_core::cards::CardSnapshot],
) -> StorageResult<bool> {
    let column_id = decoded_id(column);
    let members = cards
        .iter()
        .map(|card| {
            let object = super::object(graph, &card.card)?;
            Ok((
                decoded_id(&object),
                decoded_id(&map(graph, &object, "tags")?),
            ))
        })
        .collect::<StorageResult<Vec<_>>>()?;
    has_independent_change(graph, column, None, |change| {
        change.operations.iter().any(|operation| {
            operation.obj == column_id
                && matches!(&operation.key, Key::Map(field) if matches!(field.as_str(), "name" | "position"))
        }) || members.iter().any(|(id, tags)| card_changed(change, id, tags)
            || change.operations.iter().any(|op| &op.obj == id
                && matches!(&op.key, Key::Map(field) if field.as_str() == "lifecycle")
                && matches!(op.primitive_value(), Some(automerge::ScalarValue::Str(value)) if value.as_str() == "active")))
    })
}

fn card_changed(change: &Change, card: &ObjectId, assignments: &ObjectId) -> bool {
    let tag_deletion = change.operations.iter().any(|operation| {
        matches!(&operation.key, Key::Map(field) if field.as_str() == "deleted")
            && operation.primitive_value() == Some(automerge::ScalarValue::Boolean(true))
    });
    change.operations.iter().any(|operation| {
        &operation.obj == assignments && !tag_deletion
            || &operation.obj == card
                && matches!(&operation.key, Key::Map(field) if matches!(
                    field.as_str(),
                    "title" | "description" | "completed" | "placement" | "column" | "position"
                ))
    })
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
