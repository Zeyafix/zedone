//! The clock and command share a writer lock. Failed work rolls back to a
//! savepoint while its calendar and effective time survive in the transaction.
use std::{
    cell::Cell,
    panic::AssertUnwindSafe,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use rusqlite::{Connection, TransactionBehavior};
use zedone_core::{
    access::Clock,
    command::CommandResult,
    error::{CommandError, ErrorCode},
};

use crate::{Storage, StorageError, StorageResult, WriteContext};

#[cfg(test)]
mod tests;

fn unavailable() -> CommandError {
    CommandError {
        code: ErrorCode::Unavailable,
    }
}

pub(crate) fn millis(time: SystemTime) -> CommandResult<i64> {
    i64::try_from(
        time.duration_since(UNIX_EPOCH)
            .map_err(|_| unavailable())?
            .as_millis(),
    )
    .map_err(|_| unavailable())
}

pub(crate) fn seed(tx: &rusqlite::Transaction<'_>) -> StorageResult<()> {
    tx.execute("INSERT INTO access_clock VALUES (1, 0, 0)", [])?;
    Ok(())
}

pub(crate) fn validate(connection: &Connection) -> StorageResult<()> {
    crate::access::validate(connection)?;
    validate_clock(connection)
}

fn validate_clock(connection: &Connection) -> StorageResult<()> {
    let count: i64 = connection.query_row(
        "SELECT count(*) FROM access_clock
         WHERE singleton = 1 AND calendar_ms >= 0 AND effective_ms >= calendar_ms",
        [],
        |r| r.get(0),
    )?;
    if count != 1 {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}

/// Created under the lock from persisted time, never supplied by an adapter.
pub struct TransactionClock<'a> {
    source: &'a dyn Fn() -> CommandResult<SystemTime>,
    timeline: &'a std::sync::Mutex<Option<(Instant, i64)>>,
    calendar: Cell<i64>,
    effective: Cell<i64>,
    deadline: Cell<Option<SystemTime>>,
}

impl Clock for TransactionClock<'_> {
    fn now(&self) -> CommandResult<SystemTime> {
        let wall = millis((self.source)()?)?;
        // Detect actual calendar rollback, not drift below a monotonic projection.
        // The independent effective floor still limits access on a frozen wall.
        if wall < self.calendar.get() {
            #[cfg(debug_assertions)]
            if std::env::var_os("ZEDONE_CLOCK_DIAGNOSTICS").as_deref()
                == Some(std::ffi::OsStr::new("1"))
            {
                use std::io::Write;
                // Explicit debug diagnostics only; never change the clock or
                // turn a failed stderr write into a different command result.
                let _ = writeln!(
                    std::io::stderr().lock(),
                    "Trusted clock rejected calendar: wall_ms={wall} calendar_ms={} effective_ms={}",
                    self.calendar.get(),
                    self.effective.get()
                );
            }
            return Err(unavailable());
        }
        let instant = Instant::now();
        let mut timeline = self.timeline.lock().map_err(|_| unavailable())?;
        let projected = match *timeline {
            Some((start, anchor)) => {
                let elapsed = i64::try_from(
                    instant
                        .checked_duration_since(start)
                        .ok_or_else(unavailable)?
                        .as_millis(),
                )
                .map_err(|_| unavailable())?;
                anchor.checked_add(elapsed).ok_or_else(unavailable)?
            }
            None => wall,
        };
        let now = wall.max(projected).max(self.effective.get());
        if timeline.is_none() || now > projected {
            // Also rebase a persisted floor advanced by another worker. Starting
            // below that floor could freeze effective time until it caught up.
            *timeline = Some((instant, now));
        }
        self.calendar.set(wall);
        self.effective.set(now);
        UNIX_EPOCH
            .checked_add(Duration::from_millis(now as u64))
            .ok_or_else(unavailable)
    }

    fn check_expiry(&self, end: Option<SystemTime>) -> CommandResult<()> {
        if let Some(end) = end {
            self.deadline
                .set(Some(self.deadline.get().map_or(end, |old| old.min(end))));
        }
        let now = self.now()?;
        if self.deadline.get().is_some_and(|end| now >= end) {
            return Err(CommandError {
                code: ErrorCode::AccessDenied,
            });
        }
        Ok(())
    }
}

impl Storage {
    /// The application command entry. No caller-selected clock or callback
    /// from JSON/IPC is accepted. Infrastructure composition supplies the work.
    pub async fn timed_transact<R: Send + 'static>(
        &self,
        work: impl FnOnce(&mut WriteContext<'_>, &TransactionClock<'_>) -> StorageResult<R>
        + Send
        + 'static,
    ) -> StorageResult<R> {
        self.timed_with_source(|| Ok(SystemTime::now()), work).await
    }

    async fn timed_with_source<R: Send + 'static>(
        &self,
        source: impl Fn() -> CommandResult<SystemTime> + Send + 'static,
        work: impl FnOnce(&mut WriteContext<'_>, &TransactionClock<'_>) -> StorageResult<R>
        + Send
        + 'static,
    ) -> StorageResult<R> {
        let registry = self.registry;
        let anchor = self.clock_anchor.clone();
        self.run(move |connection| {
            let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
            registry.check_version(&tx)?;
            let (calendar, effective): (i64, i64) = tx.query_row(
                "SELECT calendar_ms, effective_ms FROM access_clock WHERE singleton = 1",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            let clock = TransactionClock {
                source: &source,
                timeline: &anchor,
                calendar: Cell::new(calendar),
                effective: Cell::new(effective),
                deadline: Cell::new(None),
            };
            // Take an initial reading even for denied or failed work. Malformed
            // framing is rejected by the service before worker admission.
            clock.now()?;
            tx.execute_batch("SAVEPOINT command_work")?;
            let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
                let mut context = WriteContext::new(&tx);
                let result = work(&mut context, &clock)?;
                context.flush()?;
                clock.check_expiry(None)?;
                Ok(result)
            }))
            .unwrap_or(Err(StorageError::OutcomeUnknown));
            if outcome.is_err() {
                tx.execute_batch("ROLLBACK TO command_work")?;
            }
            tx.execute_batch("RELEASE command_work")?;
            tx.execute(
                "UPDATE access_clock SET calendar_ms = max(calendar_ms, ?1),
                    effective_ms = max(effective_ms, ?2) WHERE singleton = 1",
                [clock.calendar.get(), clock.effective.get()],
            )?;
            tx.commit().map_err(|_| StorageError::OutcomeUnknown)?;
            outcome
        })
        .await
    }
}
