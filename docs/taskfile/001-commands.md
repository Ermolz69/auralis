# Taskfile Commands

## Main commands

```bash
task install:all
task fe:setup:playwright
task sec:setup:rust
task dev
task check
task check:all
task ci
```

## Installation scope

- `task install:frontend` delegates to `task fe:install` and installs only the frozen pnpm dependencies needed by frontend and documentation jobs.
- `task install:rust` delegates to `task rs:fetch` and fetches only locked Rust dependencies; it does not require Node or pnpm.
- `task install:all` runs both component installers. `task install` is a backward-compatible alias for this task.

`task ci` starts with `task install:all`. Browser setup remains an explicit prerequisite
for `task ci`, `task check`, and `task check:frontend`, not part of dependency installation.
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

## Rust

```bash
task install:rust
task rs:fmt
task rs:clippy
task rs:test
task rs:check
```

`task install:rust` (also available as `task rs:fetch`) fetches dependencies with `--locked`, without changing `Cargo.lock`.

## Quality

```bash
task q:file-size
task q:color-tokens
task q:storage-fallbacks
task q:runtime-println
task q:ci-bootstrap
task q:global
```

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
