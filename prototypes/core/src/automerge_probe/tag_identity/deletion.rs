//! Logical deletion retains the causal evidence; it is never a tag trash/restore API.
use super::*;

pub(super) fn lifecycle(doc: &Automerge, tag: &Tag) -> Result<Vec<Candidate>> {
    let mut values = Vec::new();
    for id in &tag.ids {
        values.extend(candidates(doc, id, "life")?);
    }
    for key in ["k0", "k1", "k2"] {
        for mut link in raw_links(doc, &object(doc, key)?, tag)? {
            if link.value == "present" {
                link.value = "alive".to_owned();
                values.push(link);
            }
        }
    }
    let values = frontier(doc, values)?;
    // Ordinary memberships must not invalidate an observation of unrelated names.
    // Keep the causal registers in the document; expose evidence only for deletion.
    Ok(if values.iter().any(|v| v.value == "deleted") {
        values
    } else {
        Vec::new()
    })
}

#[derive(Debug, PartialEq, Eq)]
struct Observed {
    tag: Tag,
    links: Vec<Vec<Candidate>>,
}

fn observe(doc: &Automerge, id: &ObjId) -> Result<Observed> {
    let tag = tag(doc, id)?;
    let links = ["k0", "k1", "k2"]
        .iter()
        .map(|key| raw_links(doc, &object(doc, key)?, &tag))
        .collect::<Result<_>>()?;
    Ok(Observed { tag, links })
}

fn decide(doc: &mut Automerge, id: &ObjId, observed: &Observed, keep: bool) -> Result<()> {
    let current = observe(doc, id)?;
    if current != *observed || current.tag.deleted() {
        return Err("Stale or deleted tag: read the conflict again".into());
    }
    if !current.tag.deletion_conflict() {
        if keep || current.tag.conflicting() {
            return Err(
                "Keep requires a deletion conflict; ordinary deletion needs resolved names".into(),
            );
        }
        for key in ["k0", "k1", "k2"] {
            let card = object(doc, key)?;
            if raw_links(doc, &card, &current.tag)?
                .iter()
                .any(|v| v.value == "present")
                && resolution::card_conflicting(doc, &card)?
            {
                return Err("Ordinary deletion blocked by an affected card conflict".into());
            }
        }
    }
    let context = doc.get_heads();
    let mut tx = doc.transaction();
    put(
        &mut tx,
        id,
        "life",
        if keep { "alive" } else { "deleted" },
        &context,
    )?;
    // One causal decision covers all aliases and assignments, without erasing evidence.
    tx.commit();
    Ok(())
}

mod scenario;
pub(in super::super) use scenario::run;

#[cfg(test)]
mod tests;
