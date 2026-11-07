use super::*;
use crate::{Stage, Storage, access::PendingAccess};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tempfile::TempDir;
use zedone_core::{
    access::{Clock, Credential, GrantScope, SpacePermission},
    cards::{self, CardStore, CreateCard, EditCard, RestoreCard, TrashCard},
    columns::{self, CreateColumn},
    command::Command,
    replay::StateVersion,
    spaces::{self, CreateSpace},
    tags::{self as core, DeleteTag, ListTags, ReadTag, RenameTag},
    wire::CommandRequest,
};
struct Now;
impl Clock for Now {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_millis(100))
    }
}
fn sid(id: u8) -> String {
    format!("{id:032x}")
}
fn req<C: Command>(id: u8, parameters: C) -> CommandRequest<C> {
    CommandRequest {
        id: [id; 16],
        operation: C::NAME.into(),
        parameters,
    }
}
macro_rules! run {
    ($s:expr, $a:expr, $handler:path, $p:expr) => {{
        let key = $a.credential().0.to_vec();
        let request = $p;
        $s.transact(move |ctx| Ok($handler(ctx, &Credential(&key), &Now, &request)?))
            .await
    }};
}
mod preservation;
fn create(id: u8, tag: u8, name: &str) -> CommandRequest<CreateTag> {
    req(
        id,
        CreateTag {
            space: sid(1),
            tag: sid(tag),
            name: name.into(),
        },
    )
}
fn set(
    id: u8,
    card: u8,
    tag: u8,
    present: bool,
    expected: StateVersion,
    expected_tag: StateVersion,
) -> CommandRequest<SetCardTag> {
    req(
        id,
        SetCardTag {
            space: sid(1),
            card: sid(card),
            tag: sid(tag),
            present,
            expected,
            expected_tag,
        },
    )
}
fn rename(id: u8, tag: u8, name: &str, expected: StateVersion) -> CommandRequest<RenameTag> {
    req(
        id,
        RenameTag {
            space: sid(1),
            tag: sid(tag),
            name: name.into(),
            expected,
        },
    )
}
fn delete(id: u8, tag: u8, expected: StateVersion) -> CommandRequest<DeleteTag> {
    req(
        id,
        DeleteTag {
            space: sid(1),
            tag: sid(tag),
            expected,
        },
    )
}
fn rejected<T: std::fmt::Debug>(r: StorageResult<T>, code: ErrorCode) {
    assert_eq!(
        r.unwrap_err(),
        StorageError::Rejected(CommandError { code })
    );
}
async fn setup() -> (TempDir, Storage, Arc<PendingAccess>) {
    let dir = TempDir::new().unwrap();
    let s = Storage::open(dir.path().join("tags.db")).await.unwrap();
    let a = Arc::new(PendingAccess::generate().unwrap());
    let b = a.clone();
    s.transact(move |ctx| {
        ctx.bootstrap_access(&b)?;
        spaces::create(
            ctx,
            &b.credential(),
            &Now,
            &req(
                250,
                CreateSpace {
                    space: sid(1),
                    name: "Board".into(),
                },
            ),
        )?;
        columns::create(
            ctx,
            &b.credential(),
            &Now,
            &req(
                251,
                CreateColumn {
                    space: sid(1),
                    column: sid(2),
                    name: "Column".into(),
                },
            ),
        )?;
        for id in [3, 4] {
            cards::create(
                ctx,
                &b.credential(),
                &Now,
                &req(
                    250 + id,
                    CreateCard {
                        space: sid(1),
                        column: sid(2),
                        card: sid(id),
                        title: "Ёж 😀".into(),
                        description: "**raw**".into(),
                    },
                ),
            )?;
        }
        Ok(())
    })
    .await
    .unwrap();
    (dir, s, a)
}
async fn card(s: &Storage, id: u8) -> zedone_core::cards::CardSnapshot {
    s.transact(move |ctx| Ok(ctx.card_snapshot(&sid(1), &sid(id))?))
        .await
        .unwrap()
}
async fn tag(s: &Storage, id: u8) -> TagSnapshot {
    s.transact(move |ctx| Ok(ctx.tag_snapshot(&sid(1), &sid(id))?))
        .await
        .unwrap()
}

