# AI YouTube Dubbing Desktop App

## Stack

- Tauri v2
- Rust workspace
- React + Vite
- Tailwind
- Storybook
- Taskfile
- GitHub Actions

## Development

Run commands from the repository root. Install Node.js 24, the pnpm version pinned in
`package.json`, and Task first. Full desktop development uses Rust 1.95.0 from
`rust-toolchain.toml` and also requires the platform's Tauri build dependencies.

Set up dependencies and the Playwright Chromium runtime before running checks:

```bash
task install:all
task fe:setup:playwright
task sec:setup:rust
task setup:media-tools
task media:doctor
```

`task check`, `task check:frontend`, and `task ci` include browser tests and require
Chromium. On Linux, use `task fe:setup:playwright:ci` instead of
`task fe:setup:playwright` to install its system libraries too; package installation
requires elevated permissions. Repeat browser setup after upgrading Playwright.
Dependency installation alone does not install browsers.

The full `task check` and `task ci` suites also require the Rust security tools installed
by `task sec:setup:rust`. Frontend-only and docs-only checks do not require these tools.
See [dependency security](docs/ci/010-rust-security.md) for gate scope and remediation status.

Start development or run the full check suite:

```bash
task dev
task check
```

### Scoped installation

- `task install:frontend`: frontend and documentation dependencies only; no Rust required.
- `task install:rust`: locked Rust dependencies only; no Node/pnpm required.
- `task install:all`: both sets of dependencies. `task install` remains a compatible alias.

For docs-only work, run `task install:frontend` followed by `task check:docs` and
`task check:quality:docs`; neither Rust nor Playwright is needed. For frontend-only
checks, add `task fe:setup:playwright` before `task check:frontend`.

Run the production E2E suite independently with `task fe:e2e`. It builds the desktop
frontend and executes the 20 main user journeys in Chromium against a stateful mocked
Tauri boundary.

Installed builds check signed application updates from GitHub Releases. The Settings page
shows the current and available versions, release notes, download progress, and performs the
restart after verification. Release maintainers must configure the updater key pair described
in [the signing guide](docs/release/002-signing.md); no updater backend is required.

Versions of Rust dependencies used by more than one workspace crate belong in the root
`[workspace.dependencies]` table. Member manifests inherit them with `workspace = true`
and declare only crate-specific features. `task q:workspace-dependencies` checks this
policy and is included in the global quality gate.

## Documentation

See [docs/README.md](docs/README.md).

## Architecture

Thin Tauri shell, Rust orchestration, sidecar binaries for heavy media and AI processing.
