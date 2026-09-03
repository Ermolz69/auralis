# CI & Quality Gates

Our continuous integration pipeline enforces strict quality gates on every pull request to ensure codebase stability.

## Core Principle: No Unique Logic in CI

Checks run through the root `Taskfile`, so developers can reproduce each gate using the corresponding `task` command. Runner preparation is shared by CI, release checks, desktop builds, and Storybook Pages through `.github/actions/bootstrap/action.yml`.

The bootstrap selects only the components needed by each job: Node/pnpm and frozen frontend dependencies, Rust and Linux Tauri libraries, or Playwright Chromium with its system dependencies. Documentation and Pages jobs do not fetch Rust dependencies or install browsers. Node versions and the Tauri package list have one shared definition.

Its separate security component installs pinned Rust auditors without Tauri/GTK libraries
or browsers. The `Dependency Security` job runs independently of frontend/docs/global
quality jobs, and its result is included in `CI Summary`. The reusable full release
check enables the security component as well.

Release validation runs `task check` through `.github/workflows/full-checks.yml`, with all bootstrap components enabled. CI calls that same reusable workflow when CI, release, or repository tooling configuration changes. This validates release-check parity before merging, without creating a tag or publishing artifacts.

## Quality Gates

The local and CI entrypoints are:

- **frontend install**: Installs frozen pnpm dependencies without Rust (`task install:frontend`).
- **Rust install**: Fetches locked Cargo dependencies without frontend tooling (`task install:rust`).
- **full install**: Runs both installers (`task install:all`, also available as `task install`); browser setup is a separate prerequisite for checks.
- **frontend**: Runs typecheck, lint, tests, build with bundle budgets, and production CSP browser smoke tests (`task check:frontend`).
- **frontend quality**: Runs FSD boundary checks, color-token checks, file-size checks, and bundle/CSP policy regression tests (`task check:quality:frontend`).
- **Rust**: Runs `cargo fmt`, `cargo clippy`, `cargo check`, and workspace tests (`task check:rust`).
- **docs**: Runs markdown checks (`task check:docs`).
- **docs quality**: Runs markdown formatting checks (`task check:quality:docs`).
- **dependency security**: Audits all npm dependencies and Cargo.lock, then enforces Rust advisory/source policy (`task check:quality:security`). Local setup requires `task sec:setup:rust`.
- **global quality**: Runs repository formatting, runtime println, storage fallback, and CI bootstrap checks (`task check:quality:global`).
- **CI bootstrap**: Tests shared workflow wiring, dependency ordering, platform guards, release-check parity, and scoped installer execution plans (`task q:ci-bootstrap`).
- **runtime println**: Runs the runtime println fixture suite and production scan (`task check:quality:runtime-println`).
- **full suite**: Runs all required gates (`task check:all`).

Script-backed quality gates must run their own fixture suites before scanning production code.