#[tokio::test]
async fn assignments_rename_undo_noop_aba_history_and_restart() {
    let (dir, s, a) = setup().await;
    let t = run!(s, a, core::create, create(1, 5, " Работа 😀 "))
        .unwrap()
        .result;
    let initial_tag_version = t.version;
    assert_eq!(t.names, [" Работа 😀 "]);
    let before = card(&s, 3).await;
    let history = s
        .transact(|ctx| Ok(ctx.card_history(&sid(1), &sid(3))?))
        .await
        .unwrap();
    let assigned = run!(
        s,
        a,
        core::set_card_tag,
        set(2, 3, 5, true, before.version, t.version)
    )
    .unwrap()
    .result;
    let mut expected = before.clone();
    expected.tags = vec![sid(5)];
    expected.version = assigned.version;
    assert_eq!(assigned, expected);
    let t = tag(&s, 5).await;
    let noop = run!(
        s,
        a,
        core::set_card_tag,
        set(3, 3, 5, true, assigned.version, t.version)
    )
    .unwrap()
    .result;
    assert_eq!(noop, assigned);
    let renamed = run!(s, a, core::rename, rename(4, 5, "работа", t.version))
        .unwrap()
        .result;
    assert_eq!(renamed.tag, t.tag);
    assert_eq!(card(&s, 3).await.tags, [sid(5)]);
    let renamed = run!(
        s,
        a,
        core::rename,
        rename(5, 5, " Работа 😀 ", renamed.version)
    )
    .unwrap()
    .result;
    assert_ne!(renamed.version, t.version);
    let c = card(&s, 3).await;
    let undone = run!(
        s,
        a,
        core::set_card_tag,
        set(6, 3, 5, false, c.version, renamed.version)
    )
    .unwrap()
    .result;
    assert!(undone.tags.is_empty());
    assert_ne!(undone.version, before.version);
    rejected(
        run!(
            s,
            a,
            core::set_card_tag,
            set(7, 3, 5, true, before.version, renamed.version)
        ),
        ErrorCode::StaleState,
    );
    let replay = run!(
        s,
        a,
        core::set_card_tag,
        set(2, 3, 5, true, before.version, renamed.version)
    )
    .err();
    // A changed original precondition is a different request, never a new add.
    assert_eq!(
        replay,
        Some(StorageError::Rejected(CommandError {
            code: ErrorCode::CommandMismatch
        }))
    );
    let replay = run!(
        s,
        a,
        core::set_card_tag,
        set(2, 3, 5, true, before.version, initial_tag_version)
    )
    .unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.result, assigned);
    assert_eq!(
        s.transact(|ctx| Ok(ctx.card_history(&sid(1), &sid(3))?))
            .await
            .unwrap(),
        history
    );
    let final_tag = tag(&s, 5).await;
    s.shutdown().await.unwrap();
    let s = Storage::open(dir.path().join("tags.db")).await.unwrap();
    assert_eq!(card(&s, 3).await, undone);
    assert_eq!(tag(&s, 5).await, final_tag);
}

#[tokio::test]
async fn uniqueness_uses_space_scope_cyrillic_edges_and_preserves_spelling() {
    let (_dir, s, a) = setup().await;
    run!(s, a, core::create, create(1, 5, "Всё")).unwrap();
    for (id, name) in [(2, " ВСЁ\u{a0}"), (3, "всё")] {
        rejected(
            run!(s, a, core::create, create(id, 6, name)),
            ErrorCode::Conflict,
        );
    }
    run!(s, a, core::create, create(4, 6, "Все")).unwrap();
    let first = tag(&s, 5).await;
    rejected(
        run!(s, a, core::rename, rename(5, 5, " ВСЕ ", first.version)),
        ErrorCode::Conflict,
    );
    assert_eq!(tag(&s, 5).await, first);
    rejected(
        run!(s, a, core::create, create(6, 7, " \t\n\u{2003}")),
        ErrorCode::InvalidRequest,
    );
    let r = run!(
        s,
        a,
        spaces::create,
        req(
            7,
            CreateSpace {
                space: sid(9),
                name: "Other".into()
            }
        )
    )
    .unwrap();
    assert_eq!(r.result.space, sid(9));
    let mut request = create(8, 5, "ВСЁ");
    request.parameters.space = sid(9);
    run!(s, a, core::create, request).unwrap();
    let list = run!(
        s,
        a,
        core::list,
        req(
            9,
            ListTags {
                space: sid(1),
                include_deleted: false
            }
        )
    )
    .unwrap()
    .result;
    assert_eq!(list.tags.len(), 2);
    assert_eq!(first.names, ["Всё"]);
}

