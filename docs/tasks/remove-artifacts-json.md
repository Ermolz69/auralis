# Task: Remove `artifacts_json` from `projects`

## Context

The `artifacts_json` column is a legacy field used for storing project artifacts. In the new architecture, artifacts are managed as separate rows in the `artifacts` table. We have introduced a runtime legacy backfill process to migrate these artifacts, and new operations no longer write to the `artifacts_json` column.

PR-1 preserves this column at runtime as a temporary compatibility bridge. The final drop of the column is deferred and must not happen inside the runtime backfill.

## Goal

Completely drop the `artifacts_json` column from the database schema once it is safe to do so.

## Prerequisites

- **Compatibility Window**: Wait until we are confident that all supported deployments have passed through at least one bridge release executing the `backfill_artifacts` migration.
- **Preflight Check**: Ensure there are no rows left with `artifacts_json != '[]'` across all deployments.
- **Preflight Check**: Ensure there are no pending backfill failures.
- **Execution Mechanism**: The dropping of the column must be performed through a standard SQLx migration, never at application runtime.

## Implementation Details

1. Create a standard SQLx migration file (e.g., `ALTER TABLE projects DROP COLUMN artifacts_json;`).
2. Update tests to verify the upgrade works on:
   - Fresh databases (where the column was created as deprecated then dropped).
   - Bridged databases (where backfill completed successfully).
   - Already-dropped databases (where previous versions might have run a table rebuild).
3. Remove the runtime `backfill_artifacts` logic from the application after the migration is safely rolled out and the compatibility window is closed.
