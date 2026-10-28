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
- **adapters-translate**: Bridges application translation ports to the independent
  Translate crates pinned under `modules/auralis-translate`. Owns strict file
  inspection, the Translate SQLite boundary, immutable result reconstruction,
  explicit package installation/selection and managed checked llama.cpp processes.
  Auralis application jobs and the outbox retain project/artifact ownership; core
  translation rules and model validation stay in the nested library workspace.
  This integration is experimental; see [its evidence](../translation/README.md).
- **adapters-caster**: Calls a separately installed Caster worker for project-scoped
  voice catalog, reference inspection, compatibility checks, and explicit manual
  cast revisions. Desktop bootstrap manages its use cases and enables the
  worker when the pinned runtime configuration and wheel manifest are valid. It
  does not synthesize speech or infer roles; see the
  [Caster record](../integration/caster-execution.md).
- **adapters-tts**: Provides a separately installed TTS worker client and
  voice/profile catalog. Desktop bootstrap currently composes the catalog use
  case and selector. The adapter also has model-free execution and result
  verification code, while desktop synthesis, managed publication, real-audio
  listening, and clean-install gates remain open; see the
  [TTS record](../integration/tts-execution.md).
- **adapters-tauri**: Maps job events and DTOs to Tauri and opens project workspaces through the native shell boundary.
- **src-tauri**: Composition root that resolves runtime paths, initializes observability and SQLite services, starts workers, registers Tauri plugins and commands, and performs graceful shutdown.
- **tools/glib-backport/regression**: Workspace regression harness for the
  pinned Linux GLib backport; it is not a product adapter.

Shared dependency versions used by multiple crates live in the root
`[workspace.dependencies]` table. `task q:workspace-dependencies` enforces that
policy.

## Sidecars & External Processes

The current packaged external executables are FFmpeg, ffprobe, and yt-dlp.

- Sidecars are **never** invoked directly from the frontend.
- The React UI dispatches a command to the Tauri layer, which delegates to an application use case. The use case calls a port implemented by the appropriate adapter.
- yt-dlp descendants are owned by a Windows Job Object or a Unix process group and lifetime monitor so application termination does not leave downloads running.
- Experimental translation uses an owned, separately installed llama.cpp child;
  the application release does not embed its runtime or weights. Future production
  ASR/TTS runners must follow the same adapter boundary; `adapters-model` still
  contains test doubles. Caster and TTS worker integrations use separate runtime
  configuration and are not part of the three bundled media executables.

The [durable dispatch contract](008-job-dispatch.md) covers the current queued
subtitle importer and its bounded CPU/network/disk/process reservations. The
[job runtime](007-job-runtime.md) covers cancellation and active ownership;
[outbox concurrency](009-outbox-concurrency.md) governs artifact finalization
and terminal project state. These contracts do not make the planned full
speech-generation pipeline available.
