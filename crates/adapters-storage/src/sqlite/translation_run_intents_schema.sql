CREATE TABLE translation_run_intents (
    run_id TEXT PRIMARY KEY NOT NULL,
    translation_id TEXT NOT NULL REFERENCES project_translations(translation_id) ON DELETE CASCADE,
    profile_manifest_json TEXT NOT NULL CHECK (length(profile_manifest_json) BETWEEN 1 AND 65536),
    profile_sha256 TEXT NOT NULL CHECK (length(profile_sha256) = 64),
    max_target_segments INTEGER NOT NULL CHECK (max_target_segments BETWEEN 1 AND 64),
    context_before_segments INTEGER NOT NULL CHECK (context_before_segments BETWEEN 0 AND 8),
    context_after_segments INTEGER NOT NULL CHECK (context_after_segments BETWEEN 0 AND 8),
    parser_version INTEGER NOT NULL CHECK (parser_version > 0),
    policy_fingerprint TEXT NOT NULL CHECK (length(policy_fingerprint) = 64),
    created_at TEXT NOT NULL
);

CREATE INDEX idx_translation_run_intents_translation
    ON translation_run_intents(translation_id, created_at);

PRAGMA user_version = 7;
