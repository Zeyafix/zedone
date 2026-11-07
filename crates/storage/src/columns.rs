use crate::{
    StorageError, StorageResult, WriteContext,
    spaces::{domain_projections, valid_group},
    state_version,
};
use automerge::{Automerge, ObjId, ObjType, ROOT, ReadDoc, transaction::Transactable};
use std::collections::BTreeMap;
use zedone_core::{
    columns::{ColumnSnapshot, ColumnStore},
    command::CommandResult,
    error::{CommandError, ErrorCode},
    spaces::{SpaceState, SpaceStore},
};

#[cfg(test)]
mod tests;

fn invalid() -> StorageError {
    StorageError::InvalidDatabase
}
fn key(column: &str) -> String {
    format!("column:{column}")
}
pub(crate) fn valid_id(id: &str) -> bool {
    id.len() == 32
        && id != "00000000000000000000000000000000"
        && id
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
pub(crate) fn known_root(key: &str) -> bool {
    matches!(key, "format" | "name" | "lifecycle" | "history")
        || key.strip_prefix("column:").is_some_and(valid_id)
        || key.strip_prefix("card:").is_some_and(valid_id)
}
type Projection = BTreeMap<String, serde_json::Value>;
struct Scope<'a> {
    unknown: BTreeMap<&'a str, &'a Projection>,
    known_content: bool,
    cards: crate::cards::Memberships<'a>,
}
fn scope<'a>(
    graph: &Automerge,
    projections: &'a BTreeMap<String, Projection>,
) -> StorageResult<Scope<'a>> {
    Ok(Scope {
        unknown: projections
            .iter()
            .filter(|(key, _)| !known_root(key))
            .map(|(key, value)| (key.as_str(), value))
            .collect(),
        known_content: graph.keys(ROOT).all(|key| known_root(&key)),
        cards: crate::cards::memberships(graph, projections)?,
    })
}
pub(crate) fn object(graph: &Automerge, column: &str) -> StorageResult<ObjId> {
    let values = graph.get_all(ROOT, key(column))?;
    if values.is_empty() {
        return Err(CommandError {
            code: ErrorCode::NotFound,
        }
        .into());
    }
    // Never silently choose a branch of conflicting container identity.
    if values.len() != 1 || values[0].0 != automerge::Value::Object(ObjType::Map) {
        return Err(invalid());
    }
    Ok(values[0].1.clone())
}
pub(crate) fn strings(graph: &Automerge, object: &ObjId, key: &str) -> StorageResult<Vec<String>> {
    let mut values = graph
        .get_all(object, key)?
        .into_iter()
        .map(|(v, _)| v.as_str().map(str::to_owned).ok_or_else(invalid))
        .collect::<StorageResult<Vec<_>>>()?;
    if values.is_empty() {
        return Err(invalid());
    }
    values.sort();
    values.dedup();
    Ok(values)
}
fn snapshot(
    graph: &Automerge,
    space: &str,
    column: &str,
    projection: &Projection,
    scope: &Scope<'_>,
) -> StorageResult<ColumnSnapshot> {
    let object = object(graph, column)?;
    if strings(graph, &object, "format")? != ["zedone.column.v1"] {
        return Err(invalid());
    }
    let names = strings(graph, &object, "name")?;
    let lifecycle = strings(graph, &object, "lifecycle")?;
    if lifecycle.iter().any(|s| s != "active" && !valid_group(s)) {
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
    let positions = crate::ordering::ranks(graph, &object)?;
    let empty = scope.known_content
        && !scope.cards.occupied.contains(column)
        && graph
            .keys(&object)
            .all(|k| matches!(k.as_str(), "format" | "name" | "lifecycle" | "position"));
    // Opaque root content could reference this column. Include its causal
    // history conservatively, including removal, until its schema is supported.
    let version = if let Some(cards) = scope.cards.related.get(column) {
        state_version(
            &("column.v1", space, column),
            &(projection, &scope.unknown, cards),
        )?
    } else {
        // Keep the original token for columns which have never had cards.
        state_version(&("column.v1", space, column), &(projection, &scope.unknown))?
    };
    Ok(ColumnSnapshot {
        space: space.into(),
        column: column.into(),
        names,
        state,
        deletion_groups,
        positions,
        empty,
        version,
    })
}

impl ColumnStore for WriteContext<'_> {
    fn create_column(
        &mut self,
        space: &str,
        column: &str,
        name: &str,
    ) -> CommandResult<ColumnSnapshot> {
        let result = (|| -> StorageResult<_> {
            if !self
                .read()
                .document(space)?
                .get_all(ROOT, key(column))?
                .is_empty()
            {
                return Err(CommandError {
                    code: ErrorCode::StaleState,
                }
                .into());
            }
            // Append after every existing position, including trash, so restore
            // retains the original place. Relative moves allocate dense paths.
            let position = crate::ordering::append(
                self.columns(space)?.into_iter().flat_map(|c| {
                    c.positions
                        .into_iter()
                        .map(move |rank| (c.column.clone(), rank))
                }),
                column,
            )?;
            self.edit(space, |doc| {
                let object = doc.put_object(ROOT, key(column), ObjType::Map)?;
                doc.put(&object, "format", "zedone.column.v1")?;
                doc.put(&object, "name", name)?;
                doc.put(&object, "lifecycle", "active")?;
                crate::ordering::put_rank(doc, &object, &position)?;
                Ok(())
            })?;
            Ok(self.column_snapshot(space, column)?)
        })();
        Ok(self.checked(result)?)
    }
    fn column_snapshot(&mut self, space: &str, column: &str) -> CommandResult<ColumnSnapshot> {
        self.space_snapshot(space)?; // Validate the enclosing document format.
        let graph = self.read().document(space)?;
        let projections = domain_projections(graph)?;
        let scope = scope(graph, &projections)?;
        Ok(snapshot(
            graph,
            space,
            column,
            projections.get(&key(column)).unwrap_or(&BTreeMap::new()),
            &scope,
        )?)
    }
    fn columns(&mut self, space: &str) -> CommandResult<Vec<ColumnSnapshot>> {
        self.space_snapshot(space)?;
        let graph = self.read().document(space)?;
        // Decode the graph once for the whole list, not once per column.
        let projections = domain_projections(graph)?;
        let scope = scope(graph, &projections)?;
        let mut result = Vec::new();
        for root in graph.keys(ROOT) {
            if let Some(column) = root.strip_prefix("column:") {
                if !valid_id(column) {
                    return Err(invalid().into());
                }
                result.push(snapshot(
                    graph,
                    space,
                    column,
                    projections.get(&root).ok_or_else(invalid)?,
                    &scope,
                )?);
            }
        }
        Ok(result)
    }
    fn rename_column(&mut self, space: &str, column: &str, name: &str) -> CommandResult<()> {
        let object = object(self.read().document(space)?, column)?;
        Ok(self.edit(space, |doc| {
            doc.put(&object, "name", name)?;
            Ok(())
        })?)
    }
    fn set_column_lifecycle(
        &mut self,
        space: &str,
        column: &str,
        lifecycle: &str,
    ) -> CommandResult<()> {
        let object = object(self.read().document(space)?, column)?;
        Ok(self.edit(space, |doc| {
            doc.put(&object, "lifecycle", lifecycle)?;
            Ok(())
        })?)
    }
}
