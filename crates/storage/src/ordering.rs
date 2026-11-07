//! Version-5 additive encoding. Legacy scalar ranks/receipts are read unchanged.
use crate::{StorageError, StorageResult, WriteContext};
use automerge::{Automerge, ObjId, ReadDoc, transaction::Transactable};
use zedone_core::{
    command::CommandResult,
    ordering::{CardPlacement, OrderStore, Rank},
};

pub(crate) fn decode_rank(value: automerge::Value<'_>) -> StorageResult<Rank> {
    let rank = if let Some(n) = value.as_u64() {
        Rank::Legacy(n)
    } else {
        serde_json::from_str(value.as_str().ok_or(StorageError::InvalidDatabase)?)
            .map_err(|_| StorageError::InvalidDatabase)?
    };
    if !rank.valid() {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(rank)
}
pub(crate) fn ranks(graph: &Automerge, object: &ObjId) -> StorageResult<Vec<Rank>> {
    let mut ranks = graph
        .get_all(object, "position")?
        .into_iter()
        .map(|(v, _)| decode_rank(v))
        .collect::<StorageResult<Vec<_>>>()?;
    if ranks.is_empty() {
        return Err(StorageError::InvalidDatabase);
    }
    ranks.sort_by_key(|r| r.key(""));
    ranks.dedup();
    Ok(ranks)
}
pub(crate) fn decode_place(value: &str) -> StorageResult<CardPlacement> {
    let place: CardPlacement =
        serde_json::from_str(value).map_err(|_| StorageError::InvalidDatabase)?;
    if !crate::columns::valid_id(&place.column) || !place.rank.valid() {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(place)
}
pub(crate) fn placements(graph: &Automerge, object: &ObjId) -> StorageResult<Vec<CardPlacement>> {
    let mut places = graph
        .get_all(object, "placement")?
        .into_iter()
        .map(|(v, _)| decode_place(v.as_str().ok_or(StorageError::InvalidDatabase)?))
        .collect::<StorageResult<Vec<_>>>()?;
    places.sort_by(|a, b| {
        a.column
            .cmp(&b.column)
            .then(a.rank.key("").cmp(&b.rank.key("")))
    });
    places.dedup();
    Ok(places)
}
pub(crate) fn append(items: impl Iterator<Item = (String, Rank)>, id: &str) -> StorageResult<Rank> {
    let items: Vec<_> = items.collect();
    if items.iter().all(|(_, r)| matches!(r, Rank::Legacy(_))) {
        let max = items
            .iter()
            .filter_map(|(_, r)| {
                if let Rank::Legacy(n) = r {
                    Some(*n)
                } else {
                    None
                }
            })
            .max();
        if let Some(n) = max.map_or(Some(0), |n| n.checked_add(1)) {
            return Ok(Rank::Legacy(n));
        }
    }
    let last = items.into_iter().map(|(id, r)| r.key(&id)).max();
    Ok(Rank::between(last.as_deref(), None, id)?)
}
pub(crate) fn put_rank(
    doc: &mut automerge::transaction::Transaction<'_>,
    object: &ObjId,
    rank: &Rank,
) -> StorageResult<()> {
    match rank {
        Rank::Legacy(n) => doc.put(object, "position", *n)?,
        Rank::Path(_) => doc.put(
            object,
            "position",
            serde_json::to_string(rank).map_err(|_| StorageError::InvalidDatabase)?,
        )?,
    }
    Ok(())
}
impl OrderStore for WriteContext<'_> {
    fn place_card(&mut self, space: &str, card: &str, place: &CardPlacement) -> CommandResult<()> {
        let object = crate::cards::object(self.read().document(space)?, card)?;
        let encoded = serde_json::to_string(place).map_err(|_| StorageError::InvalidDatabase)?;
        // Old column/position remain historical baseline only. Once placement
        // exists it is the sole current authority for BOTH parent and rank.
        Ok(self.edit(space, |doc| {
            doc.put(&object, "placement", encoded)?;
            Ok(())
        })?)
    }
    fn place_column(&mut self, space: &str, column: &str, rank: &Rank) -> CommandResult<()> {
        let object = crate::columns::object(self.read().document(space)?, column)?;
        Ok(self.edit(space, |doc| put_rank(doc, &object, rank))?)
    }
}
