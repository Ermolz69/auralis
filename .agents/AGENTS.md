# Project Rules

- **No Non-Vital Comments**: Do not write explanatory, obvious, or non-vital comments in the code. Code should be self-documenting. Only use comments for highly complex logic, non-obvious workarounds, or critical warnings.
- **English Documentation**: All documentation files (e.g., Markdown files in `docs/`), commit messages, and internal documentation must be written in English.

## Module Splitting Rule

Do not put multiple responsibilities into one large module file.

When a module contains more than one domain concept, split it into separate files by responsibility. `mod.rs` must stay small and should only declare submodules and re-export the public API.

Use this structure by default:

```text
module/
├── mod.rs
├── id.rs
├── entity.rs
├── status.rs
├── kind.rs
├── error.rs
├── progress.rs
└── tests.rs
```

Apply only the files that make sense for the module. Do not create empty files.

Required rules:

* IDs go into `id.rs`.
* Main structs/entities go into `entity.rs`.
* Status enums go into `status.rs`.
* Type/kind enums go into `kind.rs`.
* Module-specific errors go into `error.rs`.
* Progress/state details go into `progress.rs`.
* Tests go into `tests.rs`.
* `mod.rs` must not contain large structs, enums, business logic, or tests.
* `mod.rs` should only contain `mod ...`, `pub use ...`, and minimal module documentation.
* Do not duplicate types across files.
* Do not define the same struct, enum, or ID twice.
* Keep each file focused on one responsibility.

For example, a `job` module should be split like this:

```text
job/
├── mod.rs
├── id.rs
├── entity.rs
├── status.rs
├── kind.rs
├── error.rs
├── progress.rs
└── tests.rs
```

The domain layer must stay pure. Do not put infrastructure logic, filesystem access, database access, FFmpeg calls, Tauri events, async runners, or UI logic inside domain modules.

## Task Runner Rule

All project commands must be executed through Taskfile tasks.

Agents must not run raw package-manager, Rust, frontend, lint, test, formatting, documentation, security, or CI commands directly when an equivalent `task` command exists.

Use the root `Taskfile.yml` as the single entrypoint for checks and workflows.

Preferred commands:

* `task check` — run the default full project check.
* `task check:frontend` — run frontend checks.
* `task check:rust` — run Rust checks.
* `task check:docs` — run documentation checks.
* `task check:quality:frontend` — run frontend quality checks.
* `task check:quality:docs` — run documentation quality checks.
* `task check:quality:security` — run security checks.
* `task check:all` — run all checks.
* `task ci` — run the CI-equivalent workflow.

Do not run commands like `pnpm lint`, `pnpm test`, `cargo test`, `cargo clippy`, `cargo fmt`, or direct script commands unless there is no matching task.

If a needed command does not have a task yet, add a new task to the appropriate Taskfile instead of introducing a raw command into docs, CI, scripts, or agent instructions.

When reporting verification steps, always mention the `task ...` command that was used.
