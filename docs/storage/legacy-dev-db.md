# Legacy Dev Database Transition

Early Auralis development databases were created by hand, before SQLx migrations.
Startup runs a SQLite preflight transition so those legacy databases are never
overwritten silently.

The transition does not import legacy rows into the managed schema. It preserves
the old database in a durable backup and activates a fresh SQLx-managed database.

## Detection

Startup inspects the database read-only:

- Missing, empty, or already SQLx-managed databases continue normally.
- The known handcrafted V0 schema is transitioned.
- Unknown schemas fail closed.
- Corrupt SQLite files fail as corrupt databases.
- If a transition manifest exists, startup resumes from the manifest before
  applying any missing-database early return.

## Filesystem Layout

For `app.sqlite`, the transition uses:

- `app.sqlite.lock` as the short-lived transition owner lock.
- `backups/transition_manifest.json` as the durable resume manifest.
- `backups/auralis-<timestamp>-<operation-id>/` as the preserved legacy backup.
- `backups/auralis-<timestamp>-<operation-id>/app.sqlite` plus optional
  `app.sqlite-wal` and `app.sqlite-shm`.
- `backups/auralis-<timestamp>-<operation-id>/manifest.json` as the backup copy
  of the transition manifest.
- `app.sqlite.new-<operation-id>` as the prepared managed database before
  activation.
- `backups/quarantine-<timestamp>-<operation-id>/` as temporary quarantine for
  the old live database files.

The backup directory is never deleted by transition rollback or cleanup.

## Ownership And Staleness

The lock payload contains the operation id, owner pid, timestamp, and manifest
path. Startup validates the payload before acting on it.

- A live owner is not stolen.
- A corrupt lock is a typed failure and is not removed automatically.
- A stale lock may be reclaimed only after the configured staleness window.
- Reclaim failure is reported as a typed startup failure.
- The lock is not held during long copy, migration, or rename operations; durable
  resume is driven by the manifest and filesystem checks.

The current staleness contract is `15 minutes`.

## Resume Matrix

| Durable stage | Expected filesystem state | Next action |
| --- | --- | --- |
| `Started` | Legacy DB still live; manifest exists | Checkpoint legacy DB, create and validate backup |
| `BackupFinalized` | Backup exists and validates; legacy DB still live | Prepare fresh SQLx database |
| `NewDatabaseReady` | Backup exists; legacy DB live; new DB exists and validates | Move legacy DB files to quarantine |
| `OldDatabaseQuarantined` | Legacy DB quarantined; new DB still staged | Rename new DB into active database path |
| `NewDatabaseActivated` | Active DB is SQLx-managed; backup still exists | Remove quarantine and transition manifest |

Each stage is repeatable. On resume, the implementation checks the actual file
state first and either performs the next idempotent operation or returns a typed
resume mismatch.

## Failure Contract

Startup failures are typed and sanitized:

- live transition lock;
- stale lock reclaim failure;
- corrupt lock payload;
- corrupt transition manifest or path escape;
- resume/filesystem mismatch;
- cleanup failure;
- backup, migration, and validation failures.

Manifest paths are validated so transition state cannot move or remove files
outside the database directory and its `backups` root. Manifest database names
must be plain filenames.

## Restoring From Backup

To inspect or restore legacy data:

1. Stop the application.
2. Open the preserved directory under `backups/auralis-<timestamp>-<operation-id>/`.
3. Use the contained `app.sqlite` with optional WAL/SHM files as the legacy
   source.

An importer from handcrafted V0 into the managed schema is intentionally out of
scope for this transition.
