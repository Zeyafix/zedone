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
