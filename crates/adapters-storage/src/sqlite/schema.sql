PRAGMA user_version = 3;

CREATE TABLE IF NOT EXISTS projects (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    source_json TEXT,
    metadata_json TEXT,
    source_language TEXT,
    target_language TEXT,
    transcript_json TEXT,
    active_job_id TEXT,
    last_terminal_job_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1
        CHECK (typeof(revision) = 'integer' AND revision >= 1),
    avatar_data_url TEXT CHECK (avatar_data_url IS NULL OR length(avatar_data_url) <= 1398200)
);

CREATE INDEX IF NOT EXISTS idx_projects_updated_at ON projects(updated_at);
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status);

CREATE TABLE IF NOT EXISTS jobs (
    id TEXT PRIMARY KEY NOT NULL,
    revision INTEGER NOT NULL DEFAULT 1
        CHECK (revision >= 1 AND revision <= 9007199254740991),
    project_id TEXT NOT NULL,
    title TEXT NOT NULL,
    kind TEXT NOT NULL,
    status TEXT NOT NULL,
    stage TEXT,
    progress_json TEXT NOT NULL,
    error_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_jobs_project_id ON jobs(project_id);
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(status);
CREATE INDEX IF NOT EXISTS idx_jobs_created_at ON jobs(created_at);

CREATE TABLE IF NOT EXISTS artifacts (
    id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    location_kind TEXT NOT NULL DEFAULT 'StorageKey'
        CHECK (location_kind = 'StorageKey'),
    location_value TEXT NOT NULL,
    size_bytes INTEGER CHECK (size_bytes IS NULL OR size_bytes >= 0),
    state TEXT NOT NULL DEFAULT 'ready'
        CHECK (state IN ('pending_finalize', 'ready', 'deleting', 'failed')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    ready_at TEXT,
    FOREIGN KEY(project_id) REFERENCES projects(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS idx_artifacts_project_id ON artifacts(project_id);
CREATE INDEX IF NOT EXISTS idx_artifacts_kind ON artifacts(kind);
CREATE INDEX IF NOT EXISTS idx_artifacts_project_kind ON artifacts(project_id, kind);
CREATE INDEX IF NOT EXISTS idx_artifacts_state ON artifacts(state);
CREATE UNIQUE INDEX IF NOT EXISTS idx_artifacts_location
    ON artifacts(location_kind, location_value);

CREATE TABLE IF NOT EXISTS outbox_messages (
    id TEXT PRIMARY KEY NOT NULL,
    kind TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending'
        CHECK (status IN ('pending', 'processing', 'done', 'failed', 'dead')),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts >= 0),
    next_attempt_at TEXT NOT NULL,
    locked_at TEXT,
    locked_by TEXT,
    last_error TEXT,
    deduplication_key TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    aggregate_type TEXT,
    aggregate_id TEXT
);

CREATE INDEX IF NOT EXISTS idx_outbox_status_next_attempt
    ON outbox_messages(status, next_attempt_at);
CREATE INDEX IF NOT EXISTS idx_outbox_kind ON outbox_messages(kind);
CREATE UNIQUE INDEX IF NOT EXISTS idx_outbox_dedup_key
    ON outbox_messages(deduplication_key) WHERE deduplication_key IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_outbox_retention
    ON outbox_messages(status, updated_at, id);
CREATE INDEX IF NOT EXISTS idx_outbox_aggregate_status
    ON outbox_messages(aggregate_type, aggregate_id, status);
