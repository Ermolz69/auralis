# CI & Quality Gates

Our continuous integration pipeline enforces strict quality gates on every pull request to ensure codebase stability.

## Core Principle: No Unique Logic in CI

Checks run through the root `Taskfile`, so developers can reproduce each gate using the corresponding `task` command. Runner preparation is shared by CI, release checks, desktop builds, and Storybook Pages through `.github/actions/bootstrap/action.yml`.

The bootstrap selects only the components needed by each job: Node/pnpm and frozen frontend dependencies, Rust and Linux Tauri libraries, or Playwright Chromium with its system dependencies. Documentation and Pages jobs do not fetch Rust dependencies or install browsers. Node versions and the Tauri package list have one shared definition.

Its separate security component installs pinned Rust auditors without Tauri/GTK libraries
or browsers. The `Dependency Security` job runs only when dependency manifests,
lockfiles, auditor configuration, or CI itself changes. Its result is included in
`CI Summary`. The reusable full release check enables the security component as well.

Release validation runs `task check` through `.github/workflows/full-checks.yml`, with all bootstrap components enabled.
Pull requests intentionally do not repeat that full workflow or build three native
installers. They run selective, locally reproducible gates; native multi-platform
packaging and cross-platform recovery tests run manually or for production tags.

## Quality Gates

The local and CI entrypoints are:

- **frontend install**: Installs frozen pnpm dependencies without Rust (`task install:frontend`).
- **Rust install**: Fetches locked Cargo dependencies without frontend tooling (`task install:rust`).
- **full install**: Runs both installers (`task install:all`, also available as `task install`); browser setup is a separate prerequisite for checks.
- **frontend**: Runs strict TypeScript checks, lint, tests with enforced V8 coverage thresholds (85% statements, 75% branches, 85% functions, and 88% lines), build with bundle budgets, 20 production-browser E2E journeys, and production CSP browser smoke tests (`task check:frontend`).
- **frontend quality**: Runs FSD boundary checks, color-token checks, file-size and duplicate-code checks, bundle/CSP policy regressions, IPC contract parity, release metadata parity, and release smoke-tool tests (`task check:quality:frontend`).
- **Rust PR**: Uses the toolchain pinned in `rust-toolchain.toml`, then runs dependency policy, `cargo fmt`, locked `cargo clippy`, and workspace tests (`task check:rust:pr`). Clippy already performs compilation, so CI does not immediately repeat `cargo check`.
- **Rust extended**: Adds the optimized GLib regression for local/release validation (`task check:rust`).
- **docs**: Runs markdown checks (`task check:docs`).
- **docs quality**: Runs markdown formatting checks (`task check:quality:docs`).
- **dependency security**: Audits all npm dependencies and Cargo.lock, then enforces Rust advisory/source policy (`task check:quality:security`). Local setup requires `task sec:setup:rust`.
- **global quality**: Runs media manifest, IPC contract, release metadata, Cargo workspace-dependency, SQLite-only dependency, GLib provenance, repository formatting, runtime println, storage fallback, and CI bootstrap checks (`task check:quality:global`).
- **CI bootstrap**: Tests shared workflow wiring, dependency ordering, platform guards, PR cost boundaries, release isolation, and scoped installer execution plans (`task q:ci-bootstrap`).
- **runtime println**: Runs the runtime println fixture suite and production scan (`task check:quality:runtime-println`).
- **crash recovery**: Workspace tests cover recovery on PRs; production tags additionally run the focused SQLite and YouTube recovery suites on Windows and macOS.
- **native bundle**: Three-platform packaging, bundled-media verification, and installed-app smoke checks run manually or for production tags, not on every PR.
- **Storybook**: The frontend job executes Storybook browser tests through `task check:frontend`; the static catalog is built by its dedicated Pages workflow instead of being duplicated in PR CI.
- **PR suite**: Reproduces all required pull-request gates locally (`task check:pr`).
- **full suite**: Adds the optimized Linux GLib regression to the PR suite (`task check:all`, also `task check`).

Script-backed quality gates must run their own fixture suites before scanning production code.
