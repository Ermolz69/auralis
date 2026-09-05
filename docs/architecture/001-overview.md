# Architecture Overview

## Goal

Desktop application for preparing AI dubbing projects from YouTube or user-owned
videos.

## Main principle

React presentation, thin Tauri commands, Rust use-case orchestration, SQLite-backed
state, and adapter-owned external processes.

## Layers

- **React UI**: Client interface built with React, Vite, and FSD. Handles all presentation logic.
- **Tauri command bridge**: The thin integration layer (`src-tauri`) exposing commands and state to the frontend.
- **Application layer**: Orchestration layer (`application` crate). Coordinates the business workflow.
- **Domain layer**: Core entities (`domain` crate). Agnostic of UI or infrastructure.
- **Ports**: Interfaces and contracts (`ports` crate) isolating domain from external IO.
- **Job runtime**: The `jobs` crate schedules work, exposes snapshots, publishes lifecycle events, and coordinates cancellation.
- **Adapters**: Storage, Tauri, yt-dlp, FFmpeg/ffprobe, and model test doubles in `adapters-*` crates.
- **Storage**: SQLite stores projects, jobs, artifacts, the YouTube import journal, and outbox messages. Project files use managed storage keys.
- **External tools**: Bundled FFmpeg, ffprobe, and yt-dlp processes are invoked through adapters. Production local AI model runners are not wired yet.

## Implemented workflows

- Create, rename, list, open, and delete persistent projects.
- Import local media or create/resume/discard a project import from YouTube.
- Probe media streams and metadata with ffprobe.
- List YouTube subtitle tracks and import the selected track as a managed artifact.
- List and cancel jobs, synchronize job events in the UI, and recover interrupted work.
- Finalize and clean managed files through the SQLite transaction/outbox flow.
- Check for and install signed application updates in packaged builds.
- Run a mock dubbing pipeline for UI and orchestration validation.

## Target dubbing pipeline

The planned end-to-end media and AI pipeline retains the following sequence. The
production model-backed stages and final mux/export implementation are not wired;
the current runtime exposes a mock pipeline instead.

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
