//! Causal removals only. Never remove Automerge changes, peer progress, receipts,
//! backup files or grant rows as a side effect of user-visible cleanup.
use crate::{StorageError, WriteContext};
use automerge::{ROOT, transaction::Transactable};
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
            // The root deletions below are CRDT tombstones. Technical causal
            // operations stay available for a future late-edit conflict.
            for card in &preview.cards {
                self.connection().execute(
                    "DELETE FROM card_editing WHERE space = ?1 AND card = ?2",
                    (&preview.space, &card.card),
                )?;
            }
            self.edit(&preview.space, |doc| {
                for card in &preview.cards {
                    doc.delete(ROOT, format!("card:{}", card.card))?;
                }
                for column in &preview.columns {
                    doc.delete(ROOT, format!("column:{}", column.column))?;
                }
                if preview.includes_space {
                    doc.put(ROOT, "lifecycle", "purged")?;
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
                // Address only observed immutable keys: independent inserts
                // and a pending editing series are never included implicitly.
                for (history, version) in targets {
                    doc.delete(history, version)?;
                }
                Ok(())
            })
        })();
        Ok(self.checked(result)?)
    }
}
