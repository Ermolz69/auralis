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
CI uses `pnpm/setup` to install both tools consistently across workflows.

```bash
task fe:install
task fe:install-update
task fe:lint
task fe:typecheck
task fe:test
task fe:build
task fe:setup:playwright
```

`task fe:install` uses the frozen lockfile. After changing dependency manifests,
run `task fe:install-update` and commit the updated `pnpm-lock.yaml`.
Dependency overrides and explicitly approved build scripts live in
`pnpm-workspace.yaml`; unreviewed dependency build scripts fail installation.

## Rust

```bash
task rs:fmt
task rs:clippy
task rs:test
task rs:check
```

## Quality

```bash
task q:file-size
task q:color-tokens
task q:storage-fallbacks
task q:runtime-println
task q:global
```
