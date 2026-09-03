# CI & Quality Gates

Our continuous integration pipeline enforces strict quality gates on every pull request to ensure codebase stability.

## Core Principle: No Unique Logic in CI

Checks run through the root `Taskfile`, so developers can reproduce each gate using the corresponding `task` command. Runner preparation is shared by CI, release checks, desktop builds, and Storybook Pages through `.github/actions/bootstrap/action.yml`.

The bootstrap selects only the components needed by each job: Node/pnpm and frozen frontend dependencies, Rust and Linux Tauri libraries, or Playwright Chromium with its system dependencies. Documentation and Pages jobs do not fetch Rust dependencies or install browsers. Node versions and the Tauri package list have one shared definition.

Release validation runs `task check` through `.github/workflows/full-checks.yml`, with all bootstrap components enabled. CI calls that same reusable workflow when CI, release, or repository tooling configuration changes. This validates release-check parity before merging, without creating a tag or publishing artifacts.

## Quality Gates

The local and CI entrypoints are:

- **install**: Verifies dependency resolution with frozen package resolution (`task install`).
- **frontend**: Runs typecheck, lint, tests, and build (`task check:frontend`).
- **frontend quality**: Runs FSD boundary checks, color-token checks, and file-size checks (`task check:quality:frontend`).
- **Rust**: Runs `cargo fmt`, `cargo clippy`, `cargo check`, and workspace tests (`task check:rust`).
- **docs**: Runs markdown checks (`task check:docs`).
- **docs quality**: Runs markdown formatting checks (`task check:quality:docs`).
- **global quality**: Runs repository formatting, runtime println, storage fallback, and CI bootstrap checks (`task check:quality:global`).
- **CI bootstrap**: Tests shared workflow wiring, dependency ordering, platform guards, and release-check parity (`task q:ci-bootstrap`).
- **runtime println**: Runs the runtime println fixture suite and production scan (`task check:quality:runtime-println`).
- **full suite**: Runs all required gates (`task check:all`).

Script-backed quality gates must run their own fixture suites before scanning production code.
