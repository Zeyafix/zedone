use crate::{StorageError, WriteContext, cards::deletion::PURGE_BOUNDARY};
use automerge::{ROOT, ReadDoc, transaction::Transactable};
use serde::Serialize;
use zedone_core::{
    cleanup::{CleanupStore, HistoryEntry, TrashPreview},
    command::CommandResult,
    replay::StateVersion,
};

impl CleanupStore for WriteContext<'_> {
    fn cleanup_version(&self, scope: &str, state: &impl Serialize) -> CommandResult<StateVersion> {
        crate::state_version(&scope, state)
    }
    fn purge_objects(&mut self, preview: &TrashPreview) -> CommandResult<()> {
        let result = (|| {
            for card in &preview.cards {
                self.connection().execute(
                    "DELETE FROM card_editing WHERE space = ?1 AND card = ?2",
                    (&preview.space, &card.card),
                )?;
            }
            let graph = self.read().document(&preview.space)?;
            let cards = preview
                .cards
                .iter()
                .map(|card| crate::cards::object(graph, &card.card))
                .collect::<Result<Vec<_>, _>>()?;
            let columns = preview
                .columns
                .iter()
                .map(|column| crate::columns::object(graph, &column.column))
                .collect::<Result<Vec<_>, _>>()?;
            self.edit(&preview.space, |doc| {
                for object in cards
                    .iter()
                    .chain(&columns)
                    .chain(preview.includes_space.then_some(&ROOT))
                {
                    if doc
                        .get_all(object, "lifecycle")?
                        .iter()
                        .any(|(value, _)| value.as_str() == Some("purged"))
                    {
                        doc.delete(object, "lifecycle")?;
                    }
                    doc.put(object, "lifecycle", "purged")?;
                }
                for container in columns
                    .iter()
                    .chain(preview.includes_space.then_some(&ROOT))
                {
                    doc.delete(container, PURGE_BOUNDARY)?;
                    doc.put(container, PURGE_BOUNDARY, true)?;
                }
                Ok(())
            })
        })();
        Ok(self.checked(result)?)
    }
    fn clear_versions(&mut self, space: &str, entries: &[HistoryEntry]) -> CommandResult<()> {
        let result = (|| {
            let graph = self.read().document(space)?;
            let targets = entries
                .iter()
                .filter(|entry| !entry.retained)
                .map(|entry| {
                    let card = crate::cards::object(graph, &entry.card)?;
                    Ok((
                        crate::cards::map(graph, &card, "history")?,
                        entry.version.id.clone(),
                    ))
                })
                .collect::<Result<Vec<_>, StorageError>>()?;
            self.edit(space, |doc| {
                for (history, version) in targets {
                    doc.delete(history, version)?;
                }
                Ok(())
            })
        })();
        Ok(self.checked(result)?)
    }
}
