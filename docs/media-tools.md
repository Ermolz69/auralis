# Media Tools Setup

Auralis uses external sidecar binaries (like `ffprobe` and `ffmpeg`) to process media files out of the main Tauri process.

## Local Development

For developers, `ffprobe` needs to be available either bundled inside `src-tauri/binaries/` or globally accessible via your system's `PATH`.

To view setup instructions, run:
```bash
task setup:media-tools
```

To verify your environment is correctly configured, run:
```bash
task media:doctor
```

## Testing the Pipeline

To manually verify that the adapter can probe media files (without running the full UI), use the Rust adapter example:
```bash
task media:probe -- /path/to/video.mp4
```

This bypasses Tauri and executes the `ffprobe` sidecar adapter natively.
