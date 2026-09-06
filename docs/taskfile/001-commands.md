# Taskfile Commands

## Main commands

```bash
task install:all
task fe:setup:playwright
task sec:setup:rust
task setup:media-tools
task media:doctor
task dev
task desktop:dev
task desktop:e2e:native
task check:pr
task check
task check:all
task ci
```

`task dev` starts the Vite frontend server. `task desktop:dev` starts the native
Tauri application; its configured development hook
uses `task desktop:before-dev` to prepare media tools before starting the same
server. `task desktop:bundle` builds native installers and verifies their bundled
media resources.

`task desktop:e2e:native` is the Windows-only real desktop acceptance test. It
generates a small video with the pinned FFmpeg binary, compiles a hidden Tauri test
application, drives the production IPC commands from React, and validates the
resulting SQLite and managed-file state. Its data lives under a unique temporary
directory and is removed after the run. Set `AURALIS_NATIVE_E2E_KEEP_TEMP=1` only
when the isolated state must be retained for diagnosis.

## Installation scope

- `task install:frontend` delegates to `task fe:install` and installs only the frozen pnpm dependencies needed by frontend and documentation jobs.
- `task install:rust` delegates to `task rs:fetch` and fetches only locked Rust dependencies; it does not require Node or pnpm.
- `task install:all` runs both component installers. `task install` is a backward-compatible alias for this task.

`task ci` starts with `task install:all` and then runs `task check:pr`, the local
equivalent of all required pull-request gates. Browser setup remains an explicit prerequisite
for `task ci`, `task check:pr`, `task check`, and `task check:frontend`, not part of dependency installation.
Frontend/docs CI jobs select `task install:frontend` through the shared bootstrap and
do not fetch Rust dependencies. Docs-only jobs also skip browser setup.

## Frontend

Use Node.js 24 and the pnpm version pinned in the root `package.json`.
The shared CI bootstrap (`.github/actions/bootstrap/action.yml`) uses `pnpm/setup` to install both tools consistently across workflows.
`actions/setup-node` handles only dependency caching; it does not select a second
Node version. This avoids the pnpm setup action's deprecated cache-pruning hook.

```bash
task install:frontend
task fe:install-update
task fe:lint
task fe:typecheck
task fe:test
task fe:build
task fe:bundle:check
task fe:smoke
task fe:e2e
task fe:storybook
task fe:storybook:dev
task fe:storybook:check
task check:storybook
task fe:setup:playwright
task fe:setup:playwright:ci
```

`task install:frontend` (also available as `task fe:install`) uses the frozen lockfile. After changing dependency manifests,
run `task fe:install-update` and commit the updated `pnpm-lock.yaml`.
Dependency overrides and explicitly approved build scripts live in
`pnpm-workspace.yaml`; unreviewed dependency build scripts fail installation.

`task fe:build` enforces production bundle budgets and prints an analysis of the
largest modules. `task fe:smoke` requires that build and Chromium; it tests the
production output under the desktop CSP with a mocked IPC boundary. Both checks
are included in `task check:frontend`. See [bundle and CSP policy](../ci/011-desktop-bundle-and-csp.md).

`task fe:setup:playwright` installs Chromium for local browser tests. CI uses
`task fe:setup:playwright:ci` to also install Chromium's system dependencies.
On Linux, system package installation requires elevated permissions.

`task fe:e2e` builds the production frontend and runs 30 browser journeys. Each
journey prints a stable ID and area. To isolate a failure, pass its ID, for
example `task fe:e2e -- --test-name-pattern=E2E-027`. The run writes JUnit and
failure diagnostics to `apps/desktop/e2e-results`; CI uploads this directory even
when the frontend job fails.

`task fe:storybook:dev` starts the interactive Auralis design-system catalog on
port 6006. `task fe:storybook` creates its static production build.
`task check:storybook` is the complete focused gate: it validates the catalog
metadata, runs every component story and accessibility check in Chromium, and
builds the static output. The metadata contract requires a unique `Design System`
or `Product` hierarchy, Autodocs, a component description, and at least one example
per story file. The faster metadata-only check is available as
`task q:storybook-contract` and is included in the frontend quality suite.

## Rust

```bash
task install:rust
task rs:fmt
task rs:clippy
task rs:test
task rs:check
task rs:pr
```

`task install:rust` (also available as `task rs:fetch`) fetches dependencies with `--locked`, without changing `Cargo.lock`.
`task rs:pr` runs dependency-policy verification, formatting, Clippy, and workspace
tests. The standalone `task rs:check` remains available for diagnosis; it is not
repeated after Clippy in the default PR path.

## Quality

```bash
task q:file-size
task q:color-tokens
task q:storybook-contract
task q:duplicate-code
task q:desktop-policies
task q:ipc-contract
task q:release-metadata
task q:workspace-dependencies
task q:storage-fallbacks
task q:runtime-println
task q:ci-bootstrap
task q:global
```

`task q:desktop-policies` covers bundle/CSP regressions, IPC contract parity,
release metadata, and release smoke-tool validation. `task q:global` adds media
manifest checks, SQLite-only dependency verification, GLib provenance checks,
formatting, runtime output policy, storage fallbacks, and CI bootstrap validation.

## Documentation

```bash
task check:docs
task check:quality:docs
```

`task check:docs` runs fixture tests, verifies that mandatory project and design
documents exist, validates relative Markdown links, and applies Markdown linting.
`task check:quality:docs` verifies Markdown formatting without rewriting files.

## Dependency security

```bash
task sec:setup:rust
task sec:audit:npm
task sec:audit:rust
task sec:deny:rust
task check:quality:security
```

`task sec:setup:rust` installs pinned `cargo-audit` and `cargo-deny` versions using
locked tool dependencies. CI installs the same versions as prebuilt binaries through
the shared bootstrap. Install these tools before running `task check` or `task ci`.
Component-only frontend/docs checks do not need Rust or these tools.

`task check:quality:security` (also `task sec:audit`) runs all three security checks,
including development dependencies. `task sec:deny:rust` checks advisories and sources;
license and duplicate-version policy are outside this gate's scope.

Inspect dependency paths with `task rs:tree -- --invert <crate> --target all`.
For reviewed, targeted lockfile updates, use
`task rs:update -- --package <crate> --precise <version>` and rerun checks.

## Media tools

```bash
task media:check
task media:prepare
task media:verify
task media:bundle:verify
task media:probe -- /path/to/video.mp4
```

`task media:check` validates metadata and repository wiring without downloads.
`task media:prepare` downloads and stages the pinned tools when necessary, and
`task media:verify` checks the staged executables. `task media:doctor` aliases the
staged verification flow.

## Release validation

```bash
task release:smoke:check
task release:tag:verify TAG=app-v0.1.0
task release:signing:preflight PLATFORM=windows
task release:signature:verify
task release:assets:validate PLATFORM=windows
```

Release tasks are normally orchestrated by GitHub Actions. Signing, package
installation, and publication tasks require the platform artifacts and credentials
described in the release documentation.
