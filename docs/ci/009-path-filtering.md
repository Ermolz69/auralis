# Smart CI Path Filtering

## Goal

Pull requests run the smallest locally reproducible gate set that can validate the
changed area. Expensive native installers, installed-app smoke tests, optimized
platform regressions, and the complete release gate are reserved for manual runs
or production tags.

## Required Check

**Only `CI Summary` should be configured as a required branch-protection check.**

Do not require individual conditional jobs. `CI Summary` accepts intentionally
skipped jobs and fails when any selected job fails or is cancelled.

## Path Groups

- **frontend**: React code, styles, frontend configuration, and pnpm workspace files.
- **rust**: Rust crates, the Tauri backend, vendored GLib, and Cargo configuration.
- **docs**: Markdown documentation.
- **quality/global**: Taskfiles, repository policies, and validation scripts.
- **release**: Tauri packaging, release workflows, media tools, and signing tooling.
- **ci**: GitHub Actions workflows and the shared bootstrap action.
- **dependencies**: npm/Cargo manifests, lockfiles, source policy, and auditor setup.

Dependency security runs only for `dependencies` or `ci` changes. Application source
changes cannot alter the resolved dependency graph, so they do not spend runner time
re-auditing an unchanged lockfile.

## Pull-request jobs

- `Frontend`: typecheck, lint, unit and Storybook browser tests with coverage,
  production build budgets, 20 E2E journeys, CSP smoke, and frontend policies.
- `Rust`: dependency graph policy, formatting, Clippy, and workspace tests. Clippy
  already compiles all targets, so the job does not immediately repeat `cargo check`.
- `Docs`: documentation validation and formatting, only for documentation changes.
- `Global Quality`: repository-wide policy scripts for Rust, quality, global, or CI changes.
- `Dependency Security`: npm/Rust audits only when dependency inputs or CI change.
- `CI Summary`: the single stable required check.

The PR workflow never calls the reusable full release check and never builds the
three-platform installer matrix. Those operations previously duplicated validated
work and consumed most Actions minutes.

## Behavior examples

### Docs-only change

Runs `changes`, `Docs`, and `CI Summary`.

### Frontend source change

Runs `changes`, `Frontend`, and `CI Summary`. Dependency Security runs only when
the same change also modifies a dependency input.

### Rust source change

Runs `changes`, `Rust`, `Global Quality`, and `CI Summary`.

### Dependency update

Runs the affected frontend/Rust gate, `Dependency Security`, and `CI Summary`.

### CI or shared bootstrap change

Runs the lean frontend, Rust, global-quality, and security gates plus `CI Summary`.
It does not trigger native packages or the duplicated full release workflow.

## Extended verification

- Run `task check:pr` to reproduce every required PR gate locally.
- Run `task check` for the extended local/release suite.
- Run the `Tauri Build` workflow manually when installer validation is needed.
- Production `app-v*` tags run the complete release gate, platform recovery tests,
  native packaging, signing, updater manifest generation, and install/launch smoke.

## Rules

- Do not use workflow-level `paths-ignore` for required checks.
- Keep `CI Summary` on `if: always()`.
- Treat `skipped` as valid and `failure`/`cancelled` as blocking.
- Do not restore native bundle or complete release duplication to ordinary PRs.
