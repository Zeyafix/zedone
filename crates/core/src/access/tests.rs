use std::{
    cell::Cell,
    time::{Duration, UNIX_EPOCH},
};

use super::*;

const TOKEN: Credential<'_> = Credential(b"test-only-credential");
const READ: RequiredAccess<'_> = RequiredAccess::Space {
    id: "A",
    permission: SpacePermission::Read,
};
const WRITE: RequiredAccess<'_> = RequiredAccess::Space {
    id: "A",
    permission: SpacePermission::ReadWrite,
};

struct TestClock(Cell<u64>);
impl Clock for TestClock {
    fn now(&self) -> CommandResult<SystemTime> {
        Ok(UNIX_EPOCH + Duration::from_secs(self.0.get()))
    }
}

#[test]
fn expiry_during_handler_is_rejected_before_its_result_can_escape() {
    let mut session = Session::new(GrantScope::Administration);
    session.access.as_mut().unwrap().grant.expires_at = Some(UNIX_EPOCH + Duration::from_secs(20));
    let clock = TestClock(Cell::new(10));
    let result = with_authorized(
        &mut session,
        &TOKEN,
        &[RequiredAccess::ManageAccess],
        &clock,
        |_, _| {
            clock.0.set(20);
            Ok(())
        },
    );
    assert_eq!(result, Err(denied()));
}

// Only a fixture: no in-memory authority or test credential enters the product.
struct Session {
    access: Option<ResolvedAccess>,
    unavailable: bool,
    resolutions: usize,
    operations: usize,
}

impl AccessTransaction for Session {
    fn resolve_access(
        &mut self,
        credential: &Credential<'_>,
    ) -> CommandResult<Option<ResolvedAccess>> {
        self.resolutions += 1;
        if self.unavailable {
            return Err(CommandError {
                code: ErrorCode::Unavailable,
            });
        }
        Ok((credential.0 == TOKEN.0)
            .then(|| self.access.clone())
            .flatten())
    }
}

impl Session {
    fn new(scope: GrantScope) -> Self {
        Self {
            access: Some(ResolvedAccess {
                subject: "actor".into(),
                grant: Grant {
                    subject: "actor".into(),
                    scope,
                    expires_at: None,
                    revoked: false,
                },
            }),
            unavailable: false,
            resolutions: 0,
            operations: 0,
        }
    }

    fn spaces(permission: SpacePermission) -> Self {
        Self::new(GrantScope::Spaces(BTreeMap::from([(
            "A".into(),
            permission,
        )])))
    }

