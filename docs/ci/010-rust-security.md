# Rust Dependency Security

## Scope and setup

Run `task sec:setup:rust` once before local full checks. It installs `cargo-audit 0.22.2`
and `cargo-deny 0.20.2` with locked tool dependencies. CI uses prebuilt binaries of those
same versions, shared by the independent security job and release checks.

- `task sec:audit:rust` checks the entire `Cargo.lock`, including dependencies not active
  in the current target/feature graph. Vulnerabilities and unsound advisories fail.
- `task sec:deny:rust` checks advisories and dependency sources for the resolved graph.
  Vulnerabilities, unsound advisories (including transitive ones), yanked versions,
  unknown registries, and unapproved Git dependencies fail.
- Unmaintained direct dependencies fail cargo-deny; transitive maintenance notices
  remain visible in cargo-audit. This is distinct from vulnerability suppression.
- License policy and duplicate-version bans are not enabled by this security task.

Both checks fetch current advisory data and fail on errors. No advisory exceptions are
configured. `task check:quality:security` runs npm audit and both Rust checks; it is part
of `task check`. Frontend/docs jobs do not install these tools or run Rust checks.

## Findings from the initial rollout

The initial scan on 2026-09-03 found and allowed compatible lockfile remediation of:

- `event-listener 5.4.1` to `5.4.2` for
  [RUSTSEC-2026-0221](https://rustsec.org/advisories/RUSTSEC-2026-0221).
- Yanked `spin 0.9.8` to `0.9.9`.

The remaining rollout blockers were remediated without advisory exceptions:

- `rsa 0.9.10`, [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071):
  the [SQLite-only SQLx 0.9 update](013-sqlite-only-dependencies.md) removes RSA,
  MySQL and PostgreSQL from the complete lockfile, not merely the active graph.
  Lockfile and resolved-feature regression gates reject their reintroduction.
- `glib 0.18.5`, [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429):
  the [maintained safety backport](012-glib-backport.md) applies the upstream
  out-pointer correction while retaining GTK3 compatibility. Archive integrity
  and optimized positive/negative Linux regressions independently verify the fix;
  the advisory scanner's treatment of a local package is not sufficient evidence.

Do not merge a failing security gate or silence findings without a project-owner
decision. The 16 transitive maintenance notices for GTK3, `unic-*` and
`proc-macro-error` remain visible in cargo-audit; they are not compiler warnings,
version-deprecation messages or fixed vulnerabilities. The policy still rejects
all vulnerabilities and unsound advisories and does not inherit upstream ignores.
Replacing the unsupported dependency families requires separate upstream-stack work.

## Investigation and upgrades

Use `task rs:tree -- --invert <crate> --target all` to inspect active dependency paths.
Use `task rs:update -- --package <crate> --precise <version>` for a reviewed targeted
upgrade, then run `task check:rust` and `task check:quality:security`.
