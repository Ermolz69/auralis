CREATE TABLE IF NOT EXISTS media_project_receipts (
    project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    manifest_json TEXT NOT NULL,
    manifest_sha256 TEXT NOT NULL,
    state TEXT NOT NULL CHECK(state IN ('publishing','ready')),
    backup_json TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS media_copy_operations (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_asset_id TEXT NOT NULL,
    source_sha256 TEXT,
    asset_json TEXT NOT NULL,
    staging_key TEXT,
    final_key TEXT,
    state TEXT NOT NULL CHECK(state IN ('copying','pending','ready','cancelled','failed')),
    completed_bytes INTEGER NOT NULL DEFAULT 0,
    total_bytes INTEGER NOT NULL DEFAULT 0,
    error TEXT
);
CREATE TABLE IF NOT EXISTS media_run_sources (
    run_id TEXT PRIMARY KEY REFERENCES speech_run_intents(run_id) ON DELETE CASCADE,
    asset_id TEXT NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE
);
