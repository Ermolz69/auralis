CREATE TABLE translation_edit_intents (
    result_id TEXT PRIMARY KEY NOT NULL,
    translation_id TEXT NOT NULL REFERENCES project_translations(translation_id) ON DELETE CASCADE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    base_result_id TEXT NOT NULL,
    run_id TEXT NOT NULL REFERENCES translation_run_intents(run_id),
    expected_head_result_id TEXT NOT NULL,
    expected_link_revision INTEGER NOT NULL CHECK (expected_link_revision BETWEEN 1 AND 9007199254740990),
    request_sha256 TEXT NOT NULL CHECK (length(request_sha256) = 64),
    created_at TEXT NOT NULL,
    CHECK (result_id != base_result_id AND result_id != expected_head_result_id)
);
CREATE INDEX idx_translation_edit_intents_pending ON translation_edit_intents(created_at, result_id);
PRAGMA user_version = 10;
