# Auralis agent entrypoint

Read the [project documentation index](docs/README.md) and the relevant contract
before changing code. The [architecture overview](docs/architecture/001-overview.md)
maps current ownership; [Rust workspace](docs/architecture/003-rust-workspace.md)
and [frontend boundaries](docs/architecture/002-frontend-fsd.md) identify the
implementation layers. Read [runtime data](docs/architecture/006-runtime-data.md),
[durable dispatch](docs/architecture/008-job-dispatch.md), and
[outbox concurrency](docs/architecture/009-outbox-concurrency.md) for persistent
work or storage changes.

Use the repository [Taskfile commands](docs/taskfile/001-commands.md) for setup
and checks. Pick the focused check for the changed boundary, then the required
[CI gate](docs/ci/001-quality-gates.md). Documentation checks are
`task check:docs` and `task check:quality:docs`; integration guides also use
`task docs:integration`. Checks do not imply model, native, or clean-install
acceptance; record those separately with the actual candidate and assets.

The React desktop lives in `apps/desktop`; Tauri composition and typed commands
live in `src-tauri`; Rust use cases, ports, domain rules, runtime, and concrete
adapters live in `crates`. Keep the existing direction of dependencies and the
typed IPC boundary. The [desktop contributor guide](apps/desktop/README.md)
explains UI changes. The [storage contract](docs/storage/production-storage.md)
explains SQLite, managed artifacts, and migration ownership.

The Translate library is an independently owned repository pinned under
`modules/auralis-translate`. Change the independent library in its own checkout;
advance this repository's gitlink only during an explicitly reviewed integration.
The [Translate status](docs/translation/README.md) and
[module integration plan](docs/integration/README.md) separate current behavior
from planned acceptance. Preserve original subtitle sources, durable checkpoints,
project ownership, and published artifact rules.

Follow the user's instructions and any scoped assignment. For the current
[maintenance packet](docs/maintenance/README.md), use its
[agent contract](docs/maintenance/AGENT_CONTRACT.md), branch ownership, and
acceptance report. Do not treat planned goals as implemented work.
