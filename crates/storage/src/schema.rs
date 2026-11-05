use std::time::Duration;

use rusqlite::{Connection, TransactionBehavior};

use crate::{StorageError, StorageResult};

const APPLICATION_ID: i64 = 0x5a45444e;
const VERSION: i64 = 1;

// Initial schema only. Changing this requires the protected migration path.
const SCHEMA: &str = "
CREATE TABLE documents (
    space TEXT PRIMARY KEY NOT NULL,
    actor BLOB NOT NULL CHECK(length(actor) > 0)
) STRICT, WITHOUT ROWID;
CREATE TABLE changes (
    space TEXT NOT NULL REFERENCES documents(space),
    hash BLOB NOT NULL CHECK(length(hash) = 32),
    bytes BLOB NOT NULL CHECK(length(bytes) > 0),
    PRIMARY KEY(space, hash)
) STRICT, WITHOUT ROWID;
CREATE TABLE peers (
    space TEXT NOT NULL REFERENCES documents(space),
    peer BLOB NOT NULL,
    state BLOB NOT NULL,
    PRIMARY KEY(space, peer)
) STRICT, WITHOUT ROWID;
CREATE TABLE command_results (
    key BLOB PRIMARY KEY NOT NULL CHECK(length(key) > 0),
    request BLOB NOT NULL,
    result BLOB NOT NULL
) STRICT, WITHOUT ROWID;
";

pub(crate) fn check_version(connection: &Connection) -> StorageResult<()> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |r| r.get(0))?;
    let application: i64 = connection.pragma_query_value(None, "application_id", |r| r.get(0))?;
    if application != APPLICATION_ID {
        return Err(StorageError::InvalidDatabase);
    }
    if version != VERSION {
        return Err(StorageError::UnsupportedSchema);
    }
    Ok(())
}

pub(crate) fn initialize(connection: &mut Connection) -> StorageResult<()> {
    connection.busy_timeout(Duration::from_secs(1))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    {
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
        let application: i64 = tx.pragma_query_value(None, "application_id", |r| r.get(0))?;
        let count: i64 = tx.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
        if version == 0 && application == 0 && count == 0 {
            tx.execute_batch(SCHEMA)?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)?;
            tx.pragma_update(None, "user_version", VERSION)?;
        } else {
            check_version(&tx)?;
        }
        check_layout(&tx)?;
        tx.commit()?;
    }
    // No persistent PRAGMA changes until identity/version/layout are accepted.
    let mode: String = connection.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    if mode != "wal" {
        return Err(StorageError::Unavailable);
    }
    connection.pragma_update(None, "synchronous", "FULL")?;
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
    if integrity != "ok" {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}

pub(crate) fn check_layout(connection: &Connection) -> StorageResult<()> {
    check_version(connection)?;
    let mut actual = connection
        .prepare("SELECT sql FROM sqlite_schema WHERE sql IS NOT NULL")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut expected: Vec<_> = SCHEMA
        .split(';')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    actual.sort();
    expected.sort();
    if actual != expected {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}
