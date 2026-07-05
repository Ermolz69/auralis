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
