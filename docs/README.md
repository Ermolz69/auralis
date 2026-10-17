# Project Documentation

For a first local check, follow the [project README](../README.md) and
[Taskfile guide](./taskfile/001-commands.md). For implementation ownership,
start with the [architecture overview](./architecture/001-overview.md) and
[Rust adapter map](./architecture/003-rust-workspace.md). Agents also have a
short entrypoint at the repository root in `AGENTS.md`.

## Sections

- [Architecture overview](./architecture/001-overview.md)
- [Technology stack](./architecture/000-stack.md)
- [Frontend FSD boundaries](./architecture/002-frontend-fsd.md)
- [Rust workspace](./architecture/003-rust-workspace.md)
- [Design system](./architecture/004-design-system.md)
- [Storybook conventions](./architecture/005-storybook-conventions.md)
- [Runtime data layout](./architecture/006-runtime-data.md)
- [Job runtime and cancellation](./architecture/007-job-runtime.md)
- [Durable job dispatch and resource ownership](./architecture/008-job-dispatch.md)
- [Translation review decisions](./architecture/008-translation-review-decision.md)
- [Project state and outbox concurrency](./architecture/009-outbox-concurrency.md)
- [Production storage contract](./storage/production-storage.md)
- [Translate integration status](./translation/README.md)
- [Module integration architecture and goal prompts](./integration/README.md)
- [Caster integration evidence](./integration/caster-execution.md)
- [TTS integration evidence](./integration/tts-execution.md)
- [Bundled media tools](./media-tools.md)
- [CI and quality gates](./ci/001-quality-gates.md)
- [Pinned runner images](./ci/002-runner-images.md)
- [Taskfile commands](./taskfile/001-commands.md)
- [Release workflow](./release/001-release-workflow.md)
- [Production signing](./release/002-signing.md)
- [Maintainability assignments and review](./maintenance/README.md)

The architecture documents distinguish implemented capabilities from the planned
speech-generation and media pipeline. The current product scope and setup commands
are summarized in the [project README](../README.md).

Use [runtime data](./architecture/006-runtime-data.md) and the
[storage contract](./storage/production-storage.md) for database and managed-file
changes. [Durable dispatch](./architecture/008-job-dispatch.md) covers admission,
resource leases and managed cancellation; [outbox concurrency](./architecture/009-outbox-concurrency.md)
covers terminal project state and artifact finalization. The
[CI guide](./ci/001-quality-gates.md) separates frontend, Rust, documentation,
native and release checks. Dated execution records remain evidence for their
recorded candidate, not blanket approval of later code.

## Storybook

Storybook is the interactive reference for foundations, reusable components, and
complete product states. Start with the [design-system contract](./architecture/004-design-system.md)
and [Storybook conventions](./architecture/005-storybook-conventions.md).

- **Interactive development**: `task fe:storybook:dev`
- **Complete focused verification**: `task check:storybook`
- **Metadata-only verification**: `task q:storybook-contract`
- **Static verification**: `task fe:storybook`
- **Published Pages build**: `task fe:storybook-pages`
- **Published catalog**: [Auralis Storybook](https://ermolz69.github.io/auralis/docs/storybook/)
- **Deployment flow**: `.github/workflows/storybook-pages.yml` deploys after
  changes reach `main` or when started manually.
- **Generated files**: Static output is not committed. GitHub Actions builds and
  uploads it as a Pages artifact.

The focused check validates the catalog contract, runs every story and
accessibility check in Chromium, and builds the production-static site. The full
frontend and frontend-quality suites already include the browser and metadata
portions respectively, so pull-request CI does not duplicate the Pages build.
