use std::{path::Path, time::Duration};

use rusqlite::{Connection, OpenFlags, Transaction};

use crate::{StorageError, StorageResult};

const APPLICATION_ID: i64 = 0x5a45444e;

// Append historical layouts here; never edit an existing version in place.
pub(crate) struct Schema {
    pub version: i64,
    pub sql: &'static str,
    pub upgrade: Option<fn(&Transaction<'_>) -> StorageResult<()>>,
    pub archive: ArchiveAdapter,
}
#[derive(Clone, Copy)]
pub(crate) struct ArchiveAdapter {
    pub spaces:
        fn(&Connection, crate::migration_backup::BackupLimits) -> StorageResult<Vec<String>>,
    pub capture: fn(
        &Connection,
        crate::snapshot::SnapshotMetadata,
        crate::snapshot::SnapshotLimits,
    ) -> StorageResult<crate::snapshot::SpaceSnapshot>,
}
pub(crate) const V1_ARCHIVE: ArchiveAdapter = ArchiveAdapter {
    spaces: crate::migration_backup::spaces_v1,
    capture: crate::snapshot::capture,
};
const SCHEMAS: &[Schema] = &[Schema {
    version: 1,
    sql: include_str!("schema/v1.sql"),
    upgrade: None,
    archive: V1_ARCHIVE,
}];
#[derive(Clone, Copy)]
pub(crate) struct Registry(pub &'static [Schema]);
pub(crate) const REGISTRY: Registry = Registry(SCHEMAS);

impl Registry {
    pub fn current(self) -> &'static Schema {
        &self.0[self.0.len() - 1]
    }

    pub fn known(self, connection: &Connection) -> StorageResult<&'static Schema> {
        let (application, version) = identity(connection)?;
        if application != APPLICATION_ID {
            return Err(StorageError::InvalidDatabase);
        }
        self.0
            .iter()
            .find(|schema| schema.version == version)
            .ok_or(StorageError::UnsupportedSchema)
    }

    pub fn check_version(self, connection: &Connection) -> StorageResult<()> {
        if self.known(connection)?.version != self.current().version {
            return Err(StorageError::UnsupportedSchema);
        }
        Ok(())
    }

    pub fn validate(self, connection: &Connection) -> StorageResult<()> {
        check_schema_layout(connection, self.known(connection)?)?;
        check_integrity(connection)
    }

    pub fn target(self, connection: &Connection) -> StorageResult<()> {
        self.check_version(connection)?;
        check_schema_layout(connection, self.current())?;
        check_integrity_with(connection, "PRAGMA integrity_check")
    }

    pub fn migrate(self, tx: &Transaction<'_>) -> StorageResult<()> {
        self.validate(tx)?;
        let source = self.known(tx)?.version;
        for schema in self.0.iter().filter(|s| s.version > source) {
            (schema.upgrade.ok_or(StorageError::UnsupportedSchema)?)(tx)?;
            tx.pragma_update(None, "user_version", schema.version)?;
            // Check each complete transition as well as the final result.
            check_schema_layout(tx, schema)?;
            check_integrity(tx)?;
        }
        self.target(tx)
    }
}

fn identity(connection: &Connection) -> StorageResult<(i64, i64)> {
    Ok((
        connection.pragma_query_value(None, "application_id", |r| r.get(0))?,
        connection.pragma_query_value(None, "user_version", |r| r.get(0))?,
    ))
}

pub(crate) fn check_version(connection: &Connection) -> StorageResult<()> {
    REGISTRY.check_version(connection)
}

pub(crate) fn empty(connection: &Connection) -> StorageResult<bool> {
    if identity(connection)? != (0, 0) {
        return Ok(false);
    }
    let count: i64 =
        connection.query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))?;
    Ok(count == 0)
}

/// No write connection, recovery writes or persistent PRAGMAs before admission.
/// This is a preflight, not a lease: initialize rechecks under its writer lock.
#[cfg(test)]
pub(crate) fn preflight(path: &Path) -> StorageResult<()> {
    preflight_with(path, REGISTRY)
}

pub(crate) fn preflight_with(path: &Path, registry: Registry) -> StorageResult<()> {
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
        registry.validate(&tx)?;
    }
    tx.rollback()?;
    Ok(())
}

#[cfg(test)]
pub(crate) fn initialize(connection: &mut Connection) -> StorageResult<()> {
    crate::startup::initialize(connection, None, REGISTRY, |_| Ok(()))
}

pub(crate) fn create(tx: &Transaction<'_>, registry: Registry) -> StorageResult<()> {
    tx.execute_batch(registry.current().sql)?;
    tx.pragma_update(None, "application_id", APPLICATION_ID)?;
    tx.pragma_update(None, "user_version", registry.current().version)?;
    Ok(())
}

pub(crate) fn configure(connection: &Connection) -> StorageResult<()> {
    // No persistent PRAGMA changes until identity/version/layout/integrity pass.
    let mode: String = connection.query_row("PRAGMA journal_mode = WAL", [], |r| r.get(0))?;
    if mode != "wal" {
        return Err(StorageError::Unavailable);
    }
    connection.pragma_update(None, "synchronous", "FULL")?;
    Ok(())
}

fn check_integrity(connection: &Connection) -> StorageResult<()> {
    check_integrity_with(connection, "PRAGMA quick_check")
}

fn check_integrity_with(connection: &Connection, pragma: &str) -> StorageResult<()> {
    let results = connection
        .prepare(pragma)?
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
// normal product commands. Each version selects its own archive adapter.
pub(crate) fn check_known_layout(connection: &Connection) -> StorageResult<()> {
    check_schema_layout(connection, REGISTRY.known(connection)?)
}

pub(crate) fn check_schema_layout(connection: &Connection, schema: &Schema) -> StorageResult<()> {
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
