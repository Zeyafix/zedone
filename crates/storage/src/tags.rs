use crate::{
    StorageError, StorageResult, WriteContext,
    columns::{strings, valid_id},
    spaces::domain_projections,
    state_version,
};
use automerge::{Automerge, ObjId, ObjType, ROOT, ReadDoc, transaction::Transactable};
use std::collections::BTreeMap;
use zedone_core::{
    cards::CardStore,
    command::CommandResult,
    error::{CommandError, ErrorCode},
    spaces::SpaceStore,
    tags::{CreateTag, SetCardTag, TagSnapshot, TagState, TagStore, name_key},
};

pub(crate) mod convergence;
mod name_groups;
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
fn raw(graph: &Automerge, tag: &str) -> StorageResult<()> {
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
    deleted(graph, &object)?;
    Ok(())
}

pub(crate) fn snapshots(
    graph: &Automerge,
    space: &str,
    projections: &BTreeMap<String, Projection>,
) -> StorageResult<BTreeMap<String, TagSnapshot>> {
    let groups = convergence::groups(graph)?;
    if groups.is_empty() {
        return Ok(BTreeMap::new());
    }
    let name_groups = name_groups::groups(graph, &groups, projections)?;
    let mut links = BTreeMap::<String, BTreeMap<String, Projection>>::new();
    for root in graph.keys(ROOT) {
        if let Some(card) = root.strip_prefix("card:") {
            let object = crate::cards::object(graph, card)?;
            let (map, assignments) = assignments(graph, &object)?;
            let assignment_map = map.to_string();
            for (tag, _) in assignments {
                if !groups.iter().any(|g| g.ids.contains(&tag)) {
                    continue;
                }
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
    let names: BTreeMap<_, _> = groups
        .iter()
        .flat_map(|g| &g.ids)
        .map(|id| Ok((id, projections.get(&key(id)).ok_or_else(invalid)?)))
        .collect::<StorageResult<_>>()?;
    let mut result = BTreeMap::new();
    for raw in &groups {
        let tag = &raw.ids[0];
        let collision = raw.deleted != [true]
            && groups.iter().any(|other| {
                other.ids[0] != *tag
                    && other.deleted != [true]
                    && raw
                        .names
                        .iter()
                        .any(|n| other.names.iter().any(|m| name_key(n) == name_key(m)))
            });
        let state = if raw.deleted == [true] {
            TagState::Deleted
        } else if raw.names.len() != 1 || raw.deleted.len() != 1 || collision || raw.ambiguous {
            TagState::Conflicted
        } else {
            TagState::Active
        };
        let projection: BTreeMap<_, _> = raw
            .ids
            .iter()
            .map(|id| Ok((id, projections.get(&key(id)).ok_or_else(invalid)?)))
            .collect::<StorageResult<_>>()?;
        let links: BTreeMap<_, _> = raw.ids.iter().map(|id| (id, links.get(id))).collect();
        let version = state_version(
            &("tag.v1", space, tag),
            &(projection, links, &unknown, &names),
        )?;
        let snapshot = TagSnapshot {
            space: space.into(),
            tag: tag.clone(),
            names: raw.names.clone(),
            state,
            deletion_conflict: raw.deleted == [false, true],
            name_group: name_groups
                .get(tag)
                .filter(|ids| ids.len() > 1 || raw.names.len() > 1)
                .cloned()
                .unwrap_or_default(),
            version,
        };
        for id in &raw.ids {
            result.insert(id.clone(), snapshot.clone());
        }
    }
    let blocked: std::collections::BTreeSet<_> = result
        .values()
        .filter(|t| t.state == TagState::Conflicted)
        .flat_map(|t| t.name_group.iter().cloned())
        .collect();
    for snapshot in result.values_mut() {
        if blocked.contains(&snapshot.tag) {
            snapshot.state = TagState::Conflicted;
        }
    }
    Ok(result)
}

impl TagStore for WriteContext<'_> {
    fn resolve_tag_deletion(&mut self, space: &str, tag: &str, keep: bool) -> CommandResult<()> {
        let graph = self.read().document(space)?;
        let objects = convergence::aliases(graph, tag)?
            .iter()
            .map(|id| object(graph, id))
            .collect::<StorageResult<Vec<_>>>()?;
        Ok(self.edit(space, |doc| {
            for object in objects {
                doc.delete(&object, "deleted")?;
                doc.put(&object, "deleted", !keep)?;
            }
            Ok(())
        })?)
    }
    fn tag_name_group(&mut self, space: &str, tag: &str) -> CommandResult<Vec<TagSnapshot>> {
        let snapshot = self.tag_snapshot(space, tag)?;
        let graph = self.read().document(space)?;
        let groups = convergence::groups(graph)?;
        if snapshot.name_group.is_empty()
            || groups
                .iter()
                .any(|g| snapshot.name_group.contains(&g.ids[0]) && g.deleted != [false])
        {
            return Err(CommandError {
                code: ErrorCode::Conflict,
            });
        }
        let snapshots = snapshots(graph, space, &domain_projections(graph)?)?;
        Ok(snapshot
            .name_group
            .iter()
            .map(|id| snapshots.get(id).cloned().ok_or_else(invalid))
            .collect::<StorageResult<_>>()?)
    }
    fn resolve_tag_names(
        &mut self,
        space: &str,
        names: &[zedone_core::tag_names::TagNameChoice],
    ) -> CommandResult<()> {
        let graph = self.read().document(space)?;
        let objects = names
            .iter()
            .map(|n| {
                convergence::aliases(graph, &n.tag)?
                    .iter()
                    .map(|id| object(graph, id))
                    .collect::<StorageResult<Vec<_>>>()
            })
            .collect::<StorageResult<Vec<_>>>()?;
        Ok(self.edit(space, |doc| {
            for (choice, aliases) in names.iter().zip(objects) {
                for object in aliases {
                    doc.delete(&object, "name")?;
                    doc.put(&object, "name", choice.name.as_str())?;
                }
            }
            Ok(())
        })?)
    }
    fn card_tag_assignments(
        &mut self,
        space: &str,
        card: &str,
    ) -> CommandResult<Vec<zedone_core::tag_assignments::TagAssignment>> {
        self.card_snapshot(space, card)?;
        let graph = self.read().document(space)?;
        let tags = snapshots(graph, space, &domain_projections(graph)?)?;
        Ok(
            convergence::merged_assignments(graph, &crate::cards::object(graph, card)?, &tags)?
                .into_iter()
                .map(|(tag, values)| zedone_core::tag_assignments::TagAssignment { tag, values })
                .collect(),
        )
    }
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
            .into_iter()
            .filter(|(id, tag)| *id == tag.tag)
            .map(|(_, tag)| tag)
            .collect())
    }
    fn rename_tag(&mut self, space: &str, tag: &str, name: &str) -> CommandResult<()> {
        let graph = self.read().document(space)?;
        let objects = convergence::aliases(graph, tag)?
            .iter()
            .map(|id| object(graph, id))
            .collect::<StorageResult<Vec<_>>>()?;
        Ok(self.edit(space, |doc| {
            for object in objects {
                doc.put(&object, "name", name)?;
            }
            Ok(())
        })?)
    }
    fn delete_tag(&mut self, space: &str, tag: &str) -> CommandResult<()> {
        let result = (|| -> StorageResult<_> {
            let graph = self.read().document(space)?;
            let aliases = convergence::aliases(graph, tag)?;
            let objects = aliases
                .iter()
                .map(|id| object(graph, id))
                .collect::<StorageResult<Vec<_>>>()?;
            let mut maps = Vec::new();
            for root in graph.keys(ROOT) {
                if let Some(card) = root.strip_prefix("card:") {
                    let card = crate::cards::object(graph, card)?;
                    let (map, assignments) = assignments(graph, &card)?;
                    if aliases
                        .iter()
                        .any(|id| assignments.get(id).is_some_and(|v| v.contains(&true)))
                    {
                        maps.push(map);
                    }
                }
            }
            self.edit(space, |doc| {
                for map in maps {
                    for alias in &aliases {
                        doc.put(&map, alias.as_str(), false)?;
                    }
                }
                for object in objects {
                    doc.put(&object, "deleted", true)?;
                }
                Ok(())
            })
        })();
        Ok(self.checked(result)?)
    }
    fn set_card_tag(&mut self, p: &SetCardTag) -> CommandResult<()> {
        let graph = self.read().document(&p.space)?;
        let card = crate::cards::object(graph, &p.card)?;
        let map = assignments(graph, &card)?.0;
        let aliases = convergence::aliases(graph, &p.tag)?;
        Ok(self.edit(&p.space, |doc| {
            for alias in aliases {
                doc.delete(&map, alias.as_str())?;
                doc.put(&map, alias, p.present)?;
            }
            Ok(())
        })?)
    }
}
