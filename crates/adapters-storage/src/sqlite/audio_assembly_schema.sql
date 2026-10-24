CREATE TABLE audio_assembly_intents (
    assembly_id TEXT PRIMARY KEY NOT NULL,
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    source_run_id TEXT NOT NULL REFERENCES speech_run_intents(run_id),
    source_attempt_id TEXT NOT NULL REFERENCES speech_host_attempts(attempt_id),
    plan_json TEXT NOT NULL CHECK(length(plan_json) BETWEEN 1 AND 16777216),
    plan_sha256 TEXT NOT NULL CHECK(length(plan_sha256)=64),
    created_at TEXT NOT NULL,
    UNIQUE(project_id, plan_sha256)
);
CREATE INDEX audio_assembly_project_history ON audio_assembly_intents(project_id, created_at DESC);
CREATE TABLE audio_assembly_attempts (
    attempt_id TEXT PRIMARY KEY NOT NULL,
    assembly_id TEXT NOT NULL REFERENCES audio_assembly_intents(assembly_id) ON DELETE CASCADE,
    job_id TEXT NOT NULL UNIQUE REFERENCES jobs(id) ON DELETE CASCADE,
    created_at TEXT NOT NULL,
    ended_at TEXT
);
CREATE UNIQUE INDEX audio_assembly_active_attempt ON audio_assembly_attempts(assembly_id) WHERE ended_at IS NULL;
CREATE TABLE audio_assembly_publications (
    assembly_id TEXT PRIMARY KEY NOT NULL REFERENCES audio_assembly_intents(assembly_id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES audio_assembly_attempts(attempt_id),
    publication_json TEXT NOT NULL CHECK(length(publication_json) BETWEEN 1 AND 65536),
    ready INTEGER NOT NULL DEFAULT 0 CHECK(ready IN (0,1)),
    created_at TEXT NOT NULL
);
CREATE TABLE audio_assembly_discarded_artifacts (
    artifact_id TEXT PRIMARY KEY NOT NULL,
    assembly_id TEXT NOT NULL REFERENCES audio_assembly_intents(assembly_id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES audio_assembly_attempts(attempt_id),
    final_key TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE TABLE audio_assembly_artifacts (
    artifact_id TEXT PRIMARY KEY NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    assembly_id TEXT NOT NULL REFERENCES audio_assembly_intents(assembly_id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES audio_assembly_attempts(attempt_id),
    role TEXT NOT NULL CHECK(role IN ('wav','manifest')),
    sha256 TEXT NOT NULL CHECK(length(sha256)=64),
    size_bytes INTEGER NOT NULL CHECK(size_bytes>0),
    UNIQUE(assembly_id,role)
);
PRAGMA user_version = 19;
