-- Rename the old table
ALTER TABLE artifacts RENAME TO artifacts_old;

-- Create the new table with all constraints
CREATE TABLE artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,

    kind TEXT NOT NULL,

    location_kind TEXT NOT NULL,
    location_value TEXT NOT NULL,

    size_bytes INTEGER,

    state TEXT NOT NULL DEFAULT 'ready',

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    ready_at TEXT,

    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE,

    CHECK (location_kind IN ('StorageKey', 'LocalPath')),
    CHECK (state IN ('pending_finalize', 'ready', 'deleting', 'failed'))
);

-- Copy data from old table
INSERT INTO artifacts (
    id, project_id, kind, location_kind, location_value, size_bytes,
    state, created_at, updated_at, ready_at
)
SELECT 
    id, project_id, kind, location_kind, location_value, size_bytes,
    'ready', created_at, created_at, created_at
FROM artifacts_old;

-- Drop old table
DROP TABLE artifacts_old;

-- Re-create indexes
CREATE INDEX idx_artifacts_project_id ON artifacts(project_id);
CREATE INDEX idx_artifacts_kind ON artifacts(kind);
CREATE INDEX idx_artifacts_project_kind ON artifacts(project_id, kind);
CREATE INDEX idx_artifacts_state ON artifacts(state);
CREATE UNIQUE INDEX idx_artifacts_location ON artifacts(location_kind, location_value);

-- Outbox Messages Table
CREATE TABLE outbox_messages (
    id TEXT PRIMARY KEY NOT NULL,

    kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,

    status TEXT NOT NULL DEFAULT 'pending',
    attempts INTEGER NOT NULL DEFAULT 0,

    next_attempt_at TEXT NOT NULL,
    locked_at TEXT,
    locked_by TEXT,

    last_error TEXT,

    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,

    CHECK (status IN ('pending', 'processing', 'done', 'failed', 'dead'))
);

CREATE INDEX idx_outbox_status_next_attempt
ON outbox_messages(status, next_attempt_at);

CREATE INDEX idx_outbox_kind
ON outbox_messages(kind);
