# Bundled Media Tools

Auralis ships pinned `ffmpeg`, `ffprobe`, and `yt-dlp` executables for every supported
desktop release target. Users do not need to install media tools or add them to `PATH`.

The tracked manifest at `tools/media-tools/manifest.json` records every version, immutable
release URL, SHA-256 digest, source revision, build revision, license, and target-specific
output name. Generated executables are ignored by Git and staged under
`src-tauri/binaries/` only after their integrity and reported versions are verified.
The Tauri resource list is an explicit allowlist, and preparation rejects unknown staging
entries while removing stale files from another supported target.

FFmpeg and ffprobe use the versioned Shaka static build for FFmpeg 8.1.2. yt-dlp uses the
official 2026.08.19 release. The application supplies the packaged FFmpeg location to
yt-dlp so formats with separate audio and video streams can be merged without a system
installation.

## Local development

Prepare the current host binaries once with:

```bash
task setup:media-tools
```

Subsequent runs reuse files only when their SHA-256 digests still match. Verify the staged
executables, licenses, provenance, and reported versions without downloading again:

```bash
task media:doctor
```

`tauri dev` and `tauri build` call the same preparation task automatically. A network
connection is required only when the pinned files are not already staged.

## CI and release enforcement

Repository quality checks validate the manifest and workflow wiring without downloading
large files. Native CI and release jobs then:

1. download the three executables for the runner target;
2. reject any SHA-256 or version mismatch;
3. build the real Tauri installer;
4. extract the generated app/MSI/Debian package;
5. verify that exactly one copy of every executable, license, provenance file, and notice
   is present and unchanged.

Windows x64, Linux x64, macOS x64, and macOS arm64 are required release targets. Linux
arm64 is also pinned for supported local or future CI builds.

The bundled FFmpeg build is GPL-3.0-or-later. Its license and provenance, including exact
source and build-script revisions, are included with the application. Release owners must
preserve the corresponding source and build inputs for every distributed version.

## Testing the adapter

To probe a media file without running the full UI:

```bash
task media:probe -- /path/to/video.mp4
```

This bypasses Tauri and executes the `ffprobe` sidecar adapter natively.
