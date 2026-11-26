use super::*;

const REVISION: StateVersion = StateVersion([1; 32]);
const CHANGED: StateVersion = StateVersion([2; 32]);

fn moment(local_day: i32, elapsed_ms: u64) -> Moment {
    Moment {
        local_day,
        elapsed_ms,
    }
}

fn start(policy: &mut DailyBackupPolicy, now: Moment, revision: StateVersion) -> Attempt {
    let Admission::Start(attempt) = policy.begin(now, revision, Trigger::Automatic).unwrap() else {
        panic!("expected an eligible attempt");
    };
    attempt
}

fn restart(policy: &DailyBackupPolicy) -> DailyBackupPolicy {
    serde_json::from_slice(&serde_json::to_vec(policy).unwrap()).unwrap()
}

#[test]
fn first_space_and_first_saved_change_are_eligible_without_waiting_for_a_fixed_hour() {
    let mut policy = DailyBackupPolicy::default();
    assert_eq!(
        policy.begin(
            moment(20, 0),
            REVISION,
            Trigger::Retry { failed_sequence: 0 }
        ),
        Ok(Admission::RetryUnavailable)
    );
    let first = start(&mut policy, moment(20, 10), REVISION);
    policy.succeed(first.sequence, moment(20, 20)).unwrap();
    assert_eq!(
        policy.begin(moment(21, 30), REVISION, Trigger::Automatic),
        Ok(Admission::Unchanged)
    );
    assert_eq!(
        policy
            .begin(moment(21, 40), CHANGED, Trigger::Automatic)
            .unwrap(),
        Admission::Start(Attempt {
            sequence: 2,
            revision: CHANGED,
            started: moment(21, 40),
        })
    );
}

#[test]
fn local_midnight_allows_twenty_minutes_but_no_second_success_on_the_same_date() {
    let mut policy = DailyBackupPolicy::default();
    let first = start(&mut policy, moment(100, 0), REVISION);
    policy.succeed(first.sequence, moment(100, 0)).unwrap();
    policy = restart(&policy);
    assert_eq!(
        policy.begin(moment(100, 600_000), CHANGED, Trigger::Automatic),
        Ok(Admission::DailyLimit)
    );
    let second = start(&mut policy, moment(101, 1_200_000), CHANGED);
    policy
        .succeed(second.sequence, moment(101, 1_200_001))
        .unwrap();
    assert_eq!(
        restart(&policy).begin(
            moment(101, 1_300_000),
            REVISION,
            Trigger::Retry { failed_sequence: 1 }
        ),
        Ok(Admission::DailyLimit)
    );
}

#[test]
fn success_uses_completion_date_and_captured_revision_across_midnight() {
    let mut policy = DailyBackupPolicy::default();
    let attempt = start(&mut policy, moment(100, 0), REVISION);
    assert_eq!(
        policy.begin(moment(101, 10), CHANGED, Trigger::Automatic),
        Ok(Admission::Running(attempt))
    );
    let success = policy.succeed(attempt.sequence, moment(101, 20)).unwrap();
    assert_eq!(success.attempt.revision, REVISION);
    assert_eq!(
        policy.begin(moment(101, 30), CHANGED, Trigger::Automatic),
        Ok(Admission::DailyLimit)
    );
    assert_eq!(
        restart(&policy)
            .begin(moment(102, 40), CHANGED, Trigger::Automatic)
            .unwrap(),
        Admission::Start(Attempt {
            sequence: 2,
            revision: CHANGED,
            started: moment(102, 40),
        })
    );
}

#[test]
fn unchanged_days_and_missed_days_do_not_create_catch_up_attempts() {
    let mut policy = DailyBackupPolicy::default();
    let first = start(&mut policy, moment(10, 0), REVISION);
    policy.succeed(first.sequence, moment(10, 1)).unwrap();
    for day in [11, 12, 30, 100] {
        let before = policy.clone();
        assert_eq!(
            policy.begin(moment(day, 100), REVISION, Trigger::Automatic),
            Ok(Admission::Unchanged)
        );
        assert_eq!(policy, before);
    }
    let attempt = start(&mut policy, moment(100, 100), CHANGED);
    assert_eq!(attempt.sequence, 2);
}

