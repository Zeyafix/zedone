use crate::{StorageError, StorageResult, WriteContext, state_version};
use automerge::{
    Automerge, ROOT, ReadDoc,
    legacy::{Key, ObjectId, OpId},
    transaction::Transactable,
};
use std::collections::{BTreeMap, HashMap};
use zedone_core::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
    spaces::{SpaceContents, SpaceSnapshot, SpaceState, SpaceStore},
};

#[cfg(test)]
mod tests;

fn invalid() -> StorageError {
    StorageError::InvalidDatabase
}

/// Domain operation identities and values, including overwritten/deleted data.
/// Exclude only the reserved technical history branch and its descendants.
/// Never hash global heads, timestamps, receipt writes or peer progress.
fn projection(graph: &Automerge) -> StorageResult<BTreeMap<String, serde_json::Value>> {
    Ok(domain_projections(graph)?.into_values().flatten().collect())
}

/// A column owns its root entry and all descendants, including deleted ones.
pub(crate) fn domain_projections(
    graph: &Automerge,
) -> StorageResult<BTreeMap<String, BTreeMap<String, serde_json::Value>>> {
    projections(graph, false)
}

pub(crate) fn history_projections(
    graph: &Automerge,
) -> StorageResult<BTreeMap<String, BTreeMap<String, serde_json::Value>>> {
    projections(graph, true)
}

fn projections(
    graph: &Automerge,
    history: bool,
) -> StorageResult<BTreeMap<String, BTreeMap<String, serde_json::Value>>> {
    let mut parents = HashMap::new();
    let mut operations = Vec::new();
    for change in graph.get_changes(&[]) {
        let change = change.decode();
        for (index, op) in change.operations.into_iter().enumerate() {
            let id = OpId::new(change.start_op.get() + index as u64, &change.actor_id);
            if op.obj_type().is_some() {
                parents.insert(ObjectId::Id(id.clone()), (op.obj.clone(), op.key.clone()));
            }
            operations.push((id, op));
        }
    }
    let mut result = BTreeMap::new();
    for (id, op) in operations {
        let mut object = &op.obj;
        let mut key = &op.key;
        let mut root_child = None;
        while *object != ObjectId::Root {
            root_child = Some(key);
            let (parent, parent_key) = parents.get(object).ok_or_else(invalid)?;
            object = parent;
            key = parent_key;
        }
        if matches!(key, Key::Map(name) if name.as_str() == "history") {
            continue;
        }
        let Key::Map(name) = key else {
            return Err(invalid());
        };
        // Text history is shared data, but changing only its bookkeeping must
        // not invalidate a prepared edit or a container membership token.
        let is_history = name
            .strip_prefix("card:")
            .is_some_and(crate::columns::valid_id)
            && matches!(root_child, Some(Key::Map(field)) if field.as_str() == "history");
        if is_history != history {
            continue;
        }
        result
            .entry(name.to_string())
            .or_insert_with(BTreeMap::new)
            .insert(
                id.to_string(),
                serde_json::to_value(op).map_err(|_| invalid())?,
            );
    }
    Ok(result)
}

fn strings(graph: &Automerge, key: &str) -> StorageResult<Vec<String>> {
    let mut result = graph
        .get_all(ROOT, key)?
        .into_iter()
        .map(|(value, _)| value.as_str().map(str::to_owned).ok_or_else(invalid))
        .collect::<StorageResult<Vec<_>>>()?;
    if result.is_empty() {
        return Err(invalid());
    }
    result.sort();
    result.dedup();
    Ok(result)
}

