use automerge::{
    Automerge, ObjType, ScalarValue,
    legacy::{Change, Key, ObjectId, OpId, OpType},
};
use std::collections::{BTreeSet, HashMap};
use zedone_core::ordering::{CardPlacement, Rank};

use super::{StorageResult, invalid};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Kind {
    Root,
    Column,
    Card,
    Tag,
    Assignments,
    History,
    Version,
}

#[derive(Default)]
pub(crate) struct Schema {
    objects: HashMap<ObjectId, Kind>,
    definitions: BTreeSet<(Kind, String)>,
    references: BTreeSet<(Kind, String)>,
}

fn id(value: &str, map: &mut impl FnMut(&str) -> StorageResult<String>) -> StorageResult<String> {
    if !crate::columns::valid_id(value) {
        return Err(invalid());
    }
    map(value)
}

fn compound(
    value: &str,
    group: bool,
    map: &mut impl FnMut(&str) -> StorageResult<String>,
) -> StorageResult<String> {
    let parts: Vec<_> = value.split(':').collect();
    let parts = if group {
        if parts.len() != 3 || parts[0] != "trash" {
            return Err(invalid());
        }
        &parts[1..]
    } else {
        parts.as_slice()
    };
    if parts.len() != 2 || !crate::columns::valid_id(parts[0]) {
        return Err(invalid());
    }
    Ok(format!(
        "{}{}:{}",
        if group { "trash:" } else { "" },
        parts[0],
        id(parts[1], map)?
    ))
}

fn rank(
    value: &mut Rank,
    map: &mut impl FnMut(&str) -> StorageResult<String>,
) -> StorageResult<()> {
    if !value.valid() {
        return Err(invalid());
    }
    if let Rank::Path(path) = value {
        for part in path {
            if !part.id.is_empty() {
                part.id = id(&part.id, map)?;
            }
        }
    }
    Ok(())
}

fn unsigned(value: &ScalarValue) -> Option<u64> {
    match value {
        ScalarValue::Uint(value) => Some(*value),
        ScalarValue::Int(value) => u64::try_from(*value).ok(),
        _ => None,
    }
}

impl Schema {
    pub(crate) fn check_references(&self) -> StorageResult<()> {
        if !self.references.is_subset(&self.definitions) {
            return Err(invalid());
        }
        Ok(())
    }

    pub(crate) fn visit(
        &mut self,
        change: &mut Change,
        map: &mut impl FnMut(&str) -> StorageResult<String>,
    ) -> StorageResult<()> {
        for (index, op) in change.operations.iter_mut().enumerate() {
            if op.insert {
                return Err(invalid());
            }
            let parent = if op.obj == ObjectId::Root {
                Kind::Root
            } else {
                *self.objects.get(&op.obj).ok_or_else(invalid)?
            };
            let Key::Map(key) = &mut op.key else {
                return Err(invalid());
            };
            let field = key.to_string();
            let mut child = None;
            match parent {
                Kind::Root => {
                    for (prefix, kind) in [
                        ("column:", Kind::Column),
                        ("card:", Kind::Card),
                        ("tag:", Kind::Tag),
                    ] {
                        if let Some(original) = field.strip_prefix(prefix) {
                            let mapped = id(original, map)?;
                            *key = format!("{prefix}{mapped}").into();
                            child = Some(kind);
                            if matches!(op.action, OpType::Make(ObjType::Map)) {
                                self.definitions.insert((kind, original.into()));
                            }
                            break;
                        }
                    }
                }
                Kind::Card if field == "tags" => child = Some(Kind::Assignments),
                Kind::Card if field == "history" => child = Some(Kind::History),
                Kind::History => {
                    *key = compound(&field, false, map)?.into();
                    child = Some(Kind::Version);
                }
                Kind::Assignments => {
                    *key = id(&field, map)?.into();
                    self.references.insert((Kind::Tag, field.clone()));
                }
                _ => {}
            }
            match &mut op.action {
                OpType::Make(ObjType::Map) if child.is_some() => {
                    let counter = change
                        .start_op
                        .get()
                        .checked_add(index as u64)
                        .ok_or_else(invalid)?;
                    self.objects.insert(
                        ObjectId::Id(OpId::new(counter, &change.actor_id)),
                        child.ok_or_else(invalid)?,
                    );
                }
                OpType::Delete if child.is_some() => {}
                OpType::Put(value) if child.is_none() => self.scalar(parent, &field, value, map)?,
                OpType::Delete if child.is_none() => {
                    if !known(parent, &field) {
                        return Err(invalid());
                    }
                }
                _ => return Err(invalid()),
            }
        }
        Ok(())
    }

