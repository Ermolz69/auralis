# Runtime Data Layout

Auralis stores runtime files under the operating system's application data directory for the
`com.auralis.app` bundle identifier. Files are not written next to the installed executable or to
the current working directory.

```text
com.auralis.app/
├── auralis.sqlite
├── artifacts/
├── logs/
└── cache/
    └── workspaces/
```

- `auralis.sqlite` contains persistent application state.
- `artifacts/` contains user-owned media artifacts managed by the application.
- `logs/` contains structured diagnostic logs with daily rotation and a 30-file retention limit.
- `cache/workspaces/` contains recoverable processing workspaces and temporary pipeline files.

The exact root is platform-specific and is resolved by Tauri. Keeping the existing database and
artifact paths at the root preserves compatibility with installations created before this layout
was centralized.
