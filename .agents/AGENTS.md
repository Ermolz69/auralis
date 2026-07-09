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

- IDs go into `id.rs`.
- Main structs/entities go into `entity.rs`.
- Status enums go into `status.rs`.
- Type/kind enums go into `kind.rs`.
- Module-specific errors go into `error.rs`.
- Progress/state details go into `progress.rs`.
- Tests go into `tests.rs`.
- `mod.rs` must not contain large structs, enums, business logic, or tests.
- `mod.rs` should only contain `mod ...`, `pub use ...`, and minimal module documentation.
- Do not duplicate types across files.
- Do not define the same struct, enum, or ID twice.
- Keep each file focused on one responsibility.

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

- `task check` — run the default full project check.
- `task check:frontend` — run frontend checks.
- `task check:rust` — run Rust checks.
- `task check:docs` — run documentation checks.
- `task check:quality:frontend` — run frontend quality checks.
- `task check:quality:docs` — run documentation quality checks.
- `task check:quality:security` — run security checks.
- `task check:all` — run all checks.
- `task ci` — run the CI-equivalent workflow.

Do not run commands like `pnpm lint`, `pnpm test`, `cargo test`, `cargo clippy`, `cargo fmt`, or direct script commands unless there is no matching task.

If a needed command does not have a task yet, add a new task to the appropriate Taskfile instead of introducing a raw command into docs, CI, scripts, or agent instructions.

When reporting verification steps, always mention the `task ...` command that was used.

# Architecture and code structure rules

## Core architecture

This project follows a layered / ports-and-adapters architecture.

Dependency direction must stay one-way:

```text
domain <- ports <- application <- adapters/src-tauri
```

Rules:

1. `domain` contains business entities, value objects, state transitions, and domain errors only.
2. `domain` must not depend on SQLite, Tauri, filesystem paths, yt-dlp, ffmpeg, UI DTOs, or async runtime details.
3. `ports` contains traits used by application use cases.
4. `application` contains use cases, workers, and orchestration logic.
5. `application` may depend on `domain` and `ports`, but must not depend on concrete adapters.
6. `adapters-*` crates implement ports.
7. `src-tauri` is the composition root: it wires concrete adapters into runtime services and exposes thin commands.
8. UI commands must not contain business logic.

## File organization

Large modules must be split by responsibility. Do not put rows, mappers, repositories, and business logic in one file.

For SQLite adapters use this structure:

```text
sqlite/
  <entity>_row.rs
  <entity>_mapper.rs
  <entity>_repository.rs
```

Use:

```text
row        = database shape
mapper     = conversion between row and domain
repository = SQL queries and port implementation
```

For cross-repository atomic writes use:

```text
sqlite/transaction_gateway.rs
```

Do not simulate transactions by calling multiple repositories one after another.

## Artifact and file handling

SQLite is the source of truth. The filesystem is an eventual side effect.

New managed artifacts must use `StorageKey`, not arbitrary `LocalPath`.

New file-producing flows must use:

```text
stage file
commit DB transaction
insert outbox message
finalize through outbox worker
mark artifact ready
```

Rules:

1. Long operations happen before DB transaction.
2. DB transaction must be short.
3. File finalization happens after commit through outbox.
4. Public artifact queries must return only `state = ready`.
5. UI must not receive legacy `LocalPath`.
6. `ArtifactStore` must not know about SQLite.
7. `ArtifactIndex` must not touch files.
8. Use cases coordinate storage/index/project/job changes through `TransactionGateway`.

## Outbox rules

All outbox actions must be idempotent.

`FinalizeStagedArtifact`:

```text
if final exists -> success
if staging exists -> move/copy staging to final -> success
if both missing -> fail/dead
```

`DeleteStorageKey`:

```text
if file missing -> success
if file exists -> delete
```

`DeleteProjectArtifactDir`:

```text
if directory missing -> success
if directory exists -> remove_dir_all
```

`DeleteTempPath` must only delete files inside allowed application temp roots.

## Use case rules

Use cases should be small vertical scenarios.

A use case may:

1. Load domain entities.
2. Validate domain transitions.
3. Call ports.
4. Build a `UnitOfWorkData`.
5. Execute one transaction through `TransactionGateway`.
6. Return application response DTOs.

A use case must not:

1. Run SQL directly.
2. Access Tauri state directly.
3. Build UI DTOs.
4. Delete files before DB commit.
5. Hold DB transactions during long downloads, probing, parsing, or copying.

## Tauri command rules

Tauri commands must stay thin.

A command may:

1. Parse command arguments.
2. Create or fetch a prebuilt use case from bootstrap/runtime services.
3. Call the use case.
4. Convert response to DTO.
5. Convert errors to command errors.

A command must not:

1. Contain business logic.
2. Contain pipeline logic.
3. Manually perform storage/index/repository sequences.
4. Know outbox internals.
5. Return unsafe local filesystem paths to the frontend.

## Testing rules

Every architectural boundary needs tests.

Minimum required tests for storage/outbox flows:

1. transaction writes project/job/artifact/outbox atomically.
2. public artifact list returns only ready artifacts.
3. outbox finalizes staged artifact and marks it ready.
4. outbox retry/backoff works.
5. repeated finalize is safe.
6. project delete enqueues deletion and does not delete files before commit.
7. `resolve_artifact_path` rejects `LocalPath`.
8. storage keys reject absolute paths and `..`.