    fn execute(
        &mut self,
        required: &[RequiredAccess<'_>],
        clock: &impl Clock,
    ) -> CommandResult<String> {
        with_authorized(self, &TOKEN, required, clock, |session, subject| {
            session.operations += 1;
            Ok(subject.to_owned())
        })
    }

    fn grant(&mut self) -> &mut Grant {
        &mut self.access.as_mut().unwrap().grant
    }
}

fn expect_denied(result: CommandResult<String>) {
    assert_eq!(
        result,
        Err(CommandError {
            code: ErrorCode::AccessDenied
        })
    );
}

#[test]
fn data_permissions_and_administration_are_independent() {
    let clock = TestClock(Cell::new(10));
    for (scope, expected) in [
        (
            GrantScope::Spaces(BTreeMap::new()),
            [false, false, false, false],
        ),
        (
            GrantScope::Spaces(BTreeMap::from([("A".into(), SpacePermission::Read)])),
            [true, false, false, false],
        ),
        (
            GrantScope::Spaces(BTreeMap::from([("A".into(), SpacePermission::ReadWrite)])),
            [true, true, false, false],
        ),
        (GrantScope::Administration, [false, false, true, true]),
    ] {
        let mut session = Session::new(scope);
        for (requirement, allowed) in [
            READ,
            WRITE,
            RequiredAccess::ManageDevices,
            RequiredAccess::ManageAccess,
        ]
        .into_iter()
        .zip(expected)
        {
            let result = session.execute(&[requirement], &clock);
            if allowed {
                assert_eq!(result.unwrap(), "actor");
            } else {
                expect_denied(result);
            }
        }
        assert_eq!(session.resolutions, 4);
        assert_eq!(
            session.operations,
            expected.into_iter().filter(|allowed| *allowed).count()
        );
    }
}

#[test]
fn exact_spaces_and_all_targets_are_checked_before_any_effect() {
    let clock = TestClock(Cell::new(10));
    let mut session = Session::spaces(SpacePermission::ReadWrite);
    for id in ["", "B", "a", " A", "*", "A/B"] {
        expect_denied(session.execute(
            &[
                WRITE,
                RequiredAccess::Space {
                    id,
                    permission: SpacePermission::Read,
                },
            ],
            &clock,
        ));
    }
    expect_denied(session.execute(&[], &clock));
    assert_eq!(session.operations, 0);
    // A literal asterisk is never a wildcard. Spaces may have different modes.
    session.grant().scope = GrantScope::Spaces(BTreeMap::from([
        ("A".into(), SpacePermission::ReadWrite),
        ("B".into(), SpacePermission::Read),
        ("*".into(), SpacePermission::Read),
    ]));
    let read_b = RequiredAccess::Space {
        id: "B",
        permission: SpacePermission::Read,
    };
    session.execute(&[WRITE, read_b], &clock).unwrap();
    expect_denied(session.execute(
        &[
            WRITE,
            RequiredAccess::Space {
                id: "B",
                permission: SpacePermission::ReadWrite,
            },
        ],
        &clock,
    ));
    expect_denied(session.execute(
        &[RequiredAccess::Space {
            id: "C",
            permission: SpacePermission::Read,
        }],
        &clock,
    ));
    assert_eq!(session.operations, 1);
}

#[test]
fn missing_invalid_or_empty_credentials_never_run_the_handler() {
    let clock = TestClock(Cell::new(10));
    let mut session = Session::new(GrantScope::Administration);
    for token in [Credential(b""), Credential(b"unknown")] {
        expect_denied(with_authorized(
            &mut session,
            &token,
            &[RequiredAccess::ManageAccess],
            &clock,
            |s, _| {
                s.operations += 1;
                Ok("unexpected".into())
            },
        ));
    }
    session.access = None;
    expect_denied(session.execute(&[READ], &clock));
    expect_denied(session.execute(&[RequiredAccess::ManageDevices], &clock));
    assert_eq!(session.operations, 0);
}

#[test]
fn grant_subject_must_match_authenticated_identity() {
    let clock = TestClock(Cell::new(10));
    let mut session = Session::spaces(SpacePermission::ReadWrite);
    session.grant().subject = "another-actor".into();
    expect_denied(session.execute(&[READ], &clock));
    let access = session.access.as_mut().unwrap();
    access.subject.clear();
    access.grant.subject.clear();
    expect_denied(session.execute(&[WRITE], &clock));
    assert_eq!(session.operations, 0);
}

#[test]
fn expiry_is_exclusive_and_checked_after_waiting() {
    let clock = TestClock(Cell::new(9));
    let mut session = Session::spaces(SpacePermission::ReadWrite);
    session.grant().expires_at = Some(UNIX_EPOCH + Duration::from_secs(10));
    session.execute(&[WRITE], &clock).unwrap();
    // A caller may wait in a queue before entering this transaction.
    clock.0.set(10);
    expect_denied(session.execute(&[READ], &clock));
    clock.0.set(11);
    expect_denied(session.execute(&[WRITE], &clock));
    assert_eq!(session.operations, 1);
    assert_eq!(session.resolutions, 3);
    // Continuing requires an explicit new grant, never a refresh of the request.
    session.access = Session::spaces(SpacePermission::Read).access;
    session.execute(&[READ], &clock).unwrap();
    expect_denied(session.execute(&[WRITE], &clock));
}

#[test]
fn time_is_obtained_after_resolving_current_access() {
    struct DelayedSession<'a> {
        inner: Session,
        clock: &'a TestClock,
    }
    impl AccessTransaction for DelayedSession<'_> {
        fn resolve_access(
            &mut self,
            credential: &Credential<'_>,
        ) -> CommandResult<Option<ResolvedAccess>> {
            let result = self.inner.resolve_access(credential);
            self.clock.0.set(10);
            result
        }
    }
    let clock = TestClock(Cell::new(9));
    let mut inner = Session::spaces(SpacePermission::ReadWrite);
    inner.grant().expires_at = Some(UNIX_EPOCH + Duration::from_secs(10));
    let mut session = DelayedSession {
        inner,
        clock: &clock,
    };
    expect_denied(with_authorized(
        &mut session,
        &TOKEN,
        &[WRITE],
        &clock,
        |_, _| panic!("resolution completed at expiry; the handler must not run"),
    ));
}

