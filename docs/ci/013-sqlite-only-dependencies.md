# SQLite-only SQLx dependencies

## Dependency policy

Auralis uses SQLite, not MySQL or PostgreSQL. SQLx 0.9.0 replaces 0.8.6 and removes
the unused MySQL/RSA chain from the complete `Cargo.lock`, including inactive
optional dependencies. No RustSec exception, patched RSA fork or prerelease RSA
version is used. The minimum Rust version for SQLx 0.9 is 1.94; verification uses
Rust 1.95.0 on Windows and Ubuntu WSL.

The root workspace dependency disables SQLx defaults and enables only
`runtime-tokio` and `sqlite-bundled`. Storage additionally enables `derive` for
`FromRow`. Application integration tests inherit the same workspace dependency.
TLS, `any`, query macros, SQLx migration tooling, JSON and UUID SQL codecs, SQLite
extension loading and deserialization are not requested. Application JSON and UUID
values retain their existing explicit serialization.

Storage depends directly on `sqlx-sqlite` with `chrono` enabled to preserve existing
timestamp bindings in recovery writes. Enabling `sqlx/chrono` instead brings
optional MySQL/PostgreSQL packages back into the lockfile; the driver-specific
feature avoids this without changing timestamp encoding. `sqlx-core` may retain
features enabled internally by upstream SQLx; the policy does not claim that every
internal migration-related item is removed.

The locked `libsqlite3-sys` stays at 0.30.1. This change does not replace the bundled
SQLite engine or change Auralis's database schema/version. Existing schema-v1/v2
upgrade, rollback, recovery, outbox, revision-conflict and persisted-avatar tests
continue to exercise the same storage contracts.

## SQLx API adaptation

SQLx 0.9 requires static or explicitly reviewed dynamic SQL strings. Production
schema inspection now uses `pragma_table_info(?)` and a bound table name, replacing
string interpolation. Table-count assertions use static query strings.
`AssertSqlSafe` is limited to tests that derive old schemas from the checked-in
schema or build failure-injection triggers from literal table/condition lists.
No external input reaches these test-only assertions.

## Regression gates

- `task sec:sqlite:verify` checks every lockfile package and tests the guard itself.
  It rejects `rsa`, `sqlx-mysql`, `sqlx-postgres`, missing/duplicate SQLx packages,
  mixed releases and a return to SQLx 0.8. This runs in global quality without Rust.
- `task rs:dependencies:verify` also checks Cargo's actual resolved packages and
  features, including dev-dependency feature unification. It is part of
  `task check:rust:pr`, `task check:rust`, and release checks.
- `task rs:test:storage` exercises storage using the locked graph. Additional
  regressions hold all five pooled connections to check WAL and foreign-key
  enforcement, then check Unicode/text and nanosecond timestamp round trips,
  rollback on dropped transactions, database reopening and integrity.
- `task check` runs the full Windows check suite, including integration tests,
  browser smoke, dependency guards and existing GLib source verification.
- `task rs:exec:wsl -- test --locked --workspace`,
  `task rs:exec:wsl -- clippy --locked --workspace --all-targets -- -D warnings`,
  and `task rs:exec:wsl -- build --locked --release -p auralis-app` verify the Linux
  workspace and native production executable with warnings denied.
- `task sec:rust:audit` scans the complete lockfile without ignored advisories.
  It aliases `task sec:audit:rust`; run `task sec:setup:rust` to install the pinned
  audit tools. The integrated security job and release gate require both Rust
  advisory and dependency-source checks.

The GLib backport and its optimized regression remain independently maintained.
GTK3, `unic-*` and `proc-macro-error` maintenance notices are not fixed by this
SQLx update and must not be described as a warning-free dependency audit.
Native GUI interaction and signed installer distribution are outside these checks.

## Upstream references

- [SQLx 0.9.0 release changes and Rust requirement](https://github.com/transact-rs/sqlx/blob/v0.9.0/CHANGELOG.md)
- [SQLx 0.9.0 feature definitions](https://github.com/transact-rs/sqlx/blob/v0.9.0/Cargo.toml)
- [RSA timing-side-channel advisory](https://rustsec.org/advisories/RUSTSEC-2023-0071.html)

Review the package and resolved-feature guards when upgrading SQLx; do not loosen
them solely to silence a failed dependency update.
