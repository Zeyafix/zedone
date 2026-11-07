use crate::{
    StorageError, StorageResult, WriteContext,
    columns::{strings, valid_id},
    spaces::domain_projections,
    state_version,
};
use automerge::{Automerge, ObjId, ObjType, ROOT, ReadDoc, transaction::Transactable};
use std::collections::BTreeMap;
use zedone_core::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
    spaces::SpaceStore,
    tags::{CreateTag, SetCardTag, TagSnapshot, TagState, TagStore, name_key},
};

#[cfg(test)]
mod tests;

type Projection = BTreeMap<String, serde_json::Value>;
fn invalid() -> StorageError {
    StorageError::InvalidDatabase
}
fn key(tag: &str) -> String {
    format!("tag:{tag}")
}
fn object(graph: &Automerge, tag: &str) -> StorageResult<ObjId> {
    let values = graph.get_all(ROOT, key(tag))?;
    if values.is_empty() {
        return Err(CommandError {
            code: ErrorCode::NotFound,
        }
        .into());
    }
    if values.len() != 1 || values[0].0 != automerge::Value::Object(ObjType::Map) {
        return Err(invalid());
    }
    Ok(values[0].1.clone())
}

/// False is retained as a causal assignment value. Deleting a map entry would
/// silently lose removal against a concurrent re-assignment in Automerge.
pub(crate) fn assignments(
    graph: &Automerge,
    card: &ObjId,
) -> StorageResult<(ObjId, BTreeMap<String, Vec<bool>>)> {
    let map = graph.get_all(card, "tags")?;
    if map.len() != 1 || map[0].0 != automerge::Value::Object(ObjType::Map) {
        return Err(invalid());
    }
    let map = map[0].1.clone();
    let mut result = BTreeMap::new();
    for tag in graph.keys(&map) {
        if !valid_id(&tag) {
            return Err(invalid());
        }
        let mut values = graph
            .get_all(&map, &tag)?
            .into_iter()
            .map(|(v, _)| v.as_bool().ok_or_else(invalid))
            .collect::<StorageResult<Vec<_>>>()?;
        if values.is_empty() {
            return Err(invalid());
        }
        values.sort();
        values.dedup();
        result.insert(tag, values);
    }
    Ok((map, result))
}
fn deleted(graph: &Automerge, object: &ObjId) -> StorageResult<Vec<bool>> {
    let mut values = graph
        .get_all(object, "deleted")?
        .into_iter()
        .map(|(v, _)| v.as_bool().ok_or_else(invalid))
        .collect::<StorageResult<Vec<_>>>()?;
    if values.is_empty() {
        return Err(invalid());
    }
    values.sort();
    values.dedup();
    Ok(values)
}
struct RawTag {
    names: Vec<String>,
    deleted: Vec<bool>,
}
fn raw(graph: &Automerge, tag: &str) -> StorageResult<RawTag> {
    let object = object(graph, tag)?;
    if strings(graph, &object, "format")? != ["zedone.tag.v1"]
        || graph
            .keys(&object)
            .any(|k| !matches!(k.as_str(), "format" | "name" | "deleted"))
    {
        return Err(invalid());
    }
    let names = strings(graph, &object, "name")?;
    if names.iter().any(|n| n.trim().is_empty()) {
        return Err(invalid());
    }
    Ok(RawTag {
        names,
        deleted: deleted(graph, &object)?,
    })
}

/// A disputed name blocks assignment of that tag, not ordinary card edits.
/// Deletion against assignment is a conflict belonging to the card itself.
pub(crate) fn blocks_card(graph: &Automerge, tag: &str) -> StorageResult<bool> {
    Ok(deleted(graph, &object(graph, tag)?)? != [false])
}

