CREATE TABLE translation_host_jobs (
    job_id TEXT PRIMARY KEY NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    run_id TEXT NOT NULL REFERENCES translation_run_intents(run_id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    ended_at TEXT
);

CREATE UNIQUE INDEX idx_translation_host_jobs_active_run
    ON translation_host_jobs(run_id) WHERE ended_at IS NULL;

CREATE INDEX idx_translation_host_jobs_run_history
    ON translation_host_jobs(run_id, created_at);

PRAGMA user_version = 8;
