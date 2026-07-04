# Rust Workspace Architecture

The backend is organized as a Rust workspace enforcing clean architecture boundaries.

## Crate Boundaries & Responsibilities

- **domain**: Contains pure business entities and rules. Strictly does not depend on Tauri, FFmpeg, or any external framework.
- **ports**: Defines the interfaces (traits) required by the application, establishing an abstraction layer over I/O and external systems.
- **application**: Orchestrates the use cases and workflow. It depends only on `domain` and `ports`. All new high-level business logic must be placed here.
- **adapters**: Concrete implementations of the `ports`. This is exactly where external integrations are located:
  - `adapters-ytdlp`: Logic for video and audio downloading.
  - `adapters-ffmpeg`: Logic for segmenting and muxing media.
  - `adapters-model`: Local AI model integrations (ASR, TTS).
- **src-tauri**: A thin bootstrap layer. Its sole responsibility is to launch the Tauri app, register IPC commands, and inject adapters into the application layer.

## Sidecars & External Processes

Heavy computational tasks are offloaded to external sidecar binaries (e.g., FFmpeg, Python AI runners).

- Sidecars are **never** invoked directly from the frontend.
- The React UI dispatches a command to the Tauri bootstrap layer, which delegates to the `application` orchestration. The `application` layer then calls the appropriate `adapter` that securely manages the sidecar execution.
