# Smart CI Path Filtering

## Goal
The primary goal of Smart CI Path Filtering is to drastically reduce CI execution time and save runner minutes by executing only the jobs that are strictly necessary for the changed files. By doing so, we provide developers with much faster feedback loops while maintaining a bulletproof quality gate.

## Required Check
**Only `CI Summary` should be configured as a required check in GitHub Branch Protection rules.**

You must NOT set `Frontend`, `Rust`, `Docs`, or `Global Quality` as required checks. If you do, GitHub will wait infinitely (Pending state) for jobs that were intentionally skipped. The `ci-summary` job is intelligently designed to evaluate the statuses of all other jobs, correctly interpreting `skipped` jobs as valid and passing, while failing if any executed job returns a `failure` or `cancelled` state.

## Path Groups
The repository is split into distinct logical zones tracked by the `changes` CI job:
- **frontend**: React code, styling, frontend configuration (e.g., `apps/desktop/**`, `package.json`, `tailwind.config.*`).
- **rust**: Rust workspace, Tauri backend (e.g., `crates/**`, `src-tauri/**`, `Cargo.toml`).
- **docs**: All markdown documentation files (`docs/**`, `README.md`, etc.).
- **quality/global**: Repository-wide tooling (`Taskfile.yml`, `taskfiles/**`, `.editorconfig`, `.prettier*`).
- **release/tauri**: Production release configuration (`src-tauri/tauri.conf.json`, `src-tauri/capabilities/**`, `release.yml`).
- **ci**: GitHub Actions workflows (`.github/workflows/**`).

## Behavior Examples

### Docs-only commit
- **Changed**: `docs/ci/009-path-filtering.md`
- **Run**: `changes` -> `docs` -> `ci-summary`
- **Skip**: `frontend`, `rust`, `quality-global`

### Frontend-only commit
- **Changed**: `apps/desktop/src/widgets/export-panel/ui/ExportPanel.tsx`
- **Run**: `changes` -> `frontend` -> `ci-summary`
- **Skip**: `rust`, `docs`, `quality-global`

### Rust-only commit
- **Changed**: `crates/application/src/run_dubbing/service.rs`
- **Run**: `changes` -> `rust` -> `ci-summary`
- **Skip**: `frontend`, `docs`, `quality-global`

### Taskfile / Tooling changes
- **Changed**: `Taskfile.yml`
- **Run**: `changes` -> `frontend`, `rust`, `docs`, `quality-global` -> `ci-summary` (All checks run to ensure global tooling changes didn't break anything).

### Release / Tauri config commit
- **Changed**: `src-tauri/tauri.conf.json`
- **Run**: `changes` -> `rust` (to verify backend), `frontend` (to verify Tauri build dependencies) -> `ci-summary`.

## How to force full CI
If you ever need to forcefully trigger the entire CI suite to validate the whole repository, you can:
1. Make a trivial update or comment in `Taskfile.yml` or `.github/workflows/ci.yml`.
2. Manually trigger the CI via the GitHub Actions UI (`workflow_dispatch`).

## Rules
- **Do not use workflow-level `paths-ignore` for required checks.** Doing so skips the entire workflow, causing Branch Protection to hang indefinitely. We only use job-level conditional filtering (`if: needs.changes.outputs...`).
- **Keep `ci-summary` always running.** Its condition is `if: always()`.
- **Treat skipped jobs as valid.** The CI Summary explicitly checks for failures or cancellations, considering skipped jobs as part of a successful, optimized run.
