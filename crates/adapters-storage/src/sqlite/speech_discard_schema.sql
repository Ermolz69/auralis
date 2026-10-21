CREATE TABLE speech_discarded_artifacts (
    artifact_id TEXT PRIMARY KEY NOT NULL,
    run_id TEXT NOT NULL REFERENCES speech_run_intents(run_id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES speech_host_attempts(attempt_id),
    final_key TEXT NOT NULL,
    created_at TEXT NOT NULL
);
PRAGMA user_version = 16;
