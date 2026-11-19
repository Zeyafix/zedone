pub mod channel;
pub mod device_control;
pub mod device_host;
mod exchange;
pub mod http;
pub mod ipc;
pub mod network;
pub mod pairing;
pub mod secrets;

use serde_json::Value;
use zedone_core::{
    access::Credential,
    access_commands::{self, InspectAccess, IssueAccess, ListAccess, ReplaceAccess, RevokeAccess},
    card_history::{AutosaveCard, BeginCardEditing, FinishCardEditing, RestoreCardText},
    cards::{
        self, CreateCard, EditCard, ListCards, ReadCard, ReadCardHistory, ResolveCardDeletion,
        ResolveCardText, RestoreCard, SetCardCompleted, TrashCard,
    },
    cleanup::{self, ClearHistory, InspectHistory, InspectTrash, PurgeTrash},
    column_positions::{self, InspectColumnPositions, ResolveColumnPosition},
    columns::{
        self, CreateColumn, ListColumns, ReadColumn, RenameColumn, ResolveColumnName,
        RestoreColumn, TrashColumn,
    },
    error::{CommandError, ErrorCode},
    invitations::{self, CancelInvitation, CreateInvitation, InspectInvitation, ListInvitations},
    ordering::{self, MoveCard, MoveColumn},
    pairing::{
        self as pending, CancelPairing, ConfirmPairing, InspectPairing, ListPairings,
        RejectPairing, UnlinkPairing,
    },
    placements::{self, InspectCardPlacements, ResolveCardPlacement},
    replay::Execution,
    search::{self, SearchCards},
    spaces::{
        self, CreateSpace, ListSpaces, ReadSpace, RenameSpace, ResolveSpaceName, RestoreSpace,
        TrashSpace,
    },
    tag_assignments::{self, InspectCardTags, ResolveCardTag},
    tag_deletion::{self, ResolveTagDeletion},
    tag_names::{self, ResolveTagNames},
    tags::{self, CreateTag, DeleteTag, ListTags, ReadTag, RenameTag, SetCardTag},
    wire::{CommandResponse, decode_request},
};
use zedone_storage::Storage;

pub const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub const MAX_EXCHANGE_BYTES: usize = 3 * 1024 * 1024;

#[derive(Clone)]
pub struct Service {
    storage: Storage,
}

impl Service {
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
            if body.len() > if operation == "sync.push" { MAX_EXCHANGE_BYTES } else { MAX_REQUEST_BYTES } {
                return Err(invalid());
            }
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
                "sync.inspect" => run!(zedone_core::sync::InspectSync, zedone_core::sync::inspect),
                "sync.select" => run!(zedone_core::sync::SelectSync, zedone_core::sync::select),
                "sync.pause" => run!(zedone_core::sync::PauseSync, zedone_core::sync::pause),
                "sync.catalog" => run!(zedone_core::sync::CatalogSync, zedone_core::sync::catalog),
                "sync.pull" => run!(zedone_core::sync::PullSync, zedone_core::sync::pull),
                "sync.push" => run!(zedone_core::sync::PushSync, zedone_core::sync::push),
                InspectAccess::NAME => run!(InspectAccess, access_commands::inspect),
                RevokeAccess::NAME => run!(RevokeAccess, access_commands::revoke),
                IssueAccess::NAME => run!(IssueAccess, access_commands::issue),
                ReplaceAccess::NAME => run!(ReplaceAccess, access_commands::replace),
                ListAccess::NAME => run!(ListAccess, access_commands::list),
                CreateInvitation::NAME => run!(CreateInvitation, invitations::create),
                InspectInvitation::NAME => run!(InspectInvitation, invitations::inspect),
                CancelInvitation::NAME => run!(CancelInvitation, invitations::cancel),
                ListInvitations::NAME => run!(ListInvitations, invitations::list),
                InspectPairing::NAME => run!(InspectPairing, pending::inspect),
                CancelPairing::NAME => run!(CancelPairing, pending::cancel),
                ListPairings::NAME => run!(ListPairings, pending::list),
                ConfirmPairing::NAME => run!(ConfirmPairing, pending::confirm),
                RejectPairing::NAME => run!(RejectPairing, pending::reject),
                UnlinkPairing::NAME => run!(UnlinkPairing, pending::unlink),
                CreateSpace::NAME => run!(CreateSpace, spaces::create),
                ReadSpace::NAME => run!(ReadSpace, spaces::read),
                ListSpaces::NAME => run!(ListSpaces, spaces::list),
                RenameSpace::NAME => run!(RenameSpace, spaces::rename),
                ResolveSpaceName::NAME => run!(ResolveSpaceName, spaces::resolve_name),
                TrashSpace::NAME => run!(TrashSpace, spaces::trash),
                RestoreSpace::NAME => run!(RestoreSpace, spaces::restore),
                CreateColumn::NAME => run!(CreateColumn, columns::create),
                ReadColumn::NAME => run!(ReadColumn, columns::read),
                ListColumns::NAME => run!(ListColumns, columns::list),
                RenameColumn::NAME => run!(RenameColumn, columns::rename),
                ResolveColumnName::NAME => run!(ResolveColumnName, columns::resolve_name),
                TrashColumn::NAME => run!(TrashColumn, columns::trash),
                RestoreColumn::NAME => run!(RestoreColumn, columns::restore),
                MoveCard::NAME => run!(MoveCard, ordering::move_card),
                MoveColumn::NAME => run!(MoveColumn, ordering::move_column),
                InspectColumnPositions::NAME => run!(InspectColumnPositions, column_positions::inspect),
                ResolveColumnPosition::NAME => run!(ResolveColumnPosition, column_positions::resolve),
                InspectCardPlacements::NAME => run!(InspectCardPlacements, placements::inspect),
                ResolveCardPlacement::NAME => run!(ResolveCardPlacement, placements::resolve),
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
                ResolveCardText::NAME => run!(ResolveCardText, cards::resolve_text),
                ResolveCardDeletion::NAME => run!(ResolveCardDeletion, cards::resolve_deletion),
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
                InspectCardTags::NAME => run!(InspectCardTags, tag_assignments::inspect),
                ResolveCardTag::NAME => run!(ResolveCardTag, tag_assignments::resolve),
                ResolveTagNames::NAME => run!(ResolveTagNames, tag_names::resolve),
                ResolveTagDeletion::NAME => run!(ResolveTagDeletion, tag_deletion::resolve),
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