#[test]
fn retry_waits_five_minutes_from_failure_completion_and_survives_serialization() {
    let mut policy = DailyBackupPolicy::default();
    let attempt = start(&mut policy, moment(10, 0), REVISION);
    policy.fail(attempt.sequence, moment(10, 30_000)).unwrap();
    policy = restart(&policy);
    for (now, revision, remaining) in [
        (30_000, REVISION, 300_000),
        (329_000, CHANGED, 1000),
        (329_999, CHANGED, 1),
    ] {
        let before = policy.clone();
        assert_eq!(
            policy.begin(moment(10, now), revision, Trigger::Automatic),
            Ok(Admission::Waiting {
                remaining_ms: remaining
            })
        );
        assert_eq!(policy, before);
    }
    assert_eq!(start(&mut policy, moment(10, 330_000), CHANGED).sequence, 2);
    assert!(policy.last_success().is_none());
}

#[test]
fn manual_retry_and_an_old_timer_cannot_bypass_the_new_failure_pause() {
    let mut policy = DailyBackupPolicy::default();
    let first = start(&mut policy, moment(10, 0), REVISION);
    policy.fail(first.sequence, moment(10, 100)).unwrap();
    let retry = Trigger::Retry {
        failed_sequence: first.sequence,
    };
    let Admission::Start(second) = policy.begin(moment(10, 1000), CHANGED, retry).unwrap() else {
        panic!()
    };
    assert_eq!(
        policy.begin(moment(10, 1001), CHANGED, retry),
        Ok(Admission::Running(second))
    );
    assert_eq!(
        policy.begin(moment(10, 1002), CHANGED, Trigger::Automatic),
        Ok(Admission::Running(second))
    );
    policy.fail(second.sequence, moment(10, 2000)).unwrap();
    policy = restart(&policy);
    assert_eq!(
        policy.begin(moment(10, 300_100), CHANGED, retry),
        Ok(Admission::RetryUnavailable)
    );
    assert_eq!(
        policy.begin(moment(10, 300_100), CHANGED, Trigger::Automatic),
        Ok(Admission::Waiting { remaining_ms: 1900 })
    );
    let Admission::Start(third) = policy
        .begin(
            moment(10, 300_101),
            CHANGED,
            Trigger::Retry {
                failed_sequence: second.sequence,
            },
        )
        .unwrap()
    else {
        panic!()
    };
    policy.succeed(third.sequence, moment(10, 300_102)).unwrap();
    assert!(policy.last_failure().is_none());
    assert_eq!(
        policy.begin(moment(10, 300_103), CHANGED, retry),
        Ok(Admission::Unchanged)
    );
    assert_eq!(
        policy.begin(moment(10, 300_104), REVISION, retry),
        Ok(Admission::DailyLimit)
    );
}

#[test]
fn failed_copy_preserves_previous_success_and_does_not_take_the_new_day_limit() {
    let mut policy = DailyBackupPolicy::default();
    let first = start(&mut policy, moment(10, 0), REVISION);
    let previous = policy.succeed(first.sequence, moment(10, 1)).unwrap();
    let second = start(&mut policy, moment(11, 2), CHANGED);
    policy.fail(second.sequence, moment(11, 3)).unwrap();
    assert_eq!(policy.last_success(), Some(previous));
    let third = start(&mut policy, moment(11, 300_003), CHANGED);
    policy.succeed(third.sequence, moment(11, 300_004)).unwrap();
    assert_eq!(policy.last_success().unwrap().attempt.revision, CHANGED);
}

