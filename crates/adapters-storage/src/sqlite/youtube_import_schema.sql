CREATE TABLE youtube_imports (
    project_id TEXT PRIMARY KEY NOT NULL,
    request_key TEXT UNIQUE NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 1),
    payload_json TEXT NOT NULL
);
PRAGMA user_version = 4;
