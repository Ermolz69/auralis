# 000-stack: Technology Stack

## 1. Context

This document records the production stack and the boundary between the current
implementation and planned media/model capabilities.

## 2. Core Decisions

### 2.1. Frontend

- **Framework**: React + Vite + TypeScript
- **Styling**: Tailwind CSS v4 with semantic tokens defined in `theme.css`
- **UI Documentation**: Storybook
- **Architecture Methodology**: Feature-Sliced Design (FSD)

### 2.2. Desktop Shell (Tauri)

- **Framework**: Tauri v2
- **Role of `src-tauri`**: Composition root for runtime paths, observability,
  SQLite-backed services, workers, Tauri plugins, lifecycle shutdown, and thin IPC
  commands. Business rules remain in the domain and application crates.

### 2.3. Backend & Heavy Processing

- **Core backend**: Rust workspace using ports-and-adapters boundaries.
- **Persistence**: SQLite is the production source of truth. Managed files are
  finalized by an idempotent outbox worker.
- **Current external tools**: Pinned FFmpeg, ffprobe, and yt-dlp executables are
  bundled with desktop builds and launched only through Rust adapters.
- **Model processing**: `adapters-model` contains ASR/TTS test doubles. A separate
  installed, checked llama.cpp runtime supports experimental subtitle translation;
  its language and delivery gates remain open. `adapters-tts` currently exposes
  the installed voice/profile catalog and a model-free worker transport candidate,
  but desktop synthesis and publication are incomplete. `adapters-caster` exposes
  installed voice and manual assignment operations without model inference.
  Production ASR and the complete speech/media pipeline remain planned work.

### 2.4. Tooling & DevOps

- **Task Runner**: Taskfile is the repository entrypoint for local and CI commands.
- **CI/CD**: GitHub Actions
- **Release**: Native Windows, macOS, and Linux packages with bundled media tools,
  production signing gates, and Tauri updater artifacts.
- **Documentation**: Markdown stored in `docs/`, plus repository and desktop READMEs.

## 3. Consequences

By adopting this stack:

- The frontend remains fast and strictly focused on UI concerns using a modern React ecosystem.
- Tauri remains the composition and native-integration boundary, while application
  orchestration stays in Rust use cases.
- Durable state and managed artifacts remain recoverable across process restarts.
- External media processes are owned and terminated by the desktop runtime.
- FSD ensures the frontend codebase remains maintainable and decoupled as it grows.
