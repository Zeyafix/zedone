//! Tag identity and assignment policy, shared by HTTP, CLI and IPC.
use crate::{
    access::{Clock, Credential, RequiredAccess, SpacePermission},
    cards::{CardSnapshot, CardStore},
    command::{Command, CommandResult},
    error::{CommandError, ErrorCode},
    replay::{self, Execution, StateVersion},
    spaces::{SpaceState, valid_id},
    wire::{CommandRequest, CommandResponse},
};
use icu_casemap::CaseMapper;
use icu_normalizer::{ComposingNormalizer, DecomposingNormalizer};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Canonical Unicode caseless comparison, with an NFC key. The intermediate
/// decomposition handles combining Greek ypogegrammeni before case folding.
/// Only the comparison key is normalized; never rewrite the entered spelling.
pub fn name_key(name: &str) -> String {
    let decomposed = DecomposingNormalizer::new_nfd().normalize(name.trim());
    let folded = CaseMapper::new().fold_string(&decomposed);
    ComposingNormalizer::new_nfc()
        .normalize(&folded)
        .into_owned()
}

#[cfg(test)]
mod comparison_tests {
    use super::name_key;
    #[test]
    fn canonical_case_folding_preserves_accents_internal_spaces_and_yo() {
        for (a, b) in [
            ("\u{a0}ВСЁ\u{2003}", "всё"),
            ("Café", "Cafe\u{301}"),
            ("Straße", "STRASSE"),
            ("Σςσ", "σσσ"),
            ("ᾃ", "ἃι"),
            ("Й", "и\u{306}"),
        ] {
            assert_eq!(name_key(a), name_key(b), "{a} / {b}");
        }
        for (a, b) in [
            ("Все", "Всё"),
            ("Cafe", "Café"),
            ("a  b", "a b"),
            ("I", "ı"),
            ("I", "İ"),
            ("Ａ", "A"),
            ("\u{200b}Work", "Work"),
        ] {
            assert_ne!(name_key(a), name_key(b), "{a} / {b}");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum TagState {
    Active,
    Deleted,
    Conflicted,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TagSnapshot {
    pub space: String,
    pub tag: String,
    pub names: Vec<String>,
    pub state: TagState,
    /// Includes causal assignment history, so deletion cannot expand silently.
    pub version: StateVersion,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct TagList {
    pub tags: Vec<TagSnapshot>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct CreateTag {
    pub space: String,
    pub tag: String,
    pub name: String,
}
impl Command for CreateTag {
    const NAME: &'static str = "tag.create";
    type Output = TagSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadTag {
    pub space: String,
    pub tag: String,
}
impl Command for ReadTag {
    const NAME: &'static str = "tag.read";
    type Output = TagSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct ListTags {
    pub space: String,
    pub include_deleted: bool,
}
impl Command for ListTags {
    const NAME: &'static str = "tag.list";
    type Output = TagList;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct RenameTag {
    pub space: String,
    pub tag: String,
    pub name: String,
    pub expected: StateVersion,
}
impl Command for RenameTag {
    const NAME: &'static str = "tag.rename";
    type Output = TagSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct DeleteTag {
    pub space: String,
    pub tag: String,
    pub expected: StateVersion,
}
impl Command for DeleteTag {
    const NAME: &'static str = "tag.delete";
    type Output = TagSnapshot;
}
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
pub struct SetCardTag {
    pub space: String,
    pub card: String,
    pub tag: String,
    pub present: bool,
    pub expected: StateVersion,
    pub expected_tag: StateVersion,
}
impl Command for SetCardTag {
    const NAME: &'static str = "card.set_tag";
    type Output = CardSnapshot;
}

pub trait TagStore: CardStore {
    fn create_tag(&mut self, parameters: &CreateTag) -> CommandResult<TagSnapshot>;
    fn tag_snapshot(&mut self, space: &str, tag: &str) -> CommandResult<TagSnapshot>;
    fn tags(&mut self, space: &str) -> CommandResult<Vec<TagSnapshot>>;
    fn rename_tag(&mut self, space: &str, tag: &str, name: &str) -> CommandResult<()>;
    /// Removes every known assignment, including cards in the trash, atomically.
    fn delete_tag(&mut self, space: &str, tag: &str) -> CommandResult<()>;
    fn set_card_tag(&mut self, parameters: &SetCardTag) -> CommandResult<()>;
}
fn error(code: ErrorCode) -> CommandError {
    CommandError { code }
}
fn validate<C: Command>(
    request: &CommandRequest<C>,
    space: &str,
    tag: Option<&str>,
) -> CommandResult<()> {
    if request.operation != C::NAME || !valid_id(space) || tag.is_some_and(|id| !valid_id(id)) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    Ok(())
}
fn active_space(tx: &mut impl TagStore, space: &str) -> CommandResult<()> {
    match tx.space_snapshot(space)?.state {
        SpaceState::Active => Ok(()),
        SpaceState::Conflicted => Err(error(ErrorCode::Conflict)),
        SpaceState::Trashed => Err(error(ErrorCode::InvalidRequest)),
    }
}
fn active_tag(tag: &TagSnapshot) -> CommandResult<()> {
    match tag.state {
        TagState::Active => Ok(()),
        TagState::Deleted => Err(error(ErrorCode::InvalidRequest)),
        TagState::Conflicted => Err(error(ErrorCode::Conflict)),
    }
}
fn required(space: &str, permission: SpacePermission) -> [RequiredAccess<'_>; 1] {
    [RequiredAccess::Space {
        id: space,
        permission,
    }]
}
fn available(tx: &mut impl TagStore, space: &str, own: &str, name: &str) -> CommandResult<()> {
    if name.trim().is_empty() {
        return Err(error(ErrorCode::InvalidRequest));
    }
    let key = name_key(name);
    if tx.tags(space)?.iter().any(|t| {
        t.tag != own && t.state != TagState::Deleted && t.names.iter().any(|n| name_key(n) == key)
    }) {
        return Err(error(ErrorCode::Conflict));
    }
    Ok(())
}

#[utoipa::path(post, path = "/v1/commands/tag.create", operation_id = "tag_create", request_body = CommandRequest<CreateTag>, responses((status = 200, body = CommandResponse<TagSnapshot>)), security(("access_key" = [])))]
pub fn create(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<CreateTag>,
) -> CommandResult<Execution<TagSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.tag))?;
    if p.name.trim().is_empty() {
        return Err(error(ErrorCode::InvalidRequest));
    }
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| {
            active_space(tx, &p.space)?;
            available(tx, &p.space, &p.tag, &p.name)?;
            tx.create_tag(p)
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/tag.read", operation_id = "tag_read", request_body = CommandRequest<ReadTag>, responses((status = 200, body = CommandResponse<TagSnapshot>)), security(("access_key" = [])))]
pub fn read(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ReadTag>,
) -> CommandResult<Execution<TagSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.tag))?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::Read),
        request.id,
        p,
        |tx| tx.tag_snapshot(&p.space, &p.tag),
    )
}
#[utoipa::path(post, path = "/v1/commands/tag.list", operation_id = "tag_list", request_body = CommandRequest<ListTags>, responses((status = 200, body = CommandResponse<TagList>)), security(("access_key" = [])))]
pub fn list(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<ListTags>,
) -> CommandResult<Execution<TagList>> {
    let p = &request.parameters;
    validate(request, &p.space, None)?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::Read),
        request.id,
        p,
        |tx| {
            let mut tags = tx.tags(&p.space)?;
            tags.retain(|t| p.include_deleted || t.state != TagState::Deleted);
            tags.sort_by(|a, b| {
                a.names
                    .iter()
                    .map(|n| name_key(n))
                    .cmp(b.names.iter().map(|n| name_key(n)))
                    .then(a.tag.cmp(&b.tag))
            });
            Ok(TagList { tags })
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/tag.rename", operation_id = "tag_rename", request_body = CommandRequest<RenameTag>, responses((status = 200, body = CommandResponse<TagSnapshot>)), security(("access_key" = [])))]
pub fn rename(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<RenameTag>,
) -> CommandResult<Execution<TagSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.tag))?;
    if p.name.trim().is_empty() {
        return Err(error(ErrorCode::InvalidRequest));
    }
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| {
            active_space(tx, &p.space)?;
            let before = tx.tag_snapshot(&p.space, &p.tag)?;
            replay::require_current(p.expected, before.version)?;
            active_tag(&before)?;
            available(tx, &p.space, &p.tag, &p.name)?;
            if before.names != [p.name.as_str()] {
                tx.rename_tag(&p.space, &p.tag, &p.name)?;
            }
            tx.tag_snapshot(&p.space, &p.tag)
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/tag.delete", operation_id = "tag_delete", request_body = CommandRequest<DeleteTag>, responses((status = 200, body = CommandResponse<TagSnapshot>)), security(("access_key" = [])))]
pub fn delete(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<DeleteTag>,
) -> CommandResult<Execution<TagSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.tag))?;
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| {
            active_space(tx, &p.space)?;
            let before = tx.tag_snapshot(&p.space, &p.tag)?;
            replay::require_current(p.expected, before.version)?;
            active_tag(&before)?;
            // A conflicted affected card must not be changed as a side effect.
            for card in tx
                .cards(&p.space)?
                .iter()
                .filter(|c| c.tags.contains(&p.tag))
            {
                if card.state == SpaceState::Conflicted {
                    return Err(error(ErrorCode::Conflict));
                }
                for column in &card.columns {
                    if tx.column_snapshot(&p.space, column)?.state == SpaceState::Conflicted {
                        return Err(error(ErrorCode::Conflict));
                    }
                }
            }
            tx.delete_tag(&p.space, &p.tag)?;
            tx.tag_snapshot(&p.space, &p.tag)
        },
    )
}
#[utoipa::path(post, path = "/v1/commands/card.set_tag", operation_id = "card_set_tag", request_body = CommandRequest<SetCardTag>, responses((status = 200, body = CommandResponse<CardSnapshot>)), security(("access_key" = [])))]
pub fn set_card_tag(
    tx: &mut impl TagStore,
    credential: &Credential<'_>,
    clock: &impl Clock,
    request: &CommandRequest<SetCardTag>,
) -> CommandResult<Execution<CardSnapshot>> {
    let p = &request.parameters;
    validate(request, &p.space, Some(&p.tag))?;
    if !valid_id(&p.card) {
        return Err(error(ErrorCode::InvalidRequest));
    }
    replay::execute(
        tx,
        credential,
        clock,
        &required(&p.space, SpacePermission::ReadWrite),
        request.id,
        p,
        |tx| {
            let card = tx.card_snapshot(&p.space, &p.card)?;
            replay::require_current(p.expected, card.version)?;
            if card.state == SpaceState::Conflicted {
                return Err(error(ErrorCode::Conflict));
            }
            if card.state != SpaceState::Active {
                return Err(error(ErrorCode::InvalidRequest));
            }
            crate::cards::parents(tx, &p.space, &card.columns[0])?;
            let tag = tx.tag_snapshot(&p.space, &p.tag)?;
            replay::require_current(p.expected_tag, tag.version)?;
            active_tag(&tag)?;
            if card.tags.contains(&p.tag) != p.present {
                tx.set_card_tag(p)?;
            }
            tx.card_snapshot(&p.space, &p.card)
        },
    )
}
