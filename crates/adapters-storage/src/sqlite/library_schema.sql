CREATE TABLE IF NOT EXISTS workspace_library (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    revision INTEGER NOT NULL CHECK (revision >= 1),
    payload TEXT NOT NULL
);
