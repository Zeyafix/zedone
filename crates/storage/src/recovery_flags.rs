use crate::{StorageError, StorageResult};
use automerge::{Automerge, ObjId, ReadDoc};
pub(crate) const CONFLICT: &str = "restored_conflict";
pub(crate) const RETIRED_TAG: &str = "retired_by_restore";
pub(crate) fn active(
    graph: &Automerge,
    object: &ObjId,
    marker: &str,
    field: &str,
) -> StorageResult<bool> {
    for (value, operation) in graph.get_all(object, marker)? {
        if !value.as_bool().ok_or(StorageError::InvalidDatabase)? {
            continue;
        }
        let hash = graph
            .hash_for_opid(&operation)
            .ok_or(StorageError::InvalidDatabase)?;
        let mut resolved = false;
        for (_, current) in graph.get_all(object, field)? {
            let current_hash = graph
                .hash_for_opid(&current)
                .ok_or(StorageError::InvalidDatabase)?;
            if current_hash != hash
                && graph
                    .get_all_at(object, marker, &[current_hash])?
                    .iter()
                    .any(|(_, seen)| *seen == operation)
            {
                resolved = true;
            }
        }
        if !resolved {
            return Ok(true);
        }
    }
    Ok(false)
}
