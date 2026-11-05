CREATE TABLE access_clock (
    singleton INTEGER PRIMARY KEY NOT NULL CHECK(singleton = 1),
    observed_ms INTEGER NOT NULL CHECK(observed_ms >= 0)
) STRICT, WITHOUT ROWID;
