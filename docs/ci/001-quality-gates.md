# CI & Quality Gates

Our continuous integration pipeline enforces strict quality gates on every pull request to ensure codebase stability.

## Core Principle: No Unique Logic in CI

The CI pipeline strictly **does not contain any unique logic**. It serves only as a runner that delegates execution directly to our local `Taskfile`. This ensures that any check running (or failing) in CI can be exactly reproduced locally by developers using the corresponding `task` command.

## Quality Gates

The pipeline runs the following checks sequentially:

- **install**: Verifies dependency resolution and environment setup (`task install`).
- **TypeScript check**: Runs static type analysis for the frontend (`task lint:ts`).
- **ESLint**: Enforces code style, FSD boundaries, and best practices for React/TS (`task lint:es`).
- **Stylelint**: Enforces CSS/Tailwind structure and rules (`task lint:style`).
- **frontend tests**: Executes unit and integration tests for the React UI (`task test:frontend`).
- **Storybook build**: Verifies that the UI documentation compiles successfully without breaking (`task build:storybook`).
- **cargo fmt**: Checks Rust codebase formatting (`task format:rust`).
- **cargo clippy**: Runs Rust static analysis and catches common mistakes (`task lint:rust`).
- **cargo test**: Runs backend unit and integration tests across the workspace (`task test:rust`).
- **architecture checks**: Validates boundaries between FSD layers and Rust clean architecture (`task check:arch`).
- **file size checks**: Prevents regressions by enforcing limits on frontend bundle sizes and Tauri sidecars (`task check:size`).
