use super::*;

pub(super) fn links(
    graph: &Automerge,
    map: &ObjId,
    alias: &str,
    ids: &[String],
    heads: Option<&[ChangeHash]>,
) -> StorageResult<Vec<Candidate<bool>>> {
    let mut values = match heads {
        Some(heads) => graph.get_all_at(map, alias, heads)?,
        None => graph.get_all(map, alias)?,
    };
    let mut result = Vec::new();
    let mut seen = BTreeSet::new();
    while let Some((value, operation)) = values.pop() {
        if !seen.insert(operation.to_string()) {
            continue;
        }
        let value = value.as_bool().ok_or_else(invalid)?;
        let hash = graph.hash_for_opid(&operation).ok_or_else(invalid)?;
        let mut cleared_by_deletion = false;
        if !value {
            for id in ids {
                cleared_by_deletion |= graph
                    .get_all_at(object(graph, id)?, "deleted", &[hash])?
                    .iter()
                    .any(|(v, op)| {
                        v.as_bool() == Some(true) && graph.hash_for_opid(op) == Some(hash)
                    });
            }
        }
        if cleared_by_deletion {
            values.extend(graph.get_all_at(map, alias, &context(graph, &operation)?)?);
        } else {
            result.push(Candidate {
                object: map.clone(),
                operation,
                value,
            });
        }
    }
    Ok(result)
}

pub(super) fn values(graph: &Automerge, ids: &[String]) -> StorageResult<Vec<bool>> {
    let objects = ids
        .iter()
        .map(|id| object(graph, id))
        .collect::<StorageResult<Vec<_>>>()?;
    for object in &objects {
        if crate::recovery_flags::active(graph, object, crate::recovery_flags::CONFLICT, "deleted")?
        {
            return Ok(vec![false, true]);
        }
    }
    let mut values = candidates(graph, &objects, "deleted", |v| v.as_bool())?;
    if values.iter().all(|value| !value.value) {
        return Ok(vec![false]);
    }
    for root in graph.keys(ROOT) {
        if let Some(card) = root.strip_prefix("card:") {
            let (map, assignments) = assignments(graph, &crate::cards::object(graph, card)?)?;
            let mut current = Vec::new();
            for alias in ids.iter().filter(|id| assignments.contains_key(*id)) {
                current.extend(links(graph, &map, alias, ids, None)?);
            }
            for mut link in frontier(graph, current)? {
                if link.value {
                    link.value = false;
                    values.push(link);
                }
            }
        }
    }
    let mut values: Vec<_> = frontier(graph, values)?
        .into_iter()
        .map(|v| v.value)
        .collect();
    values.sort();
    values.dedup();
    Ok(values)
}
