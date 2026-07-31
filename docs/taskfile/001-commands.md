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

```bash
task fe:lint
task fe:typecheck
task fe:test
task fe:build
task fe:setup:playwright
```

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
