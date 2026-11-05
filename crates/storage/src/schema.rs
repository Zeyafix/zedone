use std::{path::Path, time::Duration};

use rusqlite::{Connection, OpenFlags, TransactionBehavior};

use crate::{StorageError, StorageResult};

const APPLICATION_ID: i64 = 0x5a45444e;

// Append historical layouts here; never edit an existing version in place.
struct Schema {
    version: i64,
    sql: &'static str,
}
const SCHEMAS: &[Schema] = &[Schema {
    version: 1,
    sql: include_str!("schema/v1.sql"),
}];
const CURRENT: &Schema = &SCHEMAS[SCHEMAS.len() - 1];

fn identity(connection: &Connection) -> StorageResult<(i64, i64)> {
    Ok((
        connection.pragma_query_value(None, "application_id", |r| r.get(0))?,
        connection.pragma_query_value(None, "user_version", |r| r.get(0))?,
    ))
}

pub(crate) fn check_version(connection: &Connection) -> StorageResult<()> {
    let schema = known_schema(connection)?;
    if schema.version != CURRENT.version {
        return Err(StorageError::UnsupportedSchema);
    }
    Ok(())
}

fn known_schema(connection: &Connection) -> StorageResult<&'static Schema> {
    let (application, version) = identity(connection)?;
    if application != APPLICATION_ID {
        return Err(StorageError::InvalidDatabase);
    }
    SCHEMAS
        .iter()
        .find(|schema| schema.version == version)
        .ok_or(StorageError::UnsupportedSchema)
}

fn empty(connection: &Connection) -> StorageResult<bool> {
    if identity(connection)? != (0, 0) {
        return Ok(false);
    }
    let count: i64 =
        connection.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
    Ok(count == 0)
}

/// No write connection, recovery writes or persistent PRAGMAs before admission.
/// This is a preflight, not a lease: initialize rechecks under its writer lock.
pub(crate) fn preflight(path: &Path) -> StorageResult<()> {
    match std::fs::metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(_) => return Err(StorageError::Unavailable),
        Ok(_) => {}
    }
    let mut connection = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    connection.busy_timeout(Duration::from_secs(1))?;
    let tx = connection.transaction()?;
    if !empty(&tx)? {
        check_layout(&tx)?;
        check_integrity(&tx)?;
    }
    tx.rollback()?;
    Ok(())
}

pub(crate) fn initialize(connection: &mut Connection) -> StorageResult<()> {
    connection.busy_timeout(Duration::from_secs(1))?;
    connection.pragma_update(None, "foreign_keys", true)?;
    {
        let tx = connection.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if empty(&tx)? {
            tx.execute_batch(CURRENT.sql)?;
            tx.pragma_update(None, "application_id", APPLICATION_ID)?;
            tx.pragma_update(None, "user_version", CURRENT.version)?;
        }
        check_layout(&tx)?;
        check_integrity(&tx)?;
        tx.commit()?;
    }
    // No persistent PRAGMA changes until identity/version/layout/integrity pass.
    let mode: String = connection.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    if mode != "wal" {
        return Err(StorageError::Unavailable);
    }
    connection.pragma_update(None, "synchronous", "FULL")?;
    Ok(())
}

fn check_integrity(connection: &Connection) -> StorageResult<()> {
    let results = connection
        .prepare("PRAGMA quick_check")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    if results != ["ok"]
        || connection
            .prepare("PRAGMA foreign_key_check")?
            .query([])?
            .next()?
            .is_some()
    {
        return Err(StorageError::InvalidDatabase);
    }
    Ok(())
}

pub(crate) fn check_layout(connection: &Connection) -> StorageResult<()> {
    check_version(connection)?;
    check_known_layout(connection)
}

// Archive readers can validate a historical layout without accepting it for
// normal product commands. Schema-specific archive adapters follow in P07.08.2.
pub(crate) fn check_known_layout(connection: &Connection) -> StorageResult<()> {
    let schema = known_schema(connection)?;
    let mut actual = connection
        .prepare("SELECT sql FROM sqlite_schema WHERE sql IS NOT NULL")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut expected: Vec<_> = schema
        .sql
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

#[cfg(test)]
mod tests;
