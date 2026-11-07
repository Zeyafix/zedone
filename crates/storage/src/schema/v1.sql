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
CREATE TABLE access_principals (
    subject TEXT PRIMARY KEY NOT NULL CHECK(length(subject) = 32),
    administration INTEGER NOT NULL CHECK(administration IN (0, 1)),
    expires_ms INTEGER CHECK(expires_ms >= 0),
    revoked INTEGER NOT NULL CHECK(revoked IN (0, 1))
) STRICT, WITHOUT ROWID;
CREATE TABLE access_keys (
    verifier BLOB PRIMARY KEY NOT NULL CHECK(length(verifier) = 32),
    subject TEXT NOT NULL REFERENCES access_principals(subject),
    active INTEGER NOT NULL CHECK(active IN (0, 1))
) STRICT, WITHOUT ROWID;
CREATE UNIQUE INDEX access_current_key ON access_keys(subject) WHERE active = 1;
CREATE TABLE access_spaces (
    subject TEXT NOT NULL REFERENCES access_principals(subject),
    space TEXT NOT NULL REFERENCES documents(space) CHECK(length(space) > 0),
    writable INTEGER NOT NULL CHECK(writable IN (0, 1)),
    PRIMARY KEY(subject, space)
) STRICT, WITHOUT ROWID;
CREATE TABLE access_bootstrap (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
    subject TEXT NOT NULL REFERENCES access_principals(subject)
) STRICT, WITHOUT ROWID;
CREATE TABLE access_clock (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
    observed_ms INTEGER NOT NULL CHECK(observed_ms >= 0)
) STRICT, WITHOUT ROWID;
