use crate::{
    StorageError, StorageResult, WriteContext,
    columns::{object as column_object, strings, valid_id},
    spaces::{domain_projections, history_projections, valid_group},
    state_version,
};
use automerge::{Automerge, ObjId, ObjType, ROOT, ReadDoc, transaction::Transactable};
use std::collections::{BTreeMap, BTreeSet};
use zedone_core::{
    cards::{CardHistory, CardSnapshot, CardStore, CardTextVersion, CreateCard, EditCard},
    command::CommandResult,
    error::{CommandError, ErrorCode},
    spaces::{SpaceState, SpaceStore},
};

#[cfg(test)]
mod tests;

type Projection = BTreeMap<String, serde_json::Value>;
fn invalid() -> StorageError {
    StorageError::InvalidDatabase
}
fn key(card: &str) -> String {
    format!("card:{card}")
}
pub(crate) fn object(graph: &Automerge, card: &str) -> StorageResult<ObjId> {
    let values = graph.get_all(ROOT, key(card))?;
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
fn map(graph: &Automerge, object: &ObjId, field: &str) -> StorageResult<ObjId> {
    let values = graph.get_all(object, field)?;
    if values.len() != 1 || values[0].0 != automerge::Value::Object(ObjType::Map) {
        return Err(invalid());
    }
    Ok(values[0].1.clone())
}
fn registers<T: Ord>(
    graph: &Automerge,
    object: &ObjId,
    field: &str,
    decode: impl Fn(automerge::Value<'_>) -> Option<T>,
) -> StorageResult<Vec<T>> {
    let mut values = graph
        .get_all(object, field)?
        .into_iter()
        .map(|(v, _)| decode(v).ok_or_else(invalid))
        .collect::<StorageResult<Vec<_>>>()?;
    if values.is_empty() {
        return Err(invalid());
    }
    values.sort();
    values.dedup();
    Ok(values)
}
fn history(graph: &Automerge, card: &ObjId) -> StorageResult<Vec<CardTextVersion>> {
    let history = map(graph, card, "history")?;
    let mut result = Vec::new();
    for id in graph.keys(&history) {
        let parts: Vec<_> = id.split(':').collect();
        if parts.len() != 2 || !parts.iter().all(|s| valid_id(s)) {
            return Err(invalid());
        }
        let version = map(graph, &history, &id)?;
        if graph
            .keys(&version)
            .any(|k| !matches!(k.as_str(), "title" | "description" | "sequence"))
        {
            return Err(invalid());
        }
        let titles = strings(graph, &version, "title")?;
        let descriptions = strings(graph, &version, "description")?;
        let sequences = registers(graph, &version, "sequence", |v| v.as_u64())?;
        if titles.len() != 1
            || descriptions.len() != 1
            || sequences.len() != 1
            || titles[0].trim().is_empty()
        {
            return Err(invalid());
        }
        result.push(CardTextVersion {
            id: id.clone(),
            subject: parts[0].into(),
            sequence: sequences[0],
            title: titles[0].clone(),
            description: descriptions[0].clone(),
        });
    }
    if result.is_empty() {
        return Err(invalid());
    }
    result.sort_by(|a, b| a.sequence.cmp(&b.sequence).then(a.id.cmp(&b.id)));
    Ok(result)
}
fn snapshot(
    graph: &Automerge,
    space: &str,
    card: &str,
    projections: &BTreeMap<String, Projection>,
) -> StorageResult<CardSnapshot> {
    let object = object(graph, card)?;
    if strings(graph, &object, "format")? != ["zedone.card.v1"]
        || graph.keys(&object).any(|k| {
            !matches!(
                k.as_str(),
                "format"
                    | "title"
                    | "description"
                    | "column"
                    | "position"
                    | "placement"
                    | "completed"
                    | "tags"
                    | "lifecycle"
                    | "history"
            )
        })
    {
        return Err(invalid());
    }
    let titles = strings(graph, &object, "title")?;
    if titles.iter().any(|s| s.trim().is_empty()) {
        return Err(invalid());
    }
    let descriptions = strings(graph, &object, "description")?;
    let placements = crate::ordering::placements(graph, &object)?;
    let (mut columns, mut positions) = if placements.is_empty() {
        (
            strings(graph, &object, "column")?,
            crate::ordering::ranks(graph, &object)?,
        )
    } else {
        (
            placements.iter().map(|p| p.column.clone()).collect(),
            placements.iter().map(|p| p.rank.clone()).collect(),
        )
    };
    columns.sort();
    columns.dedup();
    positions.sort_by_key(|r| r.key(card));
    positions.dedup();
    for column in &columns {
        if !valid_id(column) {
            return Err(invalid());
        }
        column_object(graph, column)?;
    }
    let completed = registers(graph, &object, "completed", |v| v.as_bool())?;
    let tags_object = map(graph, &object, "tags")?;
    let mut tags = Vec::new();
    for tag in graph.keys(&tags_object) {
        if !valid_id(&tag) || registers(graph, &tags_object, &tag, |v| v.as_bool())? != [true] {
            return Err(invalid());
        }
        tags.push(tag);
    }
    tags.sort();
    let lifecycle = strings(graph, &object, "lifecycle")?;
    if lifecycle.iter().any(|s| s != "active" && !valid_group(s)) {
        return Err(invalid());
    }
    let conflicted = placements.len() > 1
        || titles.len() != 1
        || descriptions.len() != 1
        || columns.len() != 1
        || positions.len() != 1
        || completed.len() != 1
        || lifecycle.len() != 1;
    let state = if conflicted {
        SpaceState::Conflicted
    } else if lifecycle[0] == "active" {
        SpaceState::Active
    } else {
        SpaceState::Trashed
    };
    let deletion_groups = lifecycle.into_iter().filter(|s| s != "active").collect();
    let unknown: BTreeMap<_, _> = projections
        .iter()
        .filter(|(k, _)| !crate::columns::known_root(k))
        .collect();
    let projection = projections.get(&key(card)).ok_or_else(invalid)?;
    let version = state_version(&("card.v1", space, card), &(projection, unknown))?;
    map(graph, &object, "history")?;
    Ok(CardSnapshot {
        space: space.into(),
        card: card.into(),
        titles,
        descriptions,
        columns,
        positions,
        completed,
        placements,
        tags,
        state,
        deletion_groups,
        version,
    })
}

/// Current AND removed memberships participate in column freshness. A card
/// stays one root object across moves; no clone is used as membership.
pub(crate) struct Memberships<'a> {
    pub related: BTreeMap<String, BTreeMap<&'a str, &'a Projection>>,
    pub occupied: BTreeSet<String>,
}
pub(crate) fn memberships<'a>(
    graph: &Automerge,
    projections: &'a BTreeMap<String, Projection>,
) -> StorageResult<Memberships<'a>> {
    let mut related: BTreeMap<String, BTreeMap<&str, &Projection>> = BTreeMap::new();
    let mut occupied = BTreeSet::new();
    for root in graph.keys(ROOT) {
        if let Some(card) = root.strip_prefix("card:") {
            if !valid_id(card) {
                return Err(invalid());
            }
            let object = object(graph, card)?;
            let places = crate::ordering::placements(graph, &object)?;
            let columns = if places.is_empty() {
                strings(graph, &object, "column")?
            } else {
                places.into_iter().map(|p| p.column).collect()
            };
            for column in columns {
                if !valid_id(&column) {
                    return Err(invalid());
                }
                occupied.insert(column);
            }
        }
    }
    for (root, projection) in projections {
        if root.strip_prefix("card:").is_some_and(valid_id) {
            for op in projection.values() {
                let column = if op["key"] == "column" {
                    op["value"].as_str().map(str::to_owned)
                } else if op["key"] == "placement" {
                    Some(
                        crate::ordering::decode_place(op["value"].as_str().ok_or_else(invalid)?)?
                            .column,
                    )
                } else {
                    None
                };
                if let Some(column) = column {
                    if !valid_id(&column) {
                        return Err(invalid());
                    }
                    related
                        .entry(column)
                        .or_default()
                        .insert(root.as_str(), projection);
                }
            }
        }
    }
    Ok(Memberships { related, occupied })
}

