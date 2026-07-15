# Legacy Dev Database Migration

When developing Auralis, early databases were handcrafted without SQLx migrations. To prevent data loss or catastrophic failures during startup on an old dev database, the application performs a **preflight check** during SQLite connection initialization.

## Preflight Check Logic

The SQLite connection manager (`connect_sqlite`) inspects the existing `sqlite_master` table:

1. **Migrated Database**: If the `_sqlx_migrations` table exists, the application proceeds with normal SQLx migrations (`sqlx::migrate!`).
2. **Handcrafted Database**: If `_sqlx_migrations` is missing but the `projects` table exists, the application identifies it as a legacy dev database.
3. **Unknown Schema**: If neither table exists but there are other user-defined tables, the application aborts to avoid overwriting unrelated data.

## Automatic Backup Process

When a handcrafted dev database is detected, the application will:
1. Close the SQLite connection.
2. Rename the database files to include a backup timestamp:
   - `app.sqlite` -> `app.sqlite.backup_<timestamp>`
   - `app.sqlite-wal` -> `app.sqlite-wal.backup_<timestamp>`
   - `app.sqlite-shm` -> `app.sqlite-shm.backup_<timestamp>`
3. Emit a warning log indicating the backup path.
4. Recreate a fresh database and apply all current SQLx migrations.

## Restoring from Backup

If you need to retrieve data from your legacy dev database:
1. Stop the application.
2. Locate the `.backup_<timestamp>` files in the same directory as the database (e.g. `~/.local/share/auralis/`).
3. Rename them back to their original `.sqlite`, `.sqlite-wal`, and `.sqlite-shm` names.
4. Note: The application will automatically back them up again on next startup unless you manually write a script to import the data into the new schema. 

Currently, **legacy dev data is not automatically imported into the new schema**. If you require the old data to be present in the new schema, an explicit importer script must be written to map the old rows to the new database structure.
