# Runtime Data Layout

Auralis stores all runtime data under the operating system application data directory resolved by
Tauri for the `com.auralis.desktop` bundle identifier.

```text
<app-data>/
├── auralis.sqlite
├── auralis-translate.sqlite
├── .auralis-owner.lock
├── translation-runtime.json
├── translation-package-selection.json
├── translation-packages/
├── caster-runtime.json
├── tts-runtime.json
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

- `auralis.sqlite` owns projects, host jobs, artifacts, outbox, theme, pins,
  translation links, and speech attempt records. The separate
  `auralis-translate.sqlite` owns detailed translation runs, checkpoints,
  edits, and results; cross-database publication is reconciled through the
  [storage contract](../storage/production-storage.md).
- Runtime configuration and package-selection files appear only when their
  separately installed component is configured. They are not bundled model
  weights or evidence of completed synthesis.
- `.auralis-owner.lock` carries an exclusive OS lease before migrations and startup recovery.
- `projects/` contains only physical project files addressed by managed storage keys.
- `projects/.staging/` contains artifact files waiting for atomic finalization.
- `logs/` contains structured diagnostic logs with daily rotation, at most 30 owned files,
  16 MiB per file, and a 480 MiB total write budget. Existing oversized files are not truncated;
  they consume the budget until normal rotation removes them.
- `cache/workspaces/` contains temporary pipeline workspaces.

There is no per-project metadata file. In particular, `project.json` is not created because it
would duplicate SQLite state.

`src-tauri/src/bootstrap/paths.rs` owns the complete layout. Other bootstrap modules consume
`AppPaths` and must not independently resolve Tauri data, cache, or log directories.