#[tokio::test]
async fn deletion_is_atomic_removes_trash_assignments_and_recreation_has_new_identity() {
    let (_dir, s, a) = setup().await;
    run!(s, a, core::create, create(1, 5, "Work")).unwrap();
    for (index, id) in [3, 4].into_iter().enumerate() {
        let c = card(&s, id).await;
        let t = tag(&s, 5).await;
        run!(
            s,
            a,
            core::set_card_tag,
            set(2 + index as u8, id, 5, true, c.version, t.version)
        )
        .unwrap();
    }
    let c = card(&s, 4).await;
    run!(
        s,
        a,
        cards::trash,
        req(
            4,
            TrashCard {
                space: sid(1),
                card: sid(4),
                expected: c.version
            }
        )
    )
    .unwrap();
    let c3 = card(&s, 3).await;
    let c4 = card(&s, 4).await;
    let t = tag(&s, 5).await;
    let deleted = run!(s, a, core::delete, delete(5, 5, t.version))
        .unwrap()
        .result;
    assert_eq!(deleted.state, TagState::Deleted);
    for (id, mut expected) in [(3, c3.clone()), (4, c4.clone())] {
        let after = card(&s, id).await;
        expected.tags.clear();
        expected.version = after.version;
        assert_eq!(after, expected);
    }
    assert!(
        run!(
            s,
            a,
            core::list,
            req(
                6,
                ListTags {
                    space: sid(1),
                    include_deleted: false
                }
            )
        )
        .unwrap()
        .result
        .tags
        .is_empty()
    );
    assert_eq!(
        run!(
            s,
            a,
            core::list,
            req(
                7,
                ListTags {
                    space: sid(1),
                    include_deleted: true
                }
            )
        )
        .unwrap()
        .result
        .tags,
        std::slice::from_ref(&deleted)
    );
    let repeated = run!(s, a, core::delete, delete(5, 5, t.version)).unwrap();
    assert!(repeated.replayed);
    assert_eq!(repeated.result, deleted);
    rejected(
        run!(s, a, core::create, create(8, 5, "Work")),
        ErrorCode::StaleState,
    );
    run!(s, a, core::create, create(9, 6, "Work")).unwrap();
    let c = card(&s, 4).await;
    let restored = run!(
        s,
        a,
        cards::restore,
        req(
            10,
            RestoreCard {
                space: sid(1),
                card: sid(4),
                expected: c.version
            }
        )
    )
    .unwrap()
    .result;
    assert!(restored.tags.is_empty());
    let c = card(&s, 3).await;
    let deleted = tag(&s, 5).await;
    rejected(
        run!(
            s,
            a,
            core::set_card_tag,
            set(11, 3, 5, true, c.version, deleted.version)
        ),
        ErrorCode::InvalidRequest,
    );
}

#[tokio::test]
async fn assignment_changes_invalidate_delete_even_after_aba_but_text_does_not() {
    let (_dir, s, a) = setup().await;
    let initial = run!(s, a, core::create, create(1, 5, "Work"))
        .unwrap()
        .result;
    let c = card(&s, 3).await;
    let assigned = run!(
        s,
        a,
        core::set_card_tag,
        set(2, 3, 5, true, c.version, initial.version)
    )
    .unwrap()
    .result;
    let t = tag(&s, 5).await;
    assert_ne!(initial.version, t.version);
    rejected(
        run!(s, a, core::delete, delete(3, 5, initial.version)),
        ErrorCode::StaleState,
    );
    run!(
        s,
        a,
        cards::edit,
        req(
            4,
            EditCard {
                space: sid(1),
                card: sid(3),
                title: "Edited".into(),
                description: "".into(),
                expected: assigned.version
            }
        )
    )
    .unwrap();
    assert_eq!(tag(&s, 5).await.version, t.version);
    let c = card(&s, 3).await;
    run!(
        s,
        a,
        core::set_card_tag,
        set(5, 3, 5, false, c.version, t.version)
    )
    .unwrap();
    assert_ne!(tag(&s, 5).await.version, initial.version);
    assert_ne!(tag(&s, 5).await.version, t.version);
    rejected(
        run!(s, a, core::delete, delete(7, 5, t.version)),
        ErrorCode::StaleState,
    );
    rejected(
        run!(s, a, core::delete, delete(6, 5, initial.version)),
        ErrorCode::StaleState,
    );
}

