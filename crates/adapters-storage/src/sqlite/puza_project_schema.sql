CREATE TABLE puza_contract_projects (
    project_id TEXT PRIMARY KEY NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    snapshot_json TEXT NOT NULL CHECK(json_valid(snapshot_json)),
    snapshot_sha256 TEXT NOT NULL CHECK(length(snapshot_sha256) = 64)
);
PRAGMA user_version = 17;
