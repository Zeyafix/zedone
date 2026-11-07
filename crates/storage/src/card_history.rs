//! Local, durable editing lifecycle. Completed versions live only in the shared
//! Automerge history. The pending pair is a journal snapshot, committed with its
//! current text and receipt, never a second editable copy of card content.
use crate::{Storage, StorageError, StorageResult, WriteContext, cards};
use automerge::transaction::Transactable;
use rusqlite::OptionalExtension;
use zedone_core::{
    card_history::{AutosaveCard, SERIES_IDLE_MS},
    cards::CardStore,
    error::{CommandError, ErrorCode},
};

struct Session {
    subject: String,
    session: String,
    pending: Option<String>,
    title: Option<String>,
    description: Option<String>,
    changed_ms: Option<i64>,
}
fn rejected(code: ErrorCode) -> StorageError {
    CommandError { code }.into()
}

impl WriteContext<'_> {
    fn text_session(&self, space: &str, card: &str) -> StorageResult<Option<Session>> {
        Ok(self.connection().query_row(
            "SELECT subject, session, pending_id, title, description, changed_ms FROM card_editing WHERE space = ?1 AND card = ?2",
            (space, card), |r| Ok(Session { subject: r.get(0)?, session: r.get(1)?, pending: r.get(2)?, title: r.get(3)?, description: r.get(4)?, changed_ms: r.get(5)? }),
        ).optional()?)
    }

    fn require_text_session(
        &self,
        space: &str,
        card: &str,
        subject: &str,
        session: &str,
    ) -> StorageResult<Session> {
        self.text_session(space, card)?
            .filter(|s| s.subject == subject && s.session == session)
            .ok_or_else(|| rejected(ErrorCode::StaleState))
    }

    pub(crate) fn finish_pending_text(&mut self, space: &str, card: &str) -> StorageResult<()> {
        let result = (|| {
            let Some(session) = self.text_session(space, card)? else {
                return Ok(());
            };
            let Some(id) = session.pending else {
                return Ok(());
            };
            let title = session.title.ok_or(StorageError::InvalidDatabase)?;
            let description = session.description.ok_or(StorageError::InvalidDatabase)?;
            // Use the saved pair, never reread current text: a later client may
            // already have written different current values through sync.
            self.append_text_version(space, card, &id, &title, &description, false)?;
            self.connection().execute(
                "UPDATE card_editing SET pending_id = NULL, title = NULL, description = NULL, changed_ms = NULL WHERE space = ?1 AND card = ?2",
                (space, card),
            )?;
            Ok(())
        })();
        self.checked(result)
    }

    fn append_text_version(
        &mut self,
        space: &str,
        card: &str,
        id: &str,
        title: &str,
        description: &str,
        replace_text: bool,
    ) -> StorageResult<()> {
        let graph = self.read().document(space)?;
        let object = cards::object(graph, card)?;
        let sequence = cards::next_sequence(graph, card)?;
        let history = cards::map(graph, &object, "history")?;
        self.edit(space, |doc| {
            if replace_text {
                doc.put(&object, "title", title)?;
                doc.put(&object, "description", description)?;
            }
            cards::append_version(doc, &history, id, sequence, title, description)
        })
    }

    pub(crate) fn start_text_session(
        &mut self,
        space: &str,
        card: &str,
        subject: &str,
        session: &str,
    ) -> StorageResult<()> {
        let result = (|| {
            self.finish_pending_text(space, card)?;
            self.connection().execute(
                "INSERT INTO card_editing(space, card, subject, session) VALUES (?1, ?2, ?3, ?4) ON CONFLICT(space, card) DO UPDATE SET subject = excluded.subject, session = excluded.session",
                (space, card, subject, session),
            )?;
            Ok(())
        })();
        self.checked(result)
    }

    pub(crate) fn expire_text_series(
        &mut self,
        space: &str,
        card: &str,
        now_ms: i64,
    ) -> StorageResult<()> {
        let result = (|| {
            if let Some(last) = self.text_session(space, card)?.and_then(|s| s.changed_ms) {
                if now_ms < last {
                    return Err(rejected(ErrorCode::Unavailable));
                }
                if now_ms - last >= SERIES_IDLE_MS {
                    self.finish_pending_text(space, card)?;
                }
            }
            Ok(())
        })();
        self.checked(result)
    }

    pub(crate) fn save_text_series(
        &mut self,
        p: &AutosaveCard,
        subject: &str,
        command: &str,
        now_ms: i64,
    ) -> StorageResult<()> {
        let result = (|| {
            self.require_text_session(&p.space, &p.card, subject, &p.session)?;
            self.expire_text_series(&p.space, &p.card, now_ms)?;
            let before = self.card_snapshot(&p.space, &p.card)?;
            if before.titles == [p.title.as_str()]
                && before.descriptions == [p.description.as_str()]
            {
                return Ok(());
            }
            let session = self.require_text_session(&p.space, &p.card, subject, &p.session)?;
            let id = session
                .pending
                .unwrap_or_else(|| format!("{subject}:{command}"));
            let object = cards::object(self.read().document(&p.space)?, &p.card)?;
            self.edit(&p.space, |doc| {
                doc.put(&object, "title", p.title.as_str())?;
                doc.put(&object, "description", p.description.as_str())?;
                Ok(())
            })?;
            self.connection().execute(
                "UPDATE card_editing SET pending_id = ?3, title = ?4, description = ?5, changed_ms = ?6 WHERE space = ?1 AND card = ?2",
                (&p.space, &p.card, id, &p.title, &p.description, now_ms),
            )?;
            Ok(())
        })();
        self.checked(result)
    }

    pub(crate) fn end_text_session(
        &mut self,
        space: &str,
        card: &str,
        subject: &str,
        session: &str,
    ) -> StorageResult<()> {
        let result = (|| {
            self.require_text_session(space, card, subject, session)?;
            self.finish_pending_text(space, card)?;
            self.connection().execute(
                "DELETE FROM card_editing WHERE space = ?1 AND card = ?2",
                (space, card),
            )?;
            Ok(())
        })();
        self.checked(result)
    }

    pub(crate) fn restore_text_version(
        &mut self,
        space: &str,
        card: &str,
        version: &str,
        subject: &str,
        command: &str,
    ) -> StorageResult<()> {
        let result = (|| {
            let selected = self
                .card_history(space, card)?
                .versions
                .into_iter()
                .find(|v| v.id == version)
                .ok_or_else(|| rejected(ErrorCode::NotFound))?;
            self.finish_pending_text(space, card)?;
            self.append_text_version(
                space,
                card,
                &format!("{subject}:{command}"),
                &selected.title,
                &selected.description,
                true,
            )
        })();
        self.checked(result)
    }

    /// Trusted application-run recovery, not invoked by one-shot CLI readers.
    pub(crate) fn recover_text_sessions(&mut self) -> StorageResult<()> {
        let result = (|| {
            let sessions = self
                .connection()
                .prepare("SELECT space, card FROM card_editing ORDER BY space, card")?
                .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            let mut previous: Option<String> = None;
            for (space, card) in sessions {
                if let Some(old) = previous.as_ref().filter(|old| **old != space) {
                    self.flush_document(old)?;
                }
                self.finish_pending_text(&space, &card)?;
                previous = Some(space);
            }
            if let Some(space) = previous {
                self.flush_document(&space)?;
            }
            self.connection().execute("DELETE FROM card_editing", [])?;
            Ok(())
        })();
        self.checked(result)
    }
}

impl Storage {
    /// Call once before exposing a newly started application host. Completes
    /// durable series and invalidates pre-crash sessions atomically. New tabs
    /// use card.begin_editing for the same boundary on their chosen card.
    pub async fn recover_editing_sessions(&self) -> StorageResult<()> {
        self.transact(|ctx| ctx.recover_text_sessions()).await
    }
}
