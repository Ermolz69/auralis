# Smart CI Path Filtering

## Goal

The primary goal of Smart CI Path Filtering is to drastically reduce CI execution time and save runner minutes by executing only the jobs that are strictly necessary for the changed files. By doing so, we provide developers with much faster feedback loops while maintaining a bulletproof quality gate.

## Required Check

**Only `CI Summary` should be configured as a required check in GitHub Branch Protection rules.**

Do not require individual conditional jobs, including `Frontend`, `Rust`, `Docs`, `Global Quality`, or `Release Check Parity`. GitHub can otherwise wait for jobs that were intentionally skipped. The `ci-summary` job evaluates all these jobs, accepts `skipped`, and fails if any job returns `failure` or `cancelled`.

## Path Groups

The repository is split into distinct logical zones tracked by the `changes` CI job:

- **frontend**: React code, styling, frontend configuration (e.g., `apps/desktop/**`, `package.json`, `tailwind.config.*`).
- **rust**: Rust workspace, Tauri backend (e.g., `crates/**`, `src-tauri/**`, `Cargo.toml`).
- **docs**: All markdown documentation files (`docs/**`, `README.md`, etc.).
- **quality/global**: Repository-wide tooling (`Taskfile.yml`, `taskfiles/**`, `tools/scripts/**`, `.agents/AGENTS.md`, `.editorconfig`, `.prettier*`).
- **release/tauri**: Production release configuration (`src-tauri/tauri.conf.json`, `src-tauri/capabilities/**`, `release.yml`).
- **ci**: GitHub Actions workflows and shared actions (`.github/workflows/**`, `.github/actions/**`).

`release-checks` runs the same reusable full-check workflow as release validation when the `release`, `ci`, `quality`, or `global` group changes. It does not publish or create tags. Ordinary docs, frontend, or Rust-only edits keep their selective jobs without an additional full-check run.

## Behavior Examples

### Docs-only commit

- **Changed**: `docs/ci/009-path-filtering.md`
- **Run**: `changes` -> `docs` -> `ci-summary`
- **Skip**: `frontend`, `rust`, `quality-global`, `release-checks`

### Frontend-only commit

- **Changed**: `apps/desktop/src/widgets/export-panel/ui/ExportPanel.tsx`
- **Run**: `changes` -> `frontend` -> `ci-summary`
- **Skip**: `rust`, `docs`, `quality-global`, `release-checks`

### Rust-only commit

- **Changed**: `crates/application/src/run_dubbing/service.rs`
- **Run**: `changes` -> `rust`, `quality-global` -> `ci-summary`
- **Skip**: `frontend`, `docs`, `release-checks`

### Taskfile / Tooling changes

- **Changed**: `Taskfile.yml`
- **Run**: `changes` -> `frontend`, `rust`, `docs`, `quality-global`, `release-checks` -> `ci-summary` (all checks run to validate global tooling changes).

### Quality script changes

- **Changed**: `tools/scripts/check-file-size.mjs`
- **Run**: `changes` -> `quality-global`, `release-checks` -> `ci-summary`
- **Also Run When Needed**: any affected frontend, Rust, or docs jobs selected by the same change set.

### Release / Tauri config commit

- **Changed**: `src-tauri/tauri.conf.json`
- **Run**: `changes` -> `rust`, `frontend`, `quality-global`, `release-checks` -> `ci-summary`.

### Shared bootstrap changes

- **Changed**: `.github/actions/bootstrap/action.yml`
- **Run**: `changes` -> `frontend`, `rust`, `docs`, `quality-global`, `release-checks` -> `ci-summary`.

## How to force full CI

If you ever need to forcefully trigger the entire CI suite to validate the whole repository, you can:

1. Make a trivial update or comment in `Taskfile.yml` or `.github/workflows/ci.yml`.
2. Manually trigger the CI via the GitHub Actions UI (`workflow_dispatch`).

## Rules

- **Do not use workflow-level `paths-ignore` for required checks.** Doing so skips the entire workflow, causing Branch Protection to hang indefinitely. We only use job-level conditional filtering (`if: needs.changes.outputs...`).
- **Keep `ci-summary` always running.** Its condition is `if: always()`.
- **Treat skipped jobs as valid.** The CI Summary explicitly checks for failures or cancellations, considering skipped jobs as part of a successful, optimized run.
