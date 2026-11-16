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
CREATE TABLE sync_spaces (
    space TEXT PRIMARY KEY NOT NULL CHECK(length(space) > 0),
    paused INTEGER NOT NULL CHECK(paused IN (0, 1)),
    revision INTEGER NOT NULL CHECK(revision > 0)
) STRICT, WITHOUT ROWID;
CREATE TABLE sync_selections (
    space TEXT NOT NULL CHECK(length(space) > 0),
    pairing TEXT NOT NULL CHECK(length(pairing) = 32 AND pairing NOT GLOB '*[^0-9a-f]*' AND pairing != '00000000000000000000000000000000'),
    selected INTEGER NOT NULL CHECK(selected IN (0, 1)),
    revision INTEGER NOT NULL CHECK(revision > 0),
    PRIMARY KEY(space, pairing)
) STRICT, WITHOUT ROWID;
CREATE TABLE sync_incoming (
    space TEXT NOT NULL,
    pairing TEXT NOT NULL,
    hash BLOB NOT NULL CHECK(length(hash) = 32),
    bytes BLOB NOT NULL CHECK(length(bytes) > 0),
    PRIMARY KEY(space, pairing, hash),
    FOREIGN KEY(space, pairing) REFERENCES sync_selections(space, pairing)
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
    calendar_ms INTEGER NOT NULL CHECK(calendar_ms >= 0),
    effective_ms INTEGER NOT NULL CHECK(effective_ms >= calendar_ms)
) STRICT, WITHOUT ROWID;
CREATE TABLE card_editing (
    space TEXT NOT NULL REFERENCES documents(space),
    card TEXT NOT NULL CHECK(length(card) = 32),
    subject TEXT NOT NULL REFERENCES access_principals(subject),
    session TEXT NOT NULL CHECK(length(session) = 32),
    pending_id TEXT,
    title TEXT,
    description TEXT,
    changed_ms INTEGER CHECK(changed_ms >= 0),
    CHECK((pending_id IS NULL AND title IS NULL AND description IS NULL AND changed_ms IS NULL)
       OR (pending_id IS NOT NULL AND title IS NOT NULL AND length(trim(title)) > 0 AND description IS NOT NULL AND changed_ms IS NOT NULL)),
    PRIMARY KEY(space, card)
) STRICT, WITHOUT ROWID;
CREATE TABLE pairing_invitations (
    invitation TEXT PRIMARY KEY NOT NULL CHECK(length(invitation) = 32 AND invitation NOT GLOB '*[^0-9a-f]*'),
    verifier BLOB NOT NULL UNIQUE CHECK(length(verifier) = 32 AND verifier != zeroblob(32)),
    created_ms INTEGER NOT NULL CHECK(created_ms >= 0 AND created_ms <= 253402300199999),
    expires_ms INTEGER NOT NULL CHECK(expires_ms = created_ms + 600000),
    cancelled INTEGER NOT NULL CHECK(cancelled IN (0, 1)),
    failures INTEGER NOT NULL CHECK(failures BETWEEN 0 AND 5)
) STRICT, WITHOUT ROWID;
CREATE TABLE pairing_requests (
    request TEXT PRIMARY KEY NOT NULL CHECK(length(request) = 32 AND request NOT GLOB '*[^0-9a-f]*'),
    invitation TEXT NOT NULL UNIQUE REFERENCES pairing_invitations(invitation),
    pending_verifier BLOB NOT NULL UNIQUE CHECK(length(pending_verifier) = 32 AND pending_verifier != zeroblob(32)),
    access_verifier BLOB NOT NULL UNIQUE CHECK(length(access_verifier) = 32 AND access_verifier != zeroblob(32)),
    label TEXT NOT NULL CHECK(length(label) BETWEEN 1 AND 80),
    created_ms INTEGER NOT NULL CHECK(created_ms BETWEEN 0 AND 253402300799999),
    cancelled INTEGER NOT NULL CHECK(cancelled IN (0, 1)),
    decision INTEGER NOT NULL CHECK(decision IN (0, 1, 2)),
    subject TEXT REFERENCES access_principals(subject),
    CHECK((decision = 1 AND subject IS NOT NULL AND subject = request AND cancelled = 0)
       OR (decision IN (0, 2) AND subject IS NULL))
) STRICT, WITHOUT ROWID;
