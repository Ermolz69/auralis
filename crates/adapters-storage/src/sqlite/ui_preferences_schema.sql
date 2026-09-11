CREATE TABLE ui_settings (
    key TEXT PRIMARY KEY CHECK (key = 'color_theme'),
    value TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 9007199254740991)
);
CREATE TABLE project_pins (
    project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,
    pinned INTEGER NOT NULL CHECK (pinned IN (0, 1)),
    revision INTEGER NOT NULL CHECK (revision BETWEEN 1 AND 9007199254740991)
);
CREATE TABLE ui_migrations (key TEXT PRIMARY KEY CHECK (key = 'project_pins_v1'));
PRAGMA user_version = 5;