#[test]
fn stale_completion_and_repeated_events_cannot_finish_another_attempt() {
    let mut policy = DailyBackupPolicy::default();
    let first = start(&mut policy, moment(10, 0), REVISION);
    let mut reopened = restart(&policy);
    assert_eq!(
        reopened.begin(moment(10, 1), CHANGED, Trigger::Automatic),
        Ok(Admission::Running(first))
    );
    reopened.fail(first.sequence, moment(10, 2)).unwrap();
    let second = start(&mut reopened, moment(10, 300_002), CHANGED);
    let before = reopened.clone();
    assert_eq!(
        reopened.succeed(first.sequence, moment(10, 300_003)),
        Err(PolicyError::StaleAttempt)
    );
    assert_eq!(
        reopened.fail(first.sequence, moment(10, 300_003)),
        Err(PolicyError::StaleAttempt)
    );
    assert_eq!(reopened, before);
    reopened
        .succeed(second.sequence, moment(10, 300_004))
        .unwrap();
    assert_eq!(
        reopened.fail(second.sequence, moment(10, 300_005)),
        Err(PolicyError::StaleAttempt)
    );
}

#[test]
fn local_date_is_independent_of_elapsed_time_and_calendar_rollback_is_conservative() {
    let mut policy = DailyBackupPolicy::default();
    let first = start(&mut policy, moment(20, 1000), REVISION);
    policy.succeed(first.sequence, moment(20, 1001)).unwrap();
    assert_eq!(
        policy.begin(moment(19, 86_400_000), CHANGED, Trigger::Automatic),
        Ok(Admission::CalendarRollback)
    );
    let attempt = start(&mut policy, moment(21, 86_400_001), CHANGED);
    let before = policy.clone();
    assert_eq!(
        policy.succeed(attempt.sequence, moment(20, 86_400_002)),
        Err(PolicyError::CalendarLimit)
    );
    assert_eq!(policy, before);
    policy
        .fail(attempt.sequence, moment(19, 86_400_002))
        .unwrap();
    assert_eq!(
        policy.begin(moment(1000, 86_400_003), CHANGED, Trigger::Automatic),
        Ok(Admission::Waiting {
            remaining_ms: 299_999
        })
    );
}

#[test]
fn regressing_elapsed_clock_is_rejected_without_mutating_the_record() {
    let mut policy = DailyBackupPolicy::default();
    let attempt = start(&mut policy, moment(10, 100), REVISION);
    let before = policy.clone();
    assert_eq!(
        policy.begin(moment(100, 99), CHANGED, Trigger::Automatic),
        Err(PolicyError::ClockRegression)
    );
    assert_eq!(
        policy.succeed(attempt.sequence, moment(100, 99)),
        Err(PolicyError::ClockRegression)
    );
    assert_eq!(
        policy.fail(attempt.sequence, moment(100, 99)),
        Err(PolicyError::ClockRegression)
    );
    assert_eq!(policy, before);
}

#[test]
fn different_spaces_and_devices_keep_independent_limits() {
    let mut first_device = DailyBackupPolicy::default();
    let attempt = start(&mut first_device, moment(10, 0), REVISION);
    first_device
        .succeed(attempt.sequence, moment(10, 1))
        .unwrap();
    let mut second_device = DailyBackupPolicy::default();
    let mut another_space = DailyBackupPolicy::default();
    assert_eq!(
        start(&mut second_device, moment(10, 1), REVISION).sequence,
        1
    );
    assert_eq!(
        start(&mut another_space, moment(10, 1), REVISION).sequence,
        1
    );
    assert_eq!(
        first_device.begin(moment(10, 2), CHANGED, Trigger::Automatic),
        Ok(Admission::DailyLimit)
    );
}

