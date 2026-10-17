# Auralis

Auralis is a Tauri desktop application for preparing AI dubbing projects from
YouTube links or local media. The current product supports persistent projects,
local and YouTube source import, media inspection, YouTube subtitle selection,
durable job tracking, crash-safe storage, and signed application updates. It also
has an experimental subtitle translation workflow, a Caster voice catalog with
manual role assignment, and a TTS voice/profile catalog. The TTS selector does
not start synthesis. The complete speech-generation and final media pipeline
remains a target workflow; see the [architecture overview](docs/architecture/001-overview.md)
and [integration status](docs/integration/README.md).

## Stack

- Tauri v2
- Rust workspace
- React + Vite
- Tailwind CSS
- Storybook
- SQLite with an outbox-backed artifact store
- Bundled FFmpeg, ffprobe, and yt-dlp tools
- Taskfile
- GitHub Actions

## Development

Run commands from the repository root. Install Node.js 24, the pnpm version pinned in
`package.json`, and Task first. Full desktop development uses Rust 1.95.0 from
`rust-toolchain.toml` and also requires the platform's Tauri build dependencies.

Setup commands change the local dependency and tool environment; run only the
ones needed for the selected check. For a first, model-free documentation check:

```bash
task install:frontend
task check:docs
task check:quality:docs
```

For full desktop and browser development, install the required dependencies and
Playwright Chromium runtime explicitly:

```bash
task install:all
task fe:setup:playwright
task sec:setup:rust
task setup:media-tools
task media:doctor
```

`task check`, `task check:pr`, `task check:frontend`, and `task ci` include browser tests and require
Chromium. On Linux, use `task fe:setup:playwright:ci` instead of
`task fe:setup:playwright` to install its system libraries too; package installation
requires elevated permissions. Repeat browser setup after upgrading Playwright.
Dependency installation alone does not install browsers.

The full `task check`, pull-request `task check:pr`, and `task ci` suites also require the Rust security tools installed
by `task sec:setup:rust`. Frontend-only and docs-only checks do not require these tools.
See [dependency security](docs/ci/010-rust-security.md) for gate scope and remediation status.

Start the frontend, start the complete desktop application, or reproduce the PR checks:

```bash
task dev
task desktop:dev
task check:pr
```

`task dev` starts the Vite frontend server. The Tauri development runner is
configured to call `task desktop:before-dev`, which prepares the bundled media
tools before starting that server; launch it directly with `task desktop:dev`.
Use `task check` for the extended local/release suite, including the optimized
GLib regression.

### Scoped installation

- `task install:frontend`: frontend and documentation dependencies only; no Rust required.
- `task install:rust`: locked Rust dependencies only; no Node/pnpm required.
- `task install:all`: both sets of dependencies. `task install` remains a compatible alias.

For frontend-only checks, add `task fe:setup:playwright` before
`task check:frontend`. Choose narrower checks from the
[Taskfile guide](docs/taskfile/001-commands.md) while iterating.

Run the production E2E suite independently with `task fe:e2e`. It builds the desktop
frontend and executes the main user journeys in Chromium against a stateful mocked
Tauri boundary.

The reusable UI and complete product states are documented in the branded Auralis
Storybook. Start it with `task fe:storybook:dev`, or run the focused metadata,
browser, accessibility, and static-build gate with `task check:storybook`.

On Windows, run `task desktop:e2e:native` for the real desktop acceptance path. It
builds and launches an invisible Tauri application against an isolated temporary
data directory, then verifies React → IPC → Rust → SQLite → managed filesystem.
The manually triggered `Tauri Build` workflow runs this scenario on its Windows job;
it is intentionally excluded from routine pull-request checks because it performs a
full native release build.

Installed builds check signed application updates from GitHub Releases. The Settings page
shows the current and available versions, release notes, download progress, and performs the
restart after verification. Release maintainers must configure the updater key pair described
in [the signing guide](docs/release/002-signing.md); no updater backend is required.

Versions of Rust dependencies used by more than one workspace crate belong in the root
`[workspace.dependencies]` table. Member manifests inherit them with `workspace = true`
and declare only crate-specific features. `task q:workspace-dependencies` checks this
policy and is included in the global quality gate.

## Documentation

Start with [docs/README.md](docs/README.md). Agents have a short repository
entrypoint in `AGENTS.md`; the [desktop contributor guide](apps/desktop/README.md)
locates frontend ownership and checks.

## Architecture

The React UI calls typed, thin Tauri commands. Rust application use cases coordinate
the domain, ports, durable job runtime, SQLite storage, and concrete adapters.
FFmpeg, ffprobe, and yt-dlp run as managed external processes. When separately
installed, the Caster worker supplies catalog and manual assignment operations;
the TTS worker supplies a catalog. Experimental translation uses a separately
installed checked model runtime. Model-backed ASR, desktop TTS synthesis/publication, and
final media assembly remain outside accepted production composition.