#[tokio::test]
async fn failures_roll_back_graph_assignments_and_receipt_and_unknown_reply_replays() {
    for stage in [
        Stage::AfterWork,
        Stage::AfterChanges,
        Stage::AfterResult,
        Stage::BeforeCommit,
        Stage::AfterCommit,
    ] {
        let (_dir, s, a) = setup().await;
        let t = run!(s, a, core::create, create(1, 5, "Work"))
            .unwrap()
            .result;
        let c = card(&s, 3).await;
        run!(
            s,
            a,
            core::set_card_tag,
            set(2, 3, 5, true, c.version, t.version)
        )
        .unwrap();
        let t = tag(&s, 5).await;
        let before = card(&s, 3).await;
        let b = a.clone();
        let request = delete(3, 5, t.version);
        let result = s
            .transact_checked(
                move |ctx| Ok(core::delete(ctx, &b.credential(), &Now, &request)?),
                move |at| {
                    if at == stage {
                        Err(StorageError::Unavailable)
                    } else {
                        Ok(())
                    }
                },
            )
            .await;
        assert_eq!(
            result.unwrap_err(),
            if stage == Stage::AfterCommit {
                StorageError::OutcomeUnknown
            } else {
                StorageError::Unavailable
            }
        );
        if stage != Stage::AfterCommit {
            assert_eq!(card(&s, 3).await, before);
            assert_eq!(tag(&s, 5).await, t);
        }
        let retry = run!(s, a, core::delete, delete(3, 5, t.version)).unwrap();
        assert_eq!(retry.replayed, stage == Stage::AfterCommit);
        assert_eq!(retry.result.state, TagState::Deleted);
        assert!(card(&s, 3).await.tags.is_empty());
    }
}

#[tokio::test]
async fn read_only_other_space_expiry_and_revocation_guard_new_commands_and_replay() {
    let (_dir, s, a) = setup().await;
    let t = run!(s, a, core::create, create(1, 5, "Work"))
        .unwrap()
        .result;
    let read = Arc::new(PendingAccess::generate().unwrap());
    let b = a.clone();
    let r = read.clone();
    s.transact(move |ctx| {
        ctx.issue_access(
            &b.credential(),
            &Now,
            &r,
            &GrantScope::Spaces(BTreeMap::from([(sid(1), SpacePermission::Read)])),
            None,
        )?;
        Ok(())
    })
    .await
    .unwrap();
    run!(
        s,
        read,
        core::read,
        req(
            2,
            ReadTag {
                space: sid(1),
                tag: sid(5)
            }
        )
    )
    .unwrap();
    rejected(
        run!(s, read, core::create, create(3, 6, "New")),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(s, read, core::rename, rename(4, 5, "New", t.version)),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(s, read, core::delete, delete(5, 5, t.version)),
        ErrorCode::AccessDenied,
    );
    let c = card(&s, 3).await;
    rejected(
        run!(
            s,
            read,
            core::set_card_tag,
            set(6, 3, 5, true, c.version, t.version)
        ),
        ErrorCode::AccessDenied,
    );
    rejected(
        run!(
            s,
            read,
            core::list,
            req(
                7,
                ListTags {
                    space: sid(9),
                    include_deleted: true
                }
            )
        ),
        ErrorCode::AccessDenied,
    );
    let b = a.clone();
    let r = read.clone();
    s.transact(move |ctx| {
        ctx.revoke_access(&b.credential(), &Now, r.subject())?;
        Ok(())
    })
    .await
    .unwrap();
    rejected(
        run!(
            s,
            read,
            core::read,
            req(
                2,
                ReadTag {
                    space: sid(1),
                    tag: sid(5)
                }
            )
        ),
        ErrorCode::AccessDenied,
    );
}
