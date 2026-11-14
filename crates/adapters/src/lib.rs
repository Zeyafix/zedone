//! Transport framing only. All commands use the same Rust decoder, fixed core
//! handlers and timed storage transaction. No callback or authority from JSON.
pub mod http;
pub mod ipc;

use serde_json::Value;
use zedone_core::{
    access::Credential,
    access_commands::{self, InspectAccess, IssueAccess, ListAccess, ReplaceAccess, RevokeAccess},
    card_history::{AutosaveCard, BeginCardEditing, FinishCardEditing, RestoreCardText},
    cards::{
        self, CreateCard, EditCard, ListCards, ReadCard, ReadCardHistory, RestoreCard,
        SetCardCompleted, TrashCard,
    },
    cleanup::{self, ClearHistory, InspectHistory, InspectTrash, PurgeTrash},
    columns::{
        self, CreateColumn, ListColumns, ReadColumn, RenameColumn, RestoreColumn, TrashColumn,
    },
    error::{CommandError, ErrorCode},
    ordering::{self, MoveCard, MoveColumn},
    replay::Execution,
    search::{self, SearchCards},
    spaces::{self, CreateSpace, ListSpaces, ReadSpace, RenameSpace, RestoreSpace, TrashSpace},
    tags::{self, CreateTag, DeleteTag, ListTags, ReadTag, RenameTag, SetCardTag},
    wire::{CommandResponse, decode_request},
};
use zedone_storage::Storage;

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct Service {
    storage: Storage,
}

impl Service {
    /// Trusted host supplies an already admitted Storage; no request can open
    /// another database, migrate it, bootstrap or grant itself rights.
    pub fn new(storage: Storage) -> Self {
        Self { storage }
    }

    pub async fn dispatch(
        &self,
        operation: &str,
        body: &[u8],
        key: &str,
    ) -> CommandResponse<Value> {
        let invalid = || CommandError {
            code: ErrorCode::InvalidRequest,
        };
        let result = async {
            let secret = decode_key(key)?;
            if body.len() > MAX_REQUEST_BYTES {
                return Err(invalid());
            }
            // Every arm fixes both the concrete decoder and the core handler.
            // Neither the request nor a transport supplies policy or a callback.
            macro_rules! run {
                ($command:ty, $handler:path) => {{
                    let request = decode_request::<$command>(body)?;
                    let execution = self
                        .storage
                        .timed_transact(move |ctx, clock| {
                            Ok($handler(ctx, &Credential(&secret), clock, &request)?)
                        })
                        .await
                        .map_err(|error| {
                            #[cfg(debug_assertions)]
                            if std::env::var_os("ZEDONE_CLOCK_DIAGNOSTICS").as_deref()
                                == Some(std::ffi::OsStr::new("1"))
                            {
                                use std::io::Write;
                                // StorageError contains only local categories,
                                // never SQL, paths, credentials or request data.
                                let _ = writeln!(
                                    std::io::stderr().lock(),
                                    "Storage command failed: operation={operation} category={error:?}"
                                );
                            }
                            CommandError::from(error)
                        })?;
                    value(execution)
                }};
            }
            match operation {
                InspectAccess::NAME => run!(InspectAccess, access_commands::inspect),
                RevokeAccess::NAME => run!(RevokeAccess, access_commands::revoke),
                IssueAccess::NAME => run!(IssueAccess, access_commands::issue),
                ReplaceAccess::NAME => run!(ReplaceAccess, access_commands::replace),
                ListAccess::NAME => run!(ListAccess, access_commands::list),
                CreateSpace::NAME => run!(CreateSpace, spaces::create),
                ReadSpace::NAME => run!(ReadSpace, spaces::read),
                ListSpaces::NAME => run!(ListSpaces, spaces::list),
                RenameSpace::NAME => run!(RenameSpace, spaces::rename),
                TrashSpace::NAME => run!(TrashSpace, spaces::trash),
                RestoreSpace::NAME => run!(RestoreSpace, spaces::restore),
                CreateColumn::NAME => run!(CreateColumn, columns::create),
                ReadColumn::NAME => run!(ReadColumn, columns::read),
                ListColumns::NAME => run!(ListColumns, columns::list),
                RenameColumn::NAME => run!(RenameColumn, columns::rename),
                TrashColumn::NAME => run!(TrashColumn, columns::trash),
                RestoreColumn::NAME => run!(RestoreColumn, columns::restore),
                MoveCard::NAME => run!(MoveCard, ordering::move_card),
                MoveColumn::NAME => run!(MoveColumn, ordering::move_column),
                CreateCard::NAME => run!(CreateCard, cards::create),
                ReadCard::NAME => run!(ReadCard, cards::read),
                ReadCardHistory::NAME => run!(ReadCardHistory, cards::history),
                ListCards::NAME => run!(ListCards, cards::list),
                SearchCards::NAME => run!(SearchCards, search::search),
                BeginCardEditing::NAME => run!(BeginCardEditing, cards::begin_editing),
                AutosaveCard::NAME => run!(AutosaveCard, cards::autosave),
                FinishCardEditing::NAME => run!(FinishCardEditing, cards::finish_editing),
                RestoreCardText::NAME => run!(RestoreCardText, cards::restore_text),
                EditCard::NAME => run!(EditCard, cards::edit),
                SetCardCompleted::NAME => run!(SetCardCompleted, cards::set_completed),
                TrashCard::NAME => run!(TrashCard, cards::trash),
                RestoreCard::NAME => run!(RestoreCard, cards::restore),
                InspectTrash::NAME => run!(InspectTrash, cleanup::inspect_trash),
                PurgeTrash::NAME => run!(PurgeTrash, cleanup::purge_trash),
                InspectHistory::NAME => run!(InspectHistory, cleanup::inspect_history),
                ClearHistory::NAME => run!(ClearHistory, cleanup::clear_history),
                CreateTag::NAME => run!(CreateTag, tags::create),
                ReadTag::NAME => run!(ReadTag, tags::read),
                ListTags::NAME => run!(ListTags, tags::list),
                RenameTag::NAME => run!(RenameTag, tags::rename),
                DeleteTag::NAME => run!(DeleteTag, tags::delete),
                SetCardTag::NAME => run!(SetCardTag, tags::set_card_tag),
                _ => Err(invalid()),
            }
        }
        .await;
        result.into()
    }
}

use zedone_core::command::Command;
fn value<T: serde::Serialize>(execution: Execution<T>) -> Result<Execution<Value>, CommandError> {
    Ok(Execution {
        result: serde_json::to_value(execution.result).map_err(|_| CommandError {
            code: ErrorCode::OutcomeUnknown,
        })?,
        replayed: execution.replayed,
    })
}

/// Wire keys are exact lowercase hex. Never Debug/Serialize/log the decoded
/// secret; authentication hashes it inside the authoritative transaction.
fn decode_key(key: &str) -> Result<[u8; 32], CommandError> {
    let denied = || CommandError {
        code: ErrorCode::AccessDenied,
    };
    if key.len() != 64
        || !key
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err(denied());
    }
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&key[index * 2..index * 2 + 2], 16).map_err(|_| denied())?;
    }
    Ok(bytes)
}
