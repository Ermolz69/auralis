CREATE TABLE speech_run_intents_v18 (
    run_id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    plan_json TEXT NOT NULL CHECK (length(plan_json) BETWEEN 1 AND 16777216),
    plan_sha256 TEXT NOT NULL CHECK (length(plan_sha256) = 64),
    profile_id TEXT NOT NULL CHECK (length(profile_id) BETWEEN 1 AND 128),
    profile_sha256 TEXT NOT NULL CHECK (length(profile_sha256) = 64),
    expected_utterances INTEGER NOT NULL CHECK (expected_utterances BETWEEN 1 AND 8192),
    expected_chunks INTEGER NOT NULL CHECK (expected_chunks BETWEEN expected_utterances AND 8192),
    created_at TEXT NOT NULL
);
INSERT INTO speech_run_intents_v18 SELECT * FROM speech_run_intents;
DROP TABLE speech_run_intents;
ALTER TABLE speech_run_intents_v18 RENAME TO speech_run_intents;
CREATE INDEX idx_speech_run_intents_project ON speech_run_intents(project_id, created_at);

CREATE TABLE speech_host_checkpoints_v18 (
    attempt_id TEXT NOT NULL REFERENCES speech_host_attempts(attempt_id) ON DELETE CASCADE,
    sequence INTEGER NOT NULL CHECK (sequence BETWEEN 1 AND 8192),
    utterance_id TEXT NOT NULL,
    cue_id TEXT,
    chunk_id TEXT NOT NULL,
    cache_key TEXT NOT NULL CHECK (length(cache_key) = 64),
    audio_sha256 TEXT NOT NULL CHECK (length(audio_sha256) = 64),
    duration_ms INTEGER NOT NULL CHECK (duration_ms > 0),
    origin TEXT NOT NULL CHECK (origin IN ('generated', 'cached', 'reused_in_job')),
    saved_at TEXT NOT NULL,
    PRIMARY KEY (attempt_id, sequence),
    UNIQUE (attempt_id, chunk_id)
);
INSERT INTO speech_host_checkpoints_v18 SELECT * FROM speech_host_checkpoints;
DROP TABLE speech_host_checkpoints;
ALTER TABLE speech_host_checkpoints_v18 RENAME TO speech_host_checkpoints;
PRAGMA user_version = 18;
