use super::*;
use automerge::{ChangeHash, Value};
use std::collections::BTreeSet;

struct Candidate<T> {
    object: ObjId,
    field: String,
    operation: ObjId,
    value: T,
}

fn context(graph: &Automerge, operation: &ObjId) -> StorageResult<Vec<ChangeHash>> {
    let hash = graph.hash_for_opid(operation).ok_or_else(invalid)?;
    Ok(graph
        .get_change_by_hash(&hash)
        .ok_or_else(invalid)?
        .deps()
        .to_vec())
}

fn frontier<T>(
    graph: &Automerge,
    candidates: Vec<Candidate<T>>,
) -> StorageResult<Vec<Candidate<T>>> {
    if candidates.len() < 2 {
        return Ok(candidates);
    }
    let contexts = candidates
        .iter()
        .map(|c| context(graph, &c.operation))
        .collect::<StorageResult<Vec<_>>>()?;
    let mut result = Vec::new();
    for candidate in candidates {
        let mut superseded = false;
        for context in &contexts {
            if graph
                .get_all_at(&candidate.object, candidate.field.as_str(), context)?
                .iter()
                .any(|(_, op)| *op == candidate.operation)
            {
                superseded = true;
                break;
            }
        }
        if !superseded {
            result.push(candidate);
        }
    }
    result.sort_by_key(|c| c.operation.to_string());
    Ok(result)
}

fn candidates<T>(
    graph: &Automerge,
    objects: &[ObjId],
    field: &str,
    decode: impl Fn(Value<'_>) -> Option<T>,
) -> StorageResult<Vec<Candidate<T>>> {
    let mut result = Vec::new();
    for object in objects {
        for (value, operation) in graph.get_all(object, field)? {
            result.push(Candidate {
                object: object.clone(),
                field: field.into(),
                operation,
                value: decode(value).ok_or_else(invalid)?,
            });
        }
    }
    frontier(graph, result)
}

pub(super) struct Group {
    pub ids: Vec<String>,
    pub names: Vec<String>,
    pub deleted: Vec<bool>,
    pub ambiguous: bool,
}

pub(super) fn groups(graph: &Automerge) -> StorageResult<Vec<Group>> {
    let mut births = BTreeMap::<String, Vec<(String, ObjId, Vec<ChangeHash>)>>::new();
    for root in graph.keys(ROOT) {
        if let Some(id) = root.strip_prefix("tag:") {
            if !valid_id(id) {
                return Err(invalid());
            }
            raw(graph, id)?;
            let object = object(graph, id)?;
            let hash = graph.hash_for_opid(&object).ok_or_else(invalid)?;
            let initial = graph.get_all_at(&object, "name", &[hash])?;
            if initial.len() != 1 {
                return Err(invalid());
            }
            let name = initial[0].0.as_str().ok_or_else(invalid)?;
            births.entry(name_key(name)).or_default().push((
                id.into(),
                object.clone(),
                context(graph, &object)?,
            ));
        }
    }
    let mut result = Vec::new();
    for mut cohort in births.into_values() {
        cohort.sort_by(|a, b| a.0.cmp(&b.0));
        let mut membership: Vec<usize> = (0..cohort.len()).collect();
        let mut ordered = BTreeSet::new();
        for (i, (left, _, left_context)) in cohort.iter().enumerate() {
            for (j, (right, _, right_context)) in cohort.iter().enumerate().skip(i + 1) {
                let concurrent = graph.hash_for_opid(&cohort[i].1)
                    != graph.hash_for_opid(&cohort[j].1)
                    && graph.get_at(ROOT, key(left), right_context)?.is_none()
                    && graph.get_at(ROOT, key(right), left_context)?.is_none();
                if concurrent {
                    let (from, to) = (
                        membership[i].max(membership[j]),
                        membership[i].min(membership[j]),
                    );
                    for member in &mut membership {
                        if *member == from {
                            *member = to;
                        }
                    }
                } else {
                    ordered.insert((i, j));
                }
            }
        }
        let ambiguous: BTreeSet<_> = ordered
            .into_iter()
            .filter(|(i, j)| membership[*i] == membership[*j])
            .map(|(i, _)| membership[i])
            .collect();
        let mut members = BTreeMap::<usize, Vec<usize>>::new();
        for (i, group) in membership.iter().enumerate() {
            members
                .entry(if ambiguous.contains(group) { i } else { *group })
                .or_default()
                .push(i);
        }
        for indices in members.into_values() {
            let objects: Vec<_> = indices.iter().map(|i| cohort[*i].1.clone()).collect();
            let names = candidates(graph, &objects, "name", |v| v.as_str().map(str::to_owned))?;
            let mut unique: Vec<String> = names.iter().map(|c| c.value.clone()).collect();
            if names
                .iter()
                .all(|c| graph.hash_for_opid(&c.operation) == graph.hash_for_opid(&c.object))
            {
                unique.truncate(1);
            } else {
                unique.sort();
                unique.dedup();
            }
            let mut deleted: Vec<_> = candidates(graph, &objects, "deleted", |v| v.as_bool())?
                .into_iter()
                .map(|c| c.value)
                .collect();
            deleted.sort();
            deleted.dedup();
            result.push(Group {
                ids: indices.iter().map(|i| cohort[*i].0.clone()).collect(),
                names: unique,
                deleted,
                ambiguous: ambiguous.contains(&membership[indices[0]]),
            });
        }
    }
    Ok(result)
}

pub(super) fn aliases(graph: &Automerge, tag: &str) -> StorageResult<Vec<String>> {
    object(graph, tag)?;
    groups(graph)?
        .into_iter()
        .find(|g| g.ids.iter().any(|id| id == tag))
        .map(|g| g.ids)
        .ok_or_else(invalid)
}

pub(crate) fn merged_assignments(
    graph: &Automerge,
    card: &ObjId,
    tags: &BTreeMap<String, TagSnapshot>,
) -> StorageResult<BTreeMap<String, Vec<bool>>> {
    let (map, assignments) = assignments(graph, card)?;
    let mut by_tag = BTreeMap::<String, Vec<Candidate<bool>>>::new();
    for alias in assignments.keys() {
        let tag = tags.get(alias).map_or(alias, |t| &t.tag);
        for (value, operation) in graph.get_all(&map, alias.as_str())? {
            by_tag.entry(tag.clone()).or_default().push(Candidate {
                object: map.clone(),
                field: alias.clone(),
                operation,
                value: value.as_bool().ok_or_else(invalid)?,
            });
        }
    }
    by_tag
        .into_iter()
        .map(|(tag, values)| {
            let mut values: Vec<_> = frontier(graph, values)?
                .into_iter()
                .map(|c| c.value)
                .collect();
            values.sort();
            values.dedup();
            Ok((tag, values))
        })
        .collect()
}

pub(crate) fn blocks_card(
    graph: &Automerge,
    tag: &str,
    tags: &BTreeMap<String, TagSnapshot>,
) -> StorageResult<bool> {
    let objects = tags
        .iter()
        .filter(|(_, snapshot)| snapshot.tag == tag)
        .map(|(id, _)| object(graph, id))
        .collect::<StorageResult<Vec<_>>>()?;
    Ok(candidates(graph, &objects, "deleted", |v| v.as_bool())?
        .iter()
        .any(|c| c.value))
}
