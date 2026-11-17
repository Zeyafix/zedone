use crate::{StorageError, StorageResult, WriteContext};
#[cfg(test)]
mod tests;
use automerge::{Automerge, Change, ChangeHash, ReadDoc, sync::State};
use zedone_core::{
    command::CommandResult,
    error::{CommandError, ErrorCode},
    spaces::SpaceStore,
    sync::{Heads, PAGE_BYTES, PROTOCOL, PauseSync, SelectSync, SyncControl, SyncPage, SyncStore},
};

fn invalid() -> StorageError {
    CommandError {
        code: ErrorCode::InvalidRequest,
    }
    .into()
}
fn incompatible() -> StorageError {
    CommandError {
        code: ErrorCode::IncompatibleSync,
    }
    .into()
}
fn compatible(graph: &Automerge) -> StorageResult<()> {
    let formats = graph.get_all(automerge::ROOT, "format")?;
    if formats.is_empty() || formats.iter().any(|(v, _)| v.as_str().is_none()) {
        return Err(invalid());
    }
    if formats
        .iter()
        .any(|(v, _)| v.as_str() != Some("zedone.space.v1"))
    {
        return Err(incompatible());
    }
    Ok(())
}
fn hashes(heads: &Heads) -> Vec<ChangeHash> {
    heads.iter().copied().map(ChangeHash).collect()
}
fn heads(graph: &Automerge) -> Heads {
    graph.get_heads().into_iter().map(|h| h.0).collect()
}
pub fn decode_changes(values: &[String]) -> StorageResult<Vec<Change>> {
    if values.len() > 256 || values.iter().map(|v| v.len()).sum::<usize>() > 2 * PAGE_BYTES {
        return Err(invalid());
    }
    values
        .iter()
        .map(|s| {
            if s.len() % 2 != 0
                || !s
                    .bytes()
                    .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
            {
                return Err(invalid());
            }
            let bytes = (0..s.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| invalid()))
                .collect::<StorageResult<Vec<_>>>()?;
            Change::from_bytes(bytes).map_err(|_| invalid())
        })
        .collect()
}
fn encode(change: &Change) -> String {
    change
        .raw_bytes()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn frontier(graph: &Automerge, base: &Heads) -> StorageResult<Automerge> {
    let base = hashes(base);
    if base.iter().any(|h| graph.get_change_by_hash(h).is_none()) {
        return Err(invalid());
    }
    graph.fork_at(&base).map_err(|_| invalid())
}
pub fn page(graph: &Automerge, space: &str, known: &Heads) -> StorageResult<SyncPage> {
    let mut partial = frontier(graph, known)?;
    let changes = graph.get_changes(&hashes(known));
    let total = changes.len();
    let mut size = 0;
    let mut batch = Vec::new();
    for change in changes {
        if change.raw_bytes().len() > PAGE_BYTES {
            return Err(invalid());
        }
        if size + change.raw_bytes().len() > PAGE_BYTES || batch.len() == 256 {
            break;
        }
        size += change.raw_bytes().len();
        partial.apply_changes([change.clone()])?;
        batch.push(encode(&change));
    }
    if !partial.get_missing_deps(&[]).is_empty() {
        return Err(invalid());
    }
    Ok(SyncPage {
        protocol: PROTOCOL,
        space: space.into(),
        base: known.clone(),
        heads: heads(&partial),
        complete: batch.len() == total,
        changes: batch,
    })
}
impl SyncStore for WriteContext<'_> {
    fn sync_control(&mut self, space: &str, pairing: &str) -> CommandResult<SyncControl> {
        let pause = self.read().space_sync_control(space)?;
        let select = self.read().sync_selection(space, pairing)?;
        Ok(SyncControl {
            space: space.into(),
            pairing: pairing.into(),
            selected: select.selected,
            paused: pause.paused,
            selection_version: select.version,
            pause_version: pause.version,
        })
    }
    fn select_sync(&mut self, p: &SelectSync) -> CommandResult<SyncControl> {
        self.set_sync_selected(&p.space, &p.pairing, p.selected, p.expected)?;
        self.sync_control(&p.space, &p.pairing)
    }
    fn pause_sync(&mut self, p: &PauseSync) -> CommandResult<SyncControl> {
        self.set_space_sync_paused(&p.space, p.paused, p.expected)?;
        self.sync_control(&p.space, &p.pairing)
    }
    fn sync_space_exists(&mut self, space: &str) -> CommandResult<bool> {
        Ok(self
            .connection()
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM documents WHERE space = ?1)",
                [space],
                |r| r.get(0),
            )
            .map_err(StorageError::from)?)
    }
    fn pull_sync(&mut self, space: &str, pairing: &str, known: &Heads) -> CommandResult<SyncPage> {
        let result = (|| {
            let result = page(self.read().document(space)?, space, known)?;
            self.set_peer_state(
                space,
                pairing.as_bytes(),
                &State {
                    shared_heads: hashes(known),
                    ..State::new()
                },
            )?;
            Ok(result)
        })();
        Ok(self.checked(result)?)
    }
    fn push_sync(
        &mut self,
        space: &str,
        pairing: &str,
        base: &Heads,
        changes: &[String],
    ) -> CommandResult<SyncPage> {
        let result = (|| {
            let changes = decode_changes(changes)?;
            let mut ack = frontier(self.read().document(space)?, base)?;
            ack.apply_changes(changes.clone())?;
            if !ack.get_missing_deps(&[]).is_empty() {
                return Err(invalid());
            }
            self.accept_sync_changes(space, changes)?;
            let shared = heads(&ack);
            self.set_peer_state(
                space,
                pairing.as_bytes(),
                &State {
                    shared_heads: hashes(&shared),
                    ..State::new()
                },
            )?;
            Ok(SyncPage {
                protocol: PROTOCOL,
                space: space.into(),
                base: base.clone(),
                heads: shared,
                changes: Vec::new(),
                complete: true,
            })
        })();
        Ok(self.checked(result)?)
    }
}
impl WriteContext<'_> {
    pub fn sync_known(&mut self, space: &str, pairing: &str) -> StorageResult<Heads> {
        if self.sync_space_exists(space)? {
            Ok(self
                .read()
                .peer_state(space, pairing.as_bytes())?
                .shared_heads
                .into_iter()
                .map(|h| h.0)
                .collect())
        } else {
            Ok(heads(&self.incoming_graph(space, pairing)?))
        }
    }
    fn incoming_graph(&self, space: &str, pairing: &str) -> StorageResult<Automerge> {
        let bytes = self
            .connection()
            .prepare("SELECT bytes FROM sync_incoming WHERE space = ?1 AND pairing = ?2")?
            .query_map((space, pairing), |r| r.get::<_, Vec<u8>>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let mut graph = Automerge::new();
        graph.apply_changes(
            bytes
                .into_iter()
                .map(Change::from_bytes)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| invalid())?,
        )?;
        Ok(graph)
    }
    fn accept_sync_changes(&mut self, space: &str, changes: Vec<Change>) -> StorageResult<()> {
        let graph = self.read().document(space)?;
        let new = changes
            .iter()
            .any(|c| graph.get_change_by_hash(&c.hash()).is_none());
        if new {
            let before = crate::spaces::domain_projections(graph)?;
            let mut merged = graph.clone();
            merged.apply_changes(changes.clone())?;
            compatible(&merged)?;
            let after = crate::spaces::domain_projections(&merged)?;
            let cards = self
                .connection()
                .prepare(
                    "SELECT card FROM card_editing WHERE space = ?1 AND pending_id IS NOT NULL",
                )?
                .query_map([space], |r| r.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            for card in cards {
                let key = format!("card:{card}");
                if before.get(&key) != after.get(&key) {
                    self.finish_pending_text(space, &card)?;
                }
            }
            self.apply_changes(space, changes)?;
        }
        let snapshot = self.space_snapshot(space)?;
        if snapshot.contents.is_none() {
            return Err(invalid());
        }
        Ok(())
    }
    pub fn receive_sync_page(
        &mut self,
        space: &str,
        pairing: &str,
        subject: &str,
        page: &SyncPage,
    ) -> StorageResult<()> {
        let result = (|| {
            if page.protocol != PROTOCOL {
                return Err(incompatible());
            }
            if page.space != space || self.sync_known(space, pairing)? != page.base {
                return Err(invalid());
            }
            let changes = decode_changes(&page.changes)?;
            let existing = self.sync_space_exists(space)?;
            let mut ack = if existing {
                frontier(self.read().document(space)?, &page.base)?
            } else {
                self.incoming_graph(space, pairing)?
            };
            ack.apply_changes(changes.clone())?;
            if !ack.get_missing_deps(&[]).is_empty() || heads(&ack) != page.heads {
                return Err(invalid());
            }
            if !existing {
                for change in &changes {
                    self.connection().execute(
                        "INSERT INTO sync_incoming VALUES (?1,?2,?3,?4) ON CONFLICT DO NOTHING",
                        (space, pairing, change.hash().as_ref(), change.raw_bytes()),
                    )?;
                }
                if !page.complete {
                    return Ok(());
                }
                compatible(&ack)?;
                self.create_document(space)?;
                self.apply_changes(space, ack.get_changes(&[]))?;
                self.connection().execute(
                    "INSERT INTO access_spaces VALUES (?1,?2,1)",
                    (subject, space),
                )?;
                self.connection().execute(
                    "DELETE FROM sync_incoming WHERE space = ?1 AND pairing = ?2",
                    (space, pairing),
                )?;
            } else {
                self.accept_sync_changes(space, changes)?;
            }
            if self.space_snapshot(space)?.contents.is_none() {
                return Err(invalid());
            }
            self.set_peer_state(
                space,
                pairing.as_bytes(),
                &State {
                    shared_heads: hashes(&page.heads),
                    ..State::new()
                },
            )?;
            Ok(())
        })();
        self.checked(result)
    }
    pub fn outgoing_sync_page(&mut self, space: &str, pairing: &str) -> StorageResult<SyncPage> {
        let known = self.sync_known(space, pairing)?;
        page(self.read().document(space)?, space, &known)
    }
    pub fn acknowledge_sync(
        &mut self,
        space: &str,
        pairing: &str,
        page: &SyncPage,
    ) -> StorageResult<()> {
        let result = (|| {
            if page.protocol != PROTOCOL {
                return Err(incompatible());
            }
            if page.space != space || !page.changes.is_empty() {
                return Err(invalid());
            }
            let graph = self.read().document(space)?;
            frontier(graph, &page.heads)?;
            self.set_peer_state(
                space,
                pairing.as_bytes(),
                &State {
                    shared_heads: hashes(&page.heads),
                    ..State::new()
                },
            )
        })();
        self.checked(result)
    }
    pub fn selected_sync_spaces(&self, pairing: &str) -> StorageResult<Vec<String>> {
        Ok(self.connection().prepare("SELECT space FROM sync_selections WHERE pairing = ?1 AND selected = 1 ORDER BY space")?.query_map([pairing],|r|r.get(0))?.collect::<Result<_,_>>()?)
    }
}
