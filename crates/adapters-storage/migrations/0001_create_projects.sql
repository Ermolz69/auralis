CREATE TABLE projects (
    id TEXT PRIMARY KEY NOT NULL,
    title TEXT NOT NULL,
    status TEXT NOT NULL,
    source_json TEXT,
    metadata_json TEXT,
    source_language TEXT,
    target_language TEXT,
    transcript_json TEXT,
    artifacts_json TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX idx_projects_updated_at ON projects(updated_at);
CREATE INDEX idx_projects_status ON projects(status);
