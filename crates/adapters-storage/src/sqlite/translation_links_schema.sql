CREATE TABLE project_translations (
    translation_id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_artifact_id TEXT NOT NULL REFERENCES artifacts(id),
    source_sha256 TEXT NOT NULL CHECK (length(source_sha256) = 64),
    source_format TEXT NOT NULL,
    source_language TEXT NOT NULL,
    target_language TEXT NOT NULL,
    active_run_id TEXT,
    selected_result_id TEXT,
    output_artifact_id TEXT REFERENCES artifacts(id),
    revision INTEGER NOT NULL DEFAULT 1 CHECK (revision BETWEEN 1 AND 9007199254740991),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    CHECK ((selected_result_id IS NULL) = (output_artifact_id IS NULL))
);

CREATE INDEX idx_project_translations_project ON project_translations(project_id);
CREATE INDEX idx_project_translations_active_run
    ON project_translations(active_run_id) WHERE active_run_id IS NOT NULL;

CREATE TABLE translation_publications (
    translation_id TEXT NOT NULL REFERENCES project_translations(translation_id) ON DELETE CASCADE,
    result_id TEXT NOT NULL,
    run_id TEXT NOT NULL,
    artifact_id TEXT NOT NULL UNIQUE REFERENCES artifacts(id),
    source_sha256 TEXT NOT NULL CHECK (length(source_sha256) = 64),
    expected_link_revision INTEGER NOT NULL CHECK (expected_link_revision BETWEEN 1 AND 9007199254740991),
    state TEXT NOT NULL CHECK (state IN ('pending', 'ready', 'failed')),
    needs_review INTEGER NOT NULL CHECK (needs_review IN (0, 1)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    result_revision INTEGER NOT NULL DEFAULT 1 CHECK (result_revision BETWEEN 1 AND 2147483647),
    PRIMARY KEY (translation_id, result_id)
);

CREATE INDEX idx_translation_publications_state ON translation_publications(state);

PRAGMA user_version = 6;