/// One decode per list; no tag snapshot recursively reads card snapshots.
pub(crate) fn snapshots(
    graph: &Automerge,
    space: &str,
    projections: &BTreeMap<String, Projection>,
) -> StorageResult<BTreeMap<String, TagSnapshot>> {
    let mut raw_tags = BTreeMap::new();
    for root in graph.keys(ROOT) {
        if let Some(tag) = root.strip_prefix("tag:") {
            if !valid_id(tag) {
                return Err(invalid());
            }
            raw_tags.insert(tag.to_owned(), raw(graph, tag)?);
        }
    }
    if raw_tags.is_empty() {
        return Ok(BTreeMap::new());
    }
    let mut links = BTreeMap::<String, BTreeMap<String, Projection>>::new();
    let mut live = std::collections::BTreeSet::new();
    for root in graph.keys(ROOT) {
        if let Some(card) = root.strip_prefix("card:") {
            let object = crate::cards::object(graph, card)?;
            let (map, assignments) = assignments(graph, &object)?;
            let assignment_map = map.to_string();
            for (tag, values) in assignments {
                if !raw_tags.contains_key(&tag) {
                    // The card reader marks dangling assignments conflicted.
                    continue;
                }
                if values.contains(&true) {
                    live.insert(tag.clone());
                }
                // Includes false assignments: ABA never revives old deletions.
                // Scope only assignment operations, not unrelated card text.
                let projection = projections.get(&root).ok_or_else(invalid)?;
                let related = projection
                    .iter()
                    .filter(|(_, op)| {
                        op["obj"].as_str() == Some(assignment_map.as_str()) && op["key"] == tag
                    })
                    .map(|(id, op)| (id.clone(), op.clone()))
                    .collect();
                links.entry(tag).or_default().insert(root.clone(), related);
            }
        }
    }
    let unknown: BTreeMap<_, _> = projections
        .iter()
        .filter(|(k, _)| !crate::columns::known_root(k))
        .collect();
    let names: BTreeMap<_, _> = raw_tags
        .keys()
        .map(|id| Ok((id, projections.get(&key(id)).ok_or_else(invalid)?)))
        .collect::<StorageResult<_>>()?;
    let mut result = BTreeMap::new();
    for (tag, raw) in &raw_tags {
        let collision = raw.deleted != [true]
            && raw_tags.iter().any(|(other_id, other)| {
                other_id != tag
                    && other.deleted != [true]
                    && raw
                        .names
                        .iter()
                        .any(|n| other.names.iter().any(|m| name_key(n) == name_key(m)))
            });
        let state = if raw.names.len() != 1
            || raw.deleted.len() != 1
            || collision
            || (raw.deleted == [true] && live.contains(tag))
        {
            TagState::Conflicted
        } else if raw.deleted == [true] {
            TagState::Deleted
        } else {
            TagState::Active
        };
        let projection = projections.get(&key(tag)).ok_or_else(invalid)?;
        // Name collisions affect authorization of ordinary actions, so their
        // causal roots must also affect this token, including collision ABA.
        let version = state_version(
            &("tag.v1", space, tag),
            &(projection, links.get(tag), &unknown, &names),
        )?;
        result.insert(
            tag.clone(),
            TagSnapshot {
                space: space.into(),
                tag: tag.clone(),
                names: raw.names.clone(),
                state,
                version,
            },
        );
    }
    Ok(result)
}

impl TagStore for WriteContext<'_> {
    fn create_tag(&mut self, p: &CreateTag) -> CommandResult<TagSnapshot> {
        let result = (|| -> StorageResult<_> {
            if !self
                .read()
                .document(&p.space)?
                .get_all(ROOT, key(&p.tag))?
                .is_empty()
            {
                return Err(CommandError {
                    code: ErrorCode::StaleState,
                }
                .into());
            }
            self.edit(&p.space, |doc| {
                let object = doc.put_object(ROOT, key(&p.tag), ObjType::Map)?;
                doc.put(&object, "format", "zedone.tag.v1")?;
                doc.put(&object, "name", p.name.as_str())?;
                doc.put(&object, "deleted", false)?;
                Ok(())
            })?;
            Ok(self.tag_snapshot(&p.space, &p.tag)?)
        })();
        Ok(self.checked(result)?)
    }
    fn tag_snapshot(&mut self, space: &str, tag: &str) -> CommandResult<TagSnapshot> {
        self.space_snapshot(space)?;
        let graph = self.read().document(space)?;
        object(graph, tag)?;
        Ok(snapshots(graph, space, &domain_projections(graph)?)?
            .remove(tag)
            .ok_or_else(invalid)?)
    }
    fn tags(&mut self, space: &str) -> CommandResult<Vec<TagSnapshot>> {
        self.space_snapshot(space)?;
        let graph = self.read().document(space)?;
        Ok(snapshots(graph, space, &domain_projections(graph)?)?
            .into_values()
            .collect())
    }
    fn rename_tag(&mut self, space: &str, tag: &str, name: &str) -> CommandResult<()> {
        let object = object(self.read().document(space)?, tag)?;
        Ok(self.edit(space, |doc| {
            doc.put(&object, "name", name)?;
            Ok(())
        })?)
    }
    fn delete_tag(&mut self, space: &str, tag: &str) -> CommandResult<()> {
        let result = (|| -> StorageResult<_> {
            let graph = self.read().document(space)?;
            let object = object(graph, tag)?;
            let mut maps = Vec::new();
            for root in graph.keys(ROOT) {
                if let Some(card) = root.strip_prefix("card:") {
                    let card = crate::cards::object(graph, card)?;
                    let (map, assignments) = assignments(graph, &card)?;
                    if assignments.get(tag).is_some_and(|v| v.contains(&true)) {
                        maps.push(map);
                    }
                }
            }
            self.edit(space, |doc| {
                for map in maps {
                    doc.put(&map, tag, false)?;
                }
                doc.put(&object, "deleted", true)?;
                Ok(())
            })
        })();
        Ok(self.checked(result)?)
    }
    fn set_card_tag(&mut self, p: &SetCardTag) -> CommandResult<()> {
        let graph = self.read().document(&p.space)?;
        let card = crate::cards::object(graph, &p.card)?;
        let map = assignments(graph, &card)?.0;
        Ok(self.edit(&p.space, |doc| {
            doc.put(&map, p.tag.as_str(), p.present)?;
            Ok(())
        })?)
    }
}
