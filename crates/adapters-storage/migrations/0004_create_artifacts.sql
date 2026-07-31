CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,

    kind TEXT NOT NULL,

    location_kind TEXT NOT NULL,
    location_value TEXT NOT NULL,

    size_bytes INTEGER,
    created_at TEXT NOT NULL,

    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,

    CHECK (location_kind IN ('StorageKey', 'LocalPath'))
);

CREATE INDEX IF NOT EXISTS idx_artifacts_project_id
ON artifacts(project_id);

CREATE INDEX IF NOT EXISTS idx_artifacts_kind
ON artifacts(kind);

CREATE INDEX IF NOT EXISTS idx_artifacts_project_kind
ON artifacts(project_id, kind);

CREATE UNIQUE INDEX IF NOT EXISTS idx_artifacts_location
ON artifacts(location_kind, location_value);
