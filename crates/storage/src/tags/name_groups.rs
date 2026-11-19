use super::*;
use std::collections::BTreeSet;

pub(super) fn groups(
    graph: &Automerge,
    groups: &[convergence::Group],
    projections: &BTreeMap<String, Projection>,
) -> StorageResult<BTreeMap<String, Vec<String>>> {
    let mut membership: Vec<BTreeSet<String>> = groups
        .iter()
        .filter(|g| g.deleted != [true])
        .map(|g| BTreeSet::from([g.ids[0].clone()]))
        .collect();
    let mut connect = |ids: BTreeSet<String>| {
        let mut combined = ids;
        membership.retain(|members| {
            if members.is_disjoint(&combined) {
                true
            } else {
                combined.extend(members.iter().cloned());
                false
            }
        });
        membership.push(combined);
    };
    for group in groups {
        if !group.bridge.is_empty() && group.deleted != [true] {
            connect(
                groups
                    .iter()
                    .filter(|g| {
                        g.deleted != [true] && !g.ids.iter().all(|id| !group.bridge.contains(id))
                    })
                    .map(|g| g.ids[0].clone())
                    .collect(),
            );
        }
    }
    for (i, group) in groups
        .iter()
        .enumerate()
        .filter(|(_, g)| g.deleted != [true])
    {
        for other in groups.iter().skip(i + 1).filter(|g| g.deleted != [true]) {
            if group
                .names
                .iter()
                .any(|a| other.names.iter().any(|b| name_key(a) == name_key(b)))
            {
                connect(BTreeSet::from([group.ids[0].clone(), other.ids[0].clone()]));
            }
        }
    }
    let mut names = BTreeMap::<String, Vec<(String, automerge::ChangeHash)>>::new();
    for group in groups.iter().filter(|g| g.deleted != [true]) {
        for id in &group.ids {
            let object = object(graph, id)?.to_string();
            for (op_id, op) in projections.get(&key(id)).ok_or_else(invalid)? {
                if op["obj"].as_str() != Some(&object) || op["key"] != "name" {
                    continue;
                }
                if let Some(name) = op["value"].as_str() {
                    let operation = graph.import_obj(op_id)?;
                    let hash = graph.hash_for_opid(&operation).ok_or_else(invalid)?;
                    names
                        .entry(name_key(name))
                        .or_default()
                        .push((group.ids[0].clone(), hash));
                }
            }
        }
    }
    for candidates in names.values() {
        for (i, (left, a)) in candidates.iter().enumerate() {
            for (right, b) in candidates.iter().skip(i + 1) {
                if left != right
                    && a != b
                    && graph.get_changes_meta(&[*a]).iter().any(|c| c.hash == *b)
                    && graph.get_changes_meta(&[*b]).iter().any(|c| c.hash == *a)
                {
                    connect(BTreeSet::from([left.clone(), right.clone()]));
                }
            }
        }
    }
    Ok(membership
        .into_iter()
        .flat_map(|members| {
            let ids: Vec<_> = members.into_iter().collect();
            ids.iter()
                .map(|id| (id.clone(), ids.clone()))
                .collect::<Vec<_>>()
        })
        .collect())
}