fn snapshot(graph: &Automerge, space: &str) -> StorageResult<SpaceSnapshot> {
    if strings(graph, "format")? != ["zedone.space.v1"] {
        return Err(invalid());
    }
    if strings(graph, "lifecycle")? == ["purged"] {
        return Err(CommandError {
            code: ErrorCode::NotFound,
        }
        .into());
    }
    let names = strings(graph, "name")?;
    let lifecycle = strings(graph, "lifecycle")?;
    if lifecycle
        .iter()
        .any(|s| s != "active" && s != "purged" && !valid_group(s))
    {
        return Err(invalid());
    }
    let state = if lifecycle.len() != 1 {
        SpaceState::Conflicted
    } else if lifecycle[0] == "active" {
        SpaceState::Active
    } else {
        SpaceState::Trashed
    };
    let deletion_groups = lifecycle.into_iter().filter(|s| s != "active").collect();
    // Unknown/future content fails closed until its full lifecycle is supported.
    let empty = graph
        .keys(ROOT)
        .all(|key| matches!(key.as_str(), "format" | "name" | "lifecycle" | "history"));
    let version = state_version(&("space.v1", space), &projection(graph)?)?;
    Ok(SpaceSnapshot {
        space: space.into(),
        names,
        state,
        deletion_groups,
        empty,
        contents: if graph.keys(ROOT).all(|k| crate::columns::known_root(&k)) {
            let projections = domain_projections(graph)?;
            Some(SpaceContents {
                columns: crate::columns::snapshots(graph, space)?,
                cards: crate::cards::snapshots(graph, space, &projections)?,
                tags: crate::tags::snapshots(graph, space, &projections)?
                    .into_values()
                    .collect(),
            })
        } else {
            None
        },
        version,
    })
}

pub(crate) fn valid_group(group: &str) -> bool {
    let parts: Vec<_> = group.split(':').collect();
    parts.len() == 3
        && parts[0] == "trash"
        && parts[1..].iter().all(|s| {
            s.len() == 32
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        })
}

impl SpaceStore for WriteContext<'_> {
    fn create_space(
        &mut self,
        space: &str,
        name: &str,
        subject: &str,
    ) -> CommandResult<SpaceSnapshot> {
        let result = (|| -> StorageResult<SpaceSnapshot> {
            let exists: bool = self.connection().query_row(
                "SELECT EXISTS(SELECT 1 FROM documents WHERE space = ?1)",
                [space],
                |r| r.get(0),
            )?;
            if exists {
                return Err(CommandError {
                    code: ErrorCode::StaleState,
                }
                .into());
            }
            self.create_document(space)?;
            self.edit(space, |doc| {
                doc.put(ROOT, "format", "zedone.space.v1")?;
                doc.put(ROOT, "name", name)?;
                doc.put(ROOT, "lifecycle", "active")?;
                Ok(())
            })?;
            // The authenticated subject's existing key, administration, expiry
            // and other exact permissions are unchanged. No wildcard/owner.
            self.connection().execute(
                "INSERT INTO access_spaces VALUES (?1, ?2, 1)",
                (subject, space),
            )?;
            Ok(self.space_snapshot(space)?)
        })();
        Ok(self.checked(result)?)
    }

    fn space_snapshot(&mut self, space: &str) -> CommandResult<SpaceSnapshot> {
        Ok(snapshot(self.read().document(space)?, space)?)
    }

    fn spaces_for_subject(&mut self, subject: &str) -> CommandResult<Vec<SpaceSnapshot>> {
        let spaces = self
            .connection()
            .prepare("SELECT space FROM access_spaces WHERE subject = ?1 ORDER BY space")
            .map_err(StorageError::from)?
            .query_map([subject], |row| row.get::<_, String>(0))
            .map_err(StorageError::from)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(StorageError::from)?;
        let mut result = Vec::new();
        for space in spaces {
            if strings(self.read().document(&space)?, "lifecycle")? != ["purged"] {
                result.push(self.space_snapshot(&space)?);
            }
            self.read().release_document(&space);
        }
        Ok(result)
    }

    fn rename_space(&mut self, space: &str, name: &str) -> CommandResult<()> {
        Ok(self.edit(space, |doc| {
            doc.put(ROOT, "name", name)?;
            Ok(())
        })?)
    }
    fn set_space_lifecycle(&mut self, space: &str, lifecycle: &str) -> CommandResult<()> {
        Ok(self.edit(space, |doc| {
            doc.put(ROOT, "lifecycle", lifecycle)?;
            Ok(())
        })?)
    }
}
