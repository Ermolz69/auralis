CREATE TABLE translation_review_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    translation_id TEXT NOT NULL,
    result_id TEXT NOT NULL,
    result_sha256 TEXT NOT NULL CHECK (length(result_sha256) = 64),
    decision TEXT NOT NULL CHECK (decision IN ('approved', 'withdrawn')),
    reviewer_label TEXT NOT NULL CHECK (length(reviewer_label) BETWEEN 1 AND 120),
    note TEXT NOT NULL CHECK (length(note) <= 2000),
    created_at TEXT NOT NULL,
    FOREIGN KEY (translation_id, result_id)
        REFERENCES translation_publications(translation_id, result_id) ON DELETE CASCADE
);
CREATE INDEX idx_translation_review_result
    ON translation_review_events(project_id, translation_id, result_id, sequence DESC);
CREATE TRIGGER translation_review_events_no_update
BEFORE UPDATE ON translation_review_events
BEGIN
    SELECT RAISE(ABORT, 'translation review history is immutable');
END;
PRAGMA user_version = 11;