#[test]
fn revocation_and_reduction_apply_to_every_new_call_including_replay() {
    let clock = TestClock(Cell::new(10));
    let mut session = Session::spaces(SpacePermission::ReadWrite);
    session.execute(&[WRITE], &clock).unwrap();
    session.grant().scope =
        GrantScope::Spaces(BTreeMap::from([("A".into(), SpacePermission::Read)]));
    expect_denied(session.execute(&[WRITE], &clock));
    session.execute(&[READ], &clock).unwrap();
    session.grant().revoked = true;
    expect_denied(with_authorized(
        &mut session,
        &TOKEN,
        &[READ],
        &clock,
        |_, _| panic!("must not look up or expose the recorded result after revocation"),
    ));
    assert_eq!(session.operations, 2);
    assert_eq!(session.resolutions, 4);
}

#[test]
fn administrative_access_expires_and_can_be_revoked() {
    let clock = TestClock(Cell::new(10));
    let mut session = Session::new(GrantScope::Administration);
    session
        .execute(&[RequiredAccess::ManageAccess], &clock)
        .unwrap();
    session.grant().revoked = true;
    expect_denied(session.execute(&[RequiredAccess::ManageDevices], &clock));
    session = Session::new(GrantScope::Administration);
    session.grant().expires_at = Some(UNIX_EPOCH);
    expect_denied(session.execute(&[RequiredAccess::ManageAccess], &clock));
    assert_eq!(session.operations, 0);
}

#[test]
fn source_and_clock_failures_are_not_permission_or_target_success() {
    struct FailedClock;
    impl Clock for FailedClock {
        fn now(&self) -> CommandResult<SystemTime> {
            Err(CommandError {
                code: ErrorCode::Unavailable,
            })
        }
    }
    let mut session = Session::spaces(SpacePermission::ReadWrite);
    let unavailable = Err(CommandError {
        code: ErrorCode::Unavailable,
    });
    assert_eq!(session.execute(&[READ], &FailedClock), unavailable);
    session.unavailable = true;
    assert_eq!(
        session.execute(&[WRITE], &TestClock(Cell::new(10))),
        unavailable
    );
    assert_eq!(session.operations, 0);
}

#[test]
fn access_is_checked_before_target_existence_and_handler_errors_are_preserved() {
    let clock = TestClock(Cell::new(10));
    let mut session = Session::spaces(SpacePermission::Read);
    let missing = || {
        Err::<String, _>(CommandError {
            code: ErrorCode::NotFound,
        })
    };
    expect_denied(with_authorized(
        &mut session,
        &TOKEN,
        &[WRITE],
        &clock,
        |_, _| missing(),
    ));
    assert_eq!(
        with_authorized(&mut session, &TOKEN, &[READ], &clock, |_, _| missing()),
        missing()
    );
}
