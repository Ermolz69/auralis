# Task: Remove `artifacts_json` from `projects`

## Context
The `artifacts_json` column is a legacy field used for storing project artifacts. In the new architecture, artifacts are managed as separate rows in the `artifacts` table. We have introduced a runtime legacy backfill process to migrate these artifacts, and new operations no longer write to the `artifacts_json` column.

## Goal
Completely drop the `artifacts_json` column from the database schema once it is safe to do so.

## Prerequisites
- **Compatibility Window**: Wait until we are confident that all supported deployments have passed through at least one bridge release executing the `backfill_artifacts` migration.
- **Preflight Check**: Ensure there are no rows left with `artifacts_json != '[]'`.
- **Preflight Check**: Ensure there are no pending backfill failures.

## Implementation Details
1. Introduce a pre-migration hook or application startup check that aggressively halts if unmigrated artifacts exist, preventing data loss.
2. Create the SQL migration file (e.g., `ALTER TABLE projects DROP COLUMN artifacts_json;`).
3. Update tests:
   - Verify the migration works on a fresh database.
   - Verify the migration works on a backfilled database.
   - Verify the preflight check successfully halts the migration (without dropping the column) if unmigrated data exists.
4. Remove the runtime `backfill_artifacts` logic from the application since it will no longer be needed.
