PRAGMA user_version = 12;

CREATE TABLE job_executions (
    job_id TEXT PRIMARY KEY NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
    descriptor_json TEXT NOT NULL CHECK (length(descriptor_json) <= 16384)
);
