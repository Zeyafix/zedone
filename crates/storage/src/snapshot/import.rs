use automerge::{ActorId, Automerge, Change, ChangeHash, ReadDoc, legacy::ObjectId};
use std::collections::{BTreeMap, BTreeSet};
use zedone_core::{
    error::{CommandError, ErrorCode},
    spaces::SpaceSnapshot as ImportedSpace,
};

use super::{SnapshotLimits, SpaceSnapshot};
use crate::{StorageError, StorageResult, WriteContext};

pub(crate) mod model;

fn invalid() -> StorageError {
    CommandError {
        code: ErrorCode::InvalidRequest,
    }
    .into()
}

fn identifiers(values: BTreeSet<String>) -> StorageResult<BTreeMap<String, String>> {
    let mut fresh = BTreeSet::new();
    while fresh.len() < values.len() {
        let mut bytes = [0; 16];
        getrandom::fill(&mut bytes).map_err(|_| StorageError::Unavailable)?;
        let id: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
        if crate::columns::valid_id(&id) && !values.contains(&id) {
            fresh.insert(id);
        }
    }
    Ok(values.into_iter().zip(fresh).collect())
}

fn prepare(snapshot: SpaceSnapshot, limits: SnapshotLimits) -> StorageResult<SpaceSnapshot> {
    let mut graph = Automerge::new();
    graph
        .apply_changes(snapshot.changes.iter().cloned())
        .map_err(|_| invalid())?;
    if !graph.get_missing_deps(&[]).is_empty() {
        return Err(invalid());
    }
    model::validate(&graph, &snapshot.metadata.space)?;
    let mut changes: Vec<_> = graph.get_changes(&[]).iter().map(Change::decode).collect();
    let mut schema = model::Schema::default();
    let mut ids = BTreeSet::from([snapshot.metadata.space.clone()]);
    for change in &mut changes {
        if !change.extra_bytes.is_empty() || change.author.is_some() {
            return Err(invalid());
        }
        schema.visit(change, &mut |id| {
            ids.insert(id.into());
            Ok(id.into())
        })?;
    }
    schema.check_references()?;
    let ids = identifiers(ids)?;
    let actors = identifiers(changes.iter().map(|c| c.actor_id.to_hex_string()).collect())?;
    let actors: BTreeMap<_, _> = changes
        .iter()
        .map(|c| {
            let actor = actors
                .get(&c.actor_id.to_hex_string())
                .ok_or_else(invalid)?;
            Ok((c.actor_id.clone(), ActorId::from(hex_bytes(actor)?)))
        })
        .collect::<StorageResult<_>>()?;
    let mut hashes = BTreeMap::<ChangeHash, ChangeHash>::new();
    let mut rewritten = Vec::new();
    for mut change in changes {
        let original = change.hash.ok_or_else(invalid)?;
        schema.visit(&mut change, &mut |id| {
            ids.get(id).cloned().ok_or_else(invalid)
        })?;
        for operation in &mut change.operations {
            if let ObjectId::Id(id) = &mut operation.obj {
                id.1 = actors.get(&id.1).cloned().ok_or_else(invalid)?;
            }
            operation.pred = operation
                .pred
                .iter()
                .map(|id| {
                    Ok(automerge::legacy::OpId::new(
                        id.counter(),
                        actors.get(id.actor()).ok_or_else(invalid)?,
                    ))
                })
                .collect::<StorageResult<Vec<_>>>()?
                .into();
        }
        change.actor_id = actors.get(&change.actor_id).cloned().ok_or_else(invalid)?;
        change.deps = change
            .deps
            .iter()
            .map(|hash| hashes.get(hash).copied().ok_or_else(invalid))
            .collect::<StorageResult<_>>()?;
        change.deps.sort();
        change.hash = None;
        let change = Change::from(change);
        hashes.insert(original, change.hash());
        rewritten.push(change);
    }
    let space = ids
        .get(&snapshot.metadata.space)
        .cloned()
        .ok_or_else(invalid)?;
    let mut graph = Automerge::new();
    graph
        .apply_changes(rewritten.clone())
        .map_err(|_| invalid())?;
    model::validate(&graph, &space)?;
    rewritten.sort_by_key(Change::hash);
    let snapshot = SpaceSnapshot {
        metadata: super::SnapshotMetadata {
            space,
            created_at_unix_ms: snapshot.metadata.created_at_unix_ms,
        },
        changes: rewritten,
    };
    snapshot
        .encode(limits)
        .map_err(|_| StorageError::Unavailable)?;
    Ok(snapshot)
}

fn hex_bytes(value: &str) -> StorageResult<Vec<u8>> {
    (0..value.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&value[i..i + 2], 16).map_err(|_| invalid()))
        .collect()
}

impl WriteContext<'_> {
    pub fn import_snapshot(
        &mut self,
        bytes: &[u8],
        limits: SnapshotLimits,
    ) -> StorageResult<ImportedSpace> {
        let result = (|| {
            let source = SpaceSnapshot::decode(bytes, limits).map_err(|_| invalid())?;
            let snapshot = prepare(source, limits)?;
            let space = snapshot.metadata.space;
            self.create_document(&space)?;
            self.apply_changes(&space, snapshot.changes)?;
            crate::spaces::snapshot(self.read().document(&space)?, &space, true)
        })();
        self.checked(result)
    }
}
