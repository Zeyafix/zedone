use serde::{Deserialize, Deserializer, Serialize, de::Error};

use crate::replay::StateVersion;

pub const RETRY_DELAY_MS: u64 = 300_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Moment {
    pub local_day: i32,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Attempt {
    pub sequence: u64,
    pub revision: StateVersion,
    pub started: Moment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Success {
    pub attempt: Attempt,
    pub finished: Moment,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Failure {
    pub sequence: u64,
    pub finished_ms: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    Automatic,
    Retry { failed_sequence: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Admission {
    Start(Attempt),
    Running(Attempt),
    Unchanged,
    DailyLimit,
    CalendarRollback,
    Waiting { remaining_ms: u64 },
    RetryUnavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyError {
    InvalidState,
    ClockRegression,
    TimeOverflow,
    CounterExhausted,
    StaleAttempt,
    CalendarLimit,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Record {
    sequence: u64,
    observed_ms: u64,
    #[serde(deserialize_with = "required_option")]
    success: Option<Success>,
    #[serde(deserialize_with = "required_option")]
    failure: Option<Failure>,
    #[serde(deserialize_with = "required_option")]
    running: Option<Attempt>,
}

fn required_option<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    Option::deserialize(deserializer)
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize)]
#[serde(transparent)]
pub struct DailyBackupPolicy(Record);

impl<'de> Deserialize<'de> for DailyBackupPolicy {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let record = Record::deserialize(deserializer)?;
        record
            .validate()
            .map_err(|_| D::Error::custom("invalid daily backup state"))?;
        Ok(Self(record))
    }
}

impl Record {
    fn validate(&self) -> Result<(), PolicyError> {
        let mut sequence = 0;
        let mut finished_ms = 0;
        if let Some(success) = self.success {
            if success.attempt.sequence == 0
                || success.finished.elapsed_ms < success.attempt.started.elapsed_ms
            {
                return Err(PolicyError::InvalidState);
            }
            sequence = success.attempt.sequence;
            finished_ms = success.finished.elapsed_ms;
        }
        if let Some(failure) = self.failure {
            if failure.sequence <= sequence
                || failure.finished_ms < finished_ms
                || failure.finished_ms.checked_add(RETRY_DELAY_MS).is_none()
            {
                return Err(PolicyError::InvalidState);
            }
            sequence = failure.sequence;
            finished_ms = failure.finished_ms;
        }
        if let Some(running) = self.running {
            if running.sequence <= sequence || running.started.elapsed_ms < finished_ms {
                return Err(PolicyError::InvalidState);
            }
            sequence = running.sequence;
            finished_ms = running.started.elapsed_ms;
        }
        if self.sequence != sequence || self.observed_ms != finished_ms {
            return Err(PolicyError::InvalidState);
        }
        Ok(())
    }
}

impl DailyBackupPolicy {
    pub fn last_success(&self) -> Option<Success> {
        self.0.success
    }

    pub fn last_failure(&self) -> Option<Failure> {
        self.0.failure
    }

    pub fn running(&self) -> Option<Attempt> {
        self.0.running
    }

    fn check_time(&self, now: Moment) -> Result<(), PolicyError> {
        if now.elapsed_ms < self.0.observed_ms {
            return Err(PolicyError::ClockRegression);
        }
        Ok(())
    }

    pub fn begin(
        &mut self,
        now: Moment,
        revision: StateVersion,
        trigger: Trigger,
    ) -> Result<Admission, PolicyError> {
        self.check_time(now)?;
        if let Some(attempt) = self.0.running {
            return Ok(Admission::Running(attempt));
        }
        if let Some(success) = self.0.success {
            if success.attempt.revision == revision {
                return Ok(Admission::Unchanged);
            }
            if now.local_day == success.finished.local_day {
                return Ok(Admission::DailyLimit);
            }
            if now.local_day < success.finished.local_day {
                return Ok(Admission::CalendarRollback);
            }
        }
        match (trigger, self.0.failure) {
            (Trigger::Retry { failed_sequence }, Some(failure))
                if failed_sequence == failure.sequence => {}
            (Trigger::Retry { .. }, _) => return Ok(Admission::RetryUnavailable),
            (Trigger::Automatic, Some(failure)) => {
                let elapsed = now.elapsed_ms - failure.finished_ms;
                if elapsed < RETRY_DELAY_MS {
                    return Ok(Admission::Waiting {
                        remaining_ms: RETRY_DELAY_MS - elapsed,
                    });
                }
            }
            (Trigger::Automatic, None) => {}
        }
        let sequence = self
            .0
            .sequence
            .checked_add(1)
            .ok_or(PolicyError::CounterExhausted)?;
        let attempt = Attempt {
            sequence,
            revision,
            started: now,
        };
        self.0.sequence = sequence;
        self.0.observed_ms = now.elapsed_ms;
        self.0.running = Some(attempt);
        Ok(Admission::Start(attempt))
    }

    fn attempt(&self, sequence: u64, now: Moment) -> Result<Attempt, PolicyError> {
        self.check_time(now)?;
        self.0
            .running
            .filter(|attempt| attempt.sequence == sequence)
            .ok_or(PolicyError::StaleAttempt)
    }

    pub fn succeed(&mut self, sequence: u64, now: Moment) -> Result<Success, PolicyError> {
        let attempt = self.attempt(sequence, now)?;
        if self
            .0
            .success
            .is_some_and(|success| now.local_day <= success.finished.local_day)
        {
            return Err(PolicyError::CalendarLimit);
        }
        let success = Success {
            attempt,
            finished: now,
        };
        self.0.success = Some(success);
        self.0.failure = None;
        self.0.running = None;
        self.0.observed_ms = now.elapsed_ms;
        Ok(success)
    }

    pub fn fail(&mut self, sequence: u64, now: Moment) -> Result<Failure, PolicyError> {
        self.attempt(sequence, now)?;
        now.elapsed_ms
            .checked_add(RETRY_DELAY_MS)
            .ok_or(PolicyError::TimeOverflow)?;
        let failure = Failure {
            sequence,
            finished_ms: now.elapsed_ms,
        };
        self.0.failure = Some(failure);
        self.0.running = None;
        self.0.observed_ms = now.elapsed_ms;
        Ok(failure)
    }
}

#[cfg(test)]
mod tests;
