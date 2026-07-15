# Legacy `artifacts_json` Lifecycle

This document describes the lifecycle and removal plan for the `artifacts_json` column in the `projects` table.

## Status: Legacy / Deprecated

The `artifacts_json` field is **not** part of the current domain model (`Project`).
- It is a legacy field created in `0001_create_projects.sql`.
- New repository flows and Unit of Work (UoW) implementations **must not** write to this field.
- The only authorized reader is the `legacy backfill` process (`backfill_artifacts.rs`), which reads the JSON to migrate legacy artifacts to the new `artifacts` table.

## Lifecycle Events

1. **New Projects**: New projects created after the database migration do not populate this field.
2. **Runtime Backfill**: During application startup, `backfill_artifacts.rs` reads `artifacts_json`.
3. **Clear on Migration**: After successfully migrating legacy artifacts to the `artifacts` table, the backfill process updates the row setting `artifacts_json = '[]'`.
4. **All Managed Artifacts**: All managed artifacts post-migration reside strictly in the `artifacts` table.

## Deletion Plan

A follow-up task has been registered to completely drop the `artifacts_json` column from the database schema.
**Conditions for column deletion**:
- A "compatibility window" must pass, ensuring all supported deployments have passed through at least one bridge release that runs the backfill.
- A preflight check must execute before the SQL migration ensuring:
  - There are no rows where `artifacts_json != '[]'`.
  - There are no incomplete backfill failures remaining.
- The migration to drop the column should be tested on a fresh database, a backfilled database, and must gracefully fail (without data loss) if unmigrated artifacts exist.
