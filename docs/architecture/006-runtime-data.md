# Runtime Data Layout

Auralis stores all runtime data under the operating system application data directory resolved by
Tauri for the `com.auralis.desktop` bundle identifier.

```text
<app-data>/
├── auralis.sqlite
├── projects/
│   ├── .staging/
│   └── <project-id>/
│       ├── source-video/
│       ├── downloaded-video/
│       ├── extracted-audio/
│       ├── original-subtitle/
│       ├── generated-transcript/
│       ├── translated-transcript/
│       ├── generated-speech-segment/
│       ├── mixed-audio/
│       ├── preview-video/
│       └── final-video/
├── logs/
└── cache/
    └── workspaces/
```

- `auralis.sqlite` is the only source of truth for projects, jobs, artifacts, and outbox state.
- `projects/` contains only physical project files addressed by managed storage keys.
- `projects/.staging/` contains artifact files waiting for atomic finalization.
- `logs/` contains structured diagnostic logs with daily rotation and a 30-file retention limit.
- `cache/workspaces/` contains temporary pipeline workspaces.

There is no per-project metadata file. In particular, `project.json` is not created because it
would duplicate SQLite state.

`src-tauri/src/bootstrap/paths.rs` owns the complete layout. Other bootstrap modules consume
`AppPaths` and must not independently resolve Tauri data, cache, or log directories.
