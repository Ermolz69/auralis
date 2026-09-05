# Rust Workspace Architecture

The backend is organized as a Rust workspace enforcing clean architecture boundaries.

## Crate Boundaries & Responsibilities

- **domain**: Contains pure business entities and rules. Strictly does not depend on Tauri, FFmpeg, or any external framework.
- **ports**: Defines interfaces for repositories, transactions, artifact and temporary storage, media tools, job scheduling, events, cancellation, recovery, and workspace access.
- **application**: Implements use cases, recovery, outbox processing, and workflow orchestration. Production code depends on `domain`, `ports`, and shared diagnostics, never concrete adapters.
- **jobs**: Implements the asynchronous job scheduler/runtime, snapshots, cancellation, and event publication behind port contracts.
- **common**: Contains cross-cutting diagnostic redaction shared by runtime crates.
- **adapters-storage**: Implements SQLite repositories and transactions, managed local artifacts, temporary workspaces, and in-memory test doubles.
- **adapters-ytdlp**: Implements YouTube validation, metadata, media download/resume, and subtitle retrieval.
- **adapters-ffmpeg**: Implements ffprobe-backed media inspection. Extraction and muxing currently exist only as mock adapter behavior for the mock pipeline.
- **adapters-model**: Contains mock ASR/TTS behavior; a production model runtime is not implemented.
- **adapters-tauri**: Maps job events and DTOs to Tauri and opens project workspaces through the native shell boundary.
- **src-tauri**: Composition root that resolves runtime paths, initializes observability and SQLite services, starts workers, registers Tauri plugins and commands, and performs graceful shutdown.

Shared dependency versions used by multiple crates live in the root
`[workspace.dependencies]` table. `task q:workspace-dependencies` enforces that
policy.

## Sidecars & External Processes

The current packaged external executables are FFmpeg, ffprobe, and yt-dlp.

- Sidecars are **never** invoked directly from the frontend.
- The React UI dispatches a command to the Tauri layer, which delegates to an application use case. The use case calls a port implemented by the appropriate adapter.
- yt-dlp descendants are owned by a Windows Job Object or a Unix process group and lifetime monitor so application termination does not leave downloads running.
- Future production model runners must follow the same adapter boundary; the current model adapter is not a production sidecar integration.
