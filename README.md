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

```bash
task install
task setup:media-tools
task media:doctor
task dev
task check
```

## Documentation

See [docs/README.md](docs/README.md).

## Architecture

Thin Tauri shell, Rust orchestration, sidecar binaries for heavy media and AI processing.