fn append_version(
    doc: &mut automerge::transaction::Transaction<'_>,
    history: &ObjId,
    id: &str,
    sequence: u64,
    title: &str,
    description: &str,
) -> StorageResult<()> {
    let v = doc.put_object(history, id, ObjType::Map)?;
    doc.put(&v, "sequence", sequence)?;
    doc.put(&v, "title", title)?;
    doc.put(&v, "description", description)?;
    Ok(())
}
impl CardStore for WriteContext<'_> {
    fn create_card(
        &mut self,
        p: &CreateCard,
        subject: &str,
        command: &str,
    ) -> CommandResult<CardSnapshot> {
        let result = (|| -> StorageResult<_> {
            if !self
                .read()
                .document(&p.space)?
                .get_all(ROOT, key(&p.card))?
                .is_empty()
            {
                return Err(CommandError {
                    code: ErrorCode::StaleState,
                }
                .into());
            }
            let position = crate::ordering::append(
                self.cards(&p.space)?
                    .into_iter()
                    .filter(|c| c.columns.contains(&p.column))
                    .flat_map(|c| {
                        c.positions
                            .into_iter()
                            .map(move |rank| (c.card.clone(), rank))
                    }),
                &p.card,
            )?;
            self.edit(&p.space, |doc| {
                let object = doc.put_object(ROOT, key(&p.card), ObjType::Map)?;
                doc.put(&object, "format", "zedone.card.v1")?;
                doc.put(&object, "title", p.title.as_str())?;
                doc.put(&object, "description", p.description.as_str())?;
                doc.put(&object, "column", p.column.as_str())?;
                crate::ordering::put_rank(doc, &object, &position)?;
                doc.put(&object, "completed", false)?;
                doc.put_object(&object, "tags", ObjType::Map)?;
                doc.put(&object, "lifecycle", "active")?;
                let history = doc.put_object(&object, "history", ObjType::Map)?;
                append_version(
                    doc,
                    &history,
                    &format!("{subject}:{command}"),
                    0,
                    &p.title,
                    &p.description,
                )
            })?;
            Ok(self.card_snapshot(&p.space, &p.card)?)
        })();
        Ok(self.checked(result)?)
    }
    fn card_snapshot(&mut self, space: &str, card: &str) -> CommandResult<CardSnapshot> {
        self.space_snapshot(space)?;
        let graph = self.read().document(space)?;
        Ok(snapshot(graph, space, card, &domain_projections(graph)?)?)
    }
    fn cards(&mut self, space: &str) -> CommandResult<Vec<CardSnapshot>> {
        self.space_snapshot(space)?;
        let graph = self.read().document(space)?;
        let projections = domain_projections(graph)?;
        let mut result = Vec::new();
        for root in graph.keys(ROOT) {
            if let Some(card) = root.strip_prefix("card:") {
                if !valid_id(card) {
                    return Err(invalid().into());
                }
                result.push(snapshot(graph, space, card, &projections)?);
            }
        }
        Ok(result)
    }
    fn card_history(&mut self, space: &str, card: &str) -> CommandResult<CardHistory> {
        self.card_snapshot(space, card)?;
        let graph = self.read().document(space)?;
        let object = object(graph, card)?;
        let projections = history_projections(graph)?;
        let projection = projections.get(&key(card)).ok_or_else(invalid)?;
        Ok(CardHistory {
            space: space.into(),
            card: card.into(),
            versions: history(graph, &object)?,
            next: None,
            version: state_version(&("card.history.v1", space, card), projection)?,
        })
    }
    fn edit_card(&mut self, p: &EditCard, subject: &str, command: &str) -> CommandResult<()> {
        let result = (|| -> StorageResult<_> {
            let before = self.card_snapshot(&p.space, &p.card)?;
            if before.titles == [p.title.as_str()]
                && before.descriptions == [p.description.as_str()]
            {
                return Ok(());
            }
            let graph = self.read().document(&p.space)?;
            let object = object(graph, &p.card)?;
            let sequence = history(graph, &object)?
                .iter()
                .map(|v| v.sequence)
                .max()
                .and_then(|n| n.checked_add(1))
                .ok_or_else(invalid)?;
            let history = map(graph, &object, "history")?;
            self.edit(&p.space, |doc| {
                doc.put(&object, "title", p.title.as_str())?;
                doc.put(&object, "description", p.description.as_str())?;
                append_version(
                    doc,
                    &history,
                    &format!("{subject}:{command}"),
                    sequence,
                    &p.title,
                    &p.description,
                )
            })
        })();
        Ok(self.checked(result)?)
    }
    fn set_card_completed(
        &mut self,
        space: &str,
        card: &str,
        completed: bool,
    ) -> CommandResult<()> {
        let object = object(self.read().document(space)?, card)?;
        Ok(self.edit(space, |doc| {
            doc.put(&object, "completed", completed)?;
            Ok(())
        })?)
    }
    fn set_card_lifecycle(
        &mut self,
        space: &str,
        card: &str,
        lifecycle: &str,
    ) -> CommandResult<()> {
        let object = object(self.read().document(space)?, card)?;
        Ok(self.edit(space, |doc| {
            doc.put(&object, "lifecycle", lifecycle)?;
            Ok(())
        })?)
    }
}
