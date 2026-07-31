# Legacy `artifacts_json` Lifecycle

This document describes the lifecycle and removal plan for the `artifacts_json` column in the `projects` table.

## Status: Legacy / Deprecated

The `artifacts_json` field is **not** part of the current domain model (`Project`).

- It is a legacy field created in `0001_create_projects.sql`.
- New repository flows and Unit of Work (UoW) implementations **must not** write to this field.
- The only authorized reader/writer is the `legacy backfill` process (`backfill_artifacts.rs`), which reads the JSON to migrate legacy artifacts to the new `artifacts` table.

## Lifecycle Events

1. **New Projects**: New projects created after the database migration do not populate this field (they receive the default value of `'[]'`).
2. **Runtime Backfill**: During application startup, `backfill_artifacts.rs` checks if the `artifacts_json` column is present in the database schema. If absent, the migration successfully exits immediately as a no-op.
3. **Atomic Clear on Migration**: For each legacy project row with a non-empty `artifacts_json` column:
   - The migration starts a dedicated SQLite transaction covering the checks, location checks, inserts, and clearing of JSON.
   - Upon successfully validating and inserting all artifacts, the row is updated atomically setting `artifacts_json = '[]'` using an optimistic CAS condition (`WHERE id = ? AND artifacts_json = ?`).
   - If `rows_affected()` is not exactly 1, the transaction rolls back, leaving the legacy JSON intact.
4. **No Runtime Schema Alteration**: The runtime backfill process never performs `DROP TABLE`, `ALTER TABLE`, or RENAME operations.

## Deletion Plan

A follow-up task has been registered to completely drop the `artifacts_json` column from the database schema.
**Conditions for column deletion**:

- A "compatibility window" must pass, ensuring all supported deployments have passed through at least one bridge release that runs the backfill.
- A preflight check must execute before the SQL migration ensuring:
  - There are no rows where `artifacts_json != '[]'`.
  - There are no incomplete backfill failures remaining.
- The column drop itself must be performed through a standard SQLx migration, rather than runtime code.
- The migration to drop the column should be tested on a fresh database, a backfilled database, and must gracefully fail (without data loss) if unmigrated artifacts exist.
