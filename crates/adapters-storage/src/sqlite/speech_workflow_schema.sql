CREATE TABLE speech_text_reviews (
    decision_id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    snapshot_json TEXT NOT NULL CHECK(length(snapshot_json) BETWEEN 1 AND 16777216),
    created_at TEXT NOT NULL
);
CREATE TABLE speech_publications (
    run_id TEXT PRIMARY KEY NOT NULL REFERENCES speech_run_intents(run_id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES speech_host_attempts(attempt_id),
    publication_json TEXT NOT NULL,
    ready INTEGER NOT NULL DEFAULT 0 CHECK(ready IN (0, 1)),
    created_at TEXT NOT NULL
);
CREATE TABLE speech_run_reviews (
    run_id TEXT PRIMARY KEY NOT NULL REFERENCES speech_run_intents(run_id) ON DELETE CASCADE,
    decision_id TEXT NOT NULL REFERENCES speech_text_reviews(decision_id)
);
PRAGMA user_version = 15;
