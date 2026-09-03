# Taskfile Commands

## Main commands

```bash
task install
task dev
task check
task check:all
task ci
```

## Frontend

Use Node.js 24 and the pnpm version pinned in the root `package.json`.
The shared CI bootstrap (`.github/actions/bootstrap/action.yml`) uses `pnpm/setup` to install both tools consistently across workflows.
`actions/setup-node` handles only dependency caching; it does not select a second
Node version. This avoids the pnpm setup action's deprecated cache-pruning hook.

```bash
task fe:install
task fe:install-update
task fe:lint
task fe:typecheck
task fe:test
task fe:build
task fe:setup:playwright
task fe:setup:playwright:ci
```

`task fe:install` uses the frozen lockfile. After changing dependency manifests,
run `task fe:install-update` and commit the updated `pnpm-lock.yaml`.
Dependency overrides and explicitly approved build scripts live in
`pnpm-workspace.yaml`; unreviewed dependency build scripts fail installation.

`task fe:setup:playwright` installs Chromium for local browser tests. CI uses
`task fe:setup:playwright:ci` to also install Chromium's system dependencies.
On Linux, system package installation requires elevated permissions.

## Rust

```bash
task rs:fetch
task rs:fmt
task rs:clippy
task rs:test
task rs:check
```

`task rs:fetch` fetches dependencies with `--locked`, without changing `Cargo.lock`.

## Quality

```bash
task q:file-size
task q:color-tokens
task q:storage-fallbacks
task q:runtime-println
task q:ci-bootstrap
task q:global
```