#[test]
fn invalid_persisted_records_are_rejected_instead_of_resetting_limits() {
    let mut policy = DailyBackupPolicy::default();
    let attempt = start(&mut policy, moment(10, 1), REVISION);
    policy.succeed(attempt.sequence, moment(10, 2)).unwrap();
    let original = serde_json::to_value(&policy).unwrap();
    for (pointer, value) in [
        ("/sequence", serde_json::json!(0)),
        ("/observed_ms", serde_json::json!(1)),
        ("/success/attempt/sequence", serde_json::json!(0)),
        ("/success/attempt/started/elapsed_ms", serde_json::json!(3)),
        (
            "/failure",
            serde_json::json!({"sequence":1,"finished_ms":3}),
        ),
        (
            "/running",
            serde_json::json!({"sequence":2,"revision":vec![0;32],"started":{"local_day":11,"elapsed_ms":1}}),
        ),
    ] {
        let mut corrupt = original.clone();
        *corrupt.pointer_mut(pointer).unwrap() = value;
        assert!(
            serde_json::from_value::<DailyBackupPolicy>(corrupt).is_err(),
            "{pointer}"
        );
    }
    let mut extra = original.clone();
    extra["ignored"] = serde_json::json!(true);
    assert!(serde_json::from_value::<DailyBackupPolicy>(extra).is_err());
    for field in ["sequence", "observed_ms", "success", "failure", "running"] {
        let mut missing = original.clone();
        missing.as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<DailyBackupPolicy>(missing).is_err(),
            "{field}"
        );
    }
}

#[test]
fn time_and_sequence_overflow_do_not_erase_a_running_attempt_or_reuse_a_token() {
    let mut policy = DailyBackupPolicy::default();
    let attempt = start(&mut policy, moment(10, u64::MAX - 1), REVISION);
    let before = policy.clone();
    assert_eq!(
        policy.fail(attempt.sequence, moment(10, u64::MAX)),
        Err(PolicyError::TimeOverflow)
    );
    assert_eq!(policy, before);
    policy
        .succeed(attempt.sequence, moment(10, u64::MAX))
        .unwrap();
    let mut value = serde_json::to_value(&policy).unwrap();
    value["sequence"] = serde_json::json!(u64::MAX);
    value["success"]["attempt"]["sequence"] = serde_json::json!(u64::MAX);
    let mut policy: DailyBackupPolicy = serde_json::from_value(value).unwrap();
    let before = policy.clone();
    assert_eq!(
        policy.begin(moment(11, u64::MAX), CHANGED, Trigger::Automatic),
        Err(PolicyError::CounterExhausted)
    );
    assert_eq!(policy, before);
}

#[test]
fn bounded_event_traces_never_repeat_a_success_date_or_lose_a_captured_revision() {
    fn explore(policy: DailyBackupPolicy, dates: &[i32], step: u64) {
        if step == 6 {
            return;
        }
        for day in [10, 11, 12] {
            for event in 0..4 {
                let mut next = restart(&policy);
                let now = moment(day, step * 300_000);
                let mut dates = dates.to_vec();
                match event {
                    0 | 1 => {
                        let trigger = if event == 0 {
                            Trigger::Automatic
                        } else {
                            Trigger::Retry {
                                failed_sequence: next.last_failure().map_or(0, |f| f.sequence),
                            }
                        };
                        next.begin(
                            now,
                            if step.is_multiple_of(2) {
                                REVISION
                            } else {
                                CHANGED
                            },
                            trigger,
                        )
                        .unwrap();
                    }
                    2 => {
                        if let Some(attempt) = next.running()
                            && let Ok(success) = next.succeed(attempt.sequence, now)
                        {
                            assert!(!dates.contains(&day));
                            assert_eq!(success.attempt.revision, attempt.revision);
                            dates.push(day);
                        }
                    }
                    _ => {
                        if let Some(attempt) = next.running() {
                            next.fail(attempt.sequence, now).unwrap();
                        }
                    }
                }
                explore(next, &dates, step + 1);
            }
        }
    }
    explore(DailyBackupPolicy::default(), &[], 1);
}
