# CI & Quality Gates

Our continuous integration pipeline enforces strict quality gates on every pull request to ensure codebase stability.

## Core Principle: No Unique Logic in CI

The CI pipeline strictly **does not contain any unique logic**. It serves only as a runner that delegates execution directly to our local `Taskfile`. This ensures that any check running (or failing) in CI can be exactly reproduced locally by developers using the corresponding `task` command.

## Quality Gates

The local and CI entrypoints are:

- **install**: Verifies dependency resolution with frozen package resolution (`task install`).
- **frontend**: Runs typecheck, lint, tests, and build (`task check:frontend`).
- **frontend quality**: Runs FSD boundary checks, color-token checks, and file-size checks (`task check:quality:frontend`).
- **Rust**: Runs `cargo fmt`, `cargo clippy`, `cargo check`, and workspace tests (`task check:rust`).
- **docs**: Runs markdown checks (`task check:docs`).
- **docs quality**: Runs markdown formatting checks (`task check:quality:docs`).
- **global quality**: Runs repository formatting, runtime println, and storage fallback checks (`task check:quality:global`).
- **runtime println**: Runs the runtime println fixture suite and production scan (`task check:quality:runtime-println`).
- **full suite**: Runs all required gates (`task check:all`).

Script-backed quality gates must run their own fixture suites before scanning production code.
