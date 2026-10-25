ALTER TABLE audio_assembly_artifacts RENAME TO audio_assembly_artifacts_v19;
CREATE TABLE audio_assembly_artifacts (
    artifact_id TEXT PRIMARY KEY NOT NULL REFERENCES artifacts(id) ON DELETE CASCADE,
    assembly_id TEXT NOT NULL REFERENCES audio_assembly_intents(assembly_id) ON DELETE CASCADE,
    attempt_id TEXT NOT NULL REFERENCES audio_assembly_attempts(attempt_id),
    role TEXT NOT NULL CHECK(role IN ('wav','manifest','video','video_manifest')),
    sha256 TEXT NOT NULL CHECK(length(sha256)=64),
    size_bytes INTEGER NOT NULL CHECK(size_bytes>0),
    UNIQUE(assembly_id,role)
);
INSERT INTO audio_assembly_artifacts SELECT * FROM audio_assembly_artifacts_v19;
DROP TABLE audio_assembly_artifacts_v19;
PRAGMA user_version = 20;
