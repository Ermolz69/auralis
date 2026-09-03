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

The strict audit still blocks rollout on these unresolved upstream dependencies:

- `rsa 0.9.10`, [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071):
  no patched version is available. It is present in the SQLx/MySQL lockfile chain,
  although `task rs:tree -- --invert rsa --target all` shows no active use. Disabling
  unused SQLx default features did not remove it from the lockfile; that experiment
  was reverted. An inactive graph is evidence for review, not an automatic exemption.
- `glib 0.18.5`, [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429):
  fixed in `>=0.20.0`, incompatible with the current Tauri/GTK3 stack. The
  [upstream Tauri audit configuration](https://github.com/tauri-apps/tauri/blob/dev/.cargo/audit.toml)
  identifies GTK4 migration as the fix. Upstream exceptions are not inherited here.

Do not merge a failing security gate or silence these findings without a project-owner
decision. A reviewed exception must record its scope, risk, owner, and removal condition;
otherwise remediate or replace the dependency chain before rollout. GTK3 and `unic-*`
transitive unmaintained notices are also reported and require ongoing tracking.

## Investigation and upgrades

Use `task rs:tree -- --invert <crate> --target all` to inspect active dependency paths.
Use `task rs:update -- --package <crate> --precise <version>` for a reviewed targeted
upgrade, then run `task check:rust` and `task check:quality:security`.
