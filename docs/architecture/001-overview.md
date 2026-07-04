# Architecture Overview

## Goal

Desktop application for AI dubbing of YouTube or user-owned videos.

## Main principle

Thin Tauri shell, Rust orchestration, external sidecar binaries for heavy processing, React UI.

## Layers

- **React UI**: Client interface built with React, Vite, and FSD. Handles all presentation logic.
- **Tauri command bridge**: The thin integration layer (`src-tauri`) exposing commands and state to the frontend.
- **Application layer**: Orchestration layer (`application` crate). Coordinates the business workflow.
- **Domain layer**: Core entities (`domain` crate). Agnostic of UI or infrastructure.
- **Ports**: Interfaces and contracts (`ports` crate) isolating domain from external IO.
- **Adapters**: Concrete implementations for executing tasks (`adapters-*` crates).
- **Sidecars**: External heavyweight binaries (e.g., FFmpeg, yt-dlp, local AI models) invoked through adapters so they do not block the main process.

## Pipeline

The end-to-end media and AI pipeline executes in the following sequence:

```text
validate_url
-> inspect_subtitles
-> fetch_metadata
-> download_video_and_audio
-> extract_or_generate_transcript
-> segment_transcript
-> prepare_dubbing_script
-> synthesize_segments
-> postprocess_audio
-> mux_new_audio_track
-> export_result
```
