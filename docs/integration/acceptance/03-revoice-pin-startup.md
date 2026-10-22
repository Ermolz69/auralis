# Concurrent pin-import startup correction

Date: 23 October 2026. Scope: an INT-R05 startup defect found in the ordinary
installed candidate. Native source import remains pending.

The normal app log reported `import_project_pins_cmd` BUSY. The importer opened
a deferred transaction, read its migration marker and then attempted to write.
In SQLite WAL mode an intervening writer makes that read-to-write upgrade fail
immediately; the configured busy timeout cannot resolve the stale snapshot.

The importer now opens `BEGIN IMMEDIATE`, acquiring writer admission before
reading the marker. The existing five-second busy timeout bounds waiting. The
same transaction still records imported pins and the marker together. Duplicate
imports stay idempotent and legacy entries cannot replace a newer native pin.

`task rs:test:ui-preferences` first reproduced the defect on a temporary database:
three tests passed and the new held-writer control failed with
`Busy { message: "ui_preferences: Database is busy or locked" }`. After the
one-line transaction change, all four tests passed, including independent pools,
duplicate startup imports, rollback, persistence and the old-schema upgrade.
The initial narrower command supplied two Cargo name filters and was rejected
before testing; the task's existing filter was then used without another filter.

No installed database was modified by these tests. This correction is not yet
in the installed `b6dc303` payload. Native retry requires a newly built and
identified installer; no native pass is claimed from the SQLite control.