    fn scalar(
        &mut self,
        parent: Kind,
        field: &str,
        value: &mut ScalarValue,
        map: &mut impl FnMut(&str) -> StorageResult<String>,
    ) -> StorageResult<()> {
        if !known(parent, field) {
            return Err(invalid());
        }
        match field {
            "format" => {
                let expected = match parent {
                    Kind::Root => "zedone.space.v1",
                    Kind::Column => "zedone.column.v1",
                    Kind::Card => "zedone.card.v1",
                    Kind::Tag => "zedone.tag.v1",
                    _ => return Err(invalid()),
                };
                if value.as_str() != Some(expected) {
                    return Err(invalid());
                }
            }
            "name" | "title" if parent != Kind::Assignments => {
                if value.as_str().is_none_or(|s| s.trim().is_empty()) {
                    return Err(invalid());
                }
            }
            "description" if parent != Kind::Assignments => {
                if value.as_str().is_none() {
                    return Err(invalid());
                }
            }
            "lifecycle" => {
                let state = value.as_str().ok_or_else(invalid)?;
                if state != "active" && state != "purged" {
                    *value = compound(state, true, map)?.into();
                }
            }
            "column" => {
                let original = value.as_str().ok_or_else(invalid)?;
                self.references.insert((Kind::Column, original.into()));
                *value = id(original, map)?.into();
            }
            "position" => {
                if unsigned(value).is_none() {
                    let mut position: Rank =
                        serde_json::from_str(value.as_str().ok_or_else(invalid)?)
                            .map_err(|_| invalid())?;
                    rank(&mut position, map)?;
                    *value = serde_json::to_string(&position)
                        .map_err(|_| invalid())?
                        .into();
                }
            }
            "placement" => {
                let mut place: CardPlacement =
                    serde_json::from_str(value.as_str().ok_or_else(invalid)?)
                        .map_err(|_| invalid())?;
                self.references.insert((Kind::Column, place.column.clone()));
                place.column = id(&place.column, map)?;
                rank(&mut place.rank, map)?;
                *value = serde_json::to_string(&place).map_err(|_| invalid())?.into();
            }
            "sequence" => {
                if unsigned(value).is_none() {
                    return Err(invalid());
                }
            }
            "purge_boundary" => {
                if value.as_bool() != Some(true) {
                    return Err(invalid());
                }
            }
            _ => {
                if value.as_bool().is_none() {
                    return Err(invalid());
                }
            }
        }
        Ok(())
    }
}

fn known(parent: Kind, field: &str) -> bool {
    match parent {
        Kind::Root => matches!(
            field,
            "format" | "name" | "lifecycle" | "purge_boundary" | "restored_conflict"
        ),
        Kind::Column => matches!(
            field,
            "format" | "name" | "lifecycle" | "position" | "purge_boundary" | "restored_conflict"
        ),
        Kind::Card => matches!(
            field,
            "format"
                | "title"
                | "description"
                | "completed"
                | "lifecycle"
                | "column"
                | "position"
                | "placement"
                | "restored_conflict"
        ),
        Kind::Tag => matches!(
            field,
            "format" | "name" | "deleted" | "restored_conflict" | "retired_by_restore"
        ),
        Kind::Assignments => crate::columns::valid_id(field),
        Kind::Version => matches!(field, "sequence" | "title" | "description"),
        Kind::History => false,
    }
}

pub(crate) fn validate(graph: &Automerge, space: &str) -> StorageResult<()> {
    if !crate::columns::valid_id(space) {
        return Err(invalid());
    }
    let snapshot = crate::spaces::snapshot(graph, space, true).map_err(|_| invalid())?;
    if snapshot.contents.is_none() {
        return Err(invalid());
    }
    use automerge::ReadDoc;
    for root in graph.keys(automerge::ROOT) {
        if let Some(card) = root.strip_prefix("card:") {
            let object = crate::cards::object(graph, card).map_err(|_| invalid())?;
            crate::cards::history(graph, &object).map_err(|_| invalid())?;
        }
    }
    Ok(())
}
