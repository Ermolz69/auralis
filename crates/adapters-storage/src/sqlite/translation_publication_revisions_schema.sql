CREATE INDEX idx_translation_publications_run_revision
    ON translation_publications(translation_id, run_id, result_revision);

PRAGMA user_version = 9;
