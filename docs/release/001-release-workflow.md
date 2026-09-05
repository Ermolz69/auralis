# Release Workflow

## Why is it needed

Ensures a predictable, safe, and automated build of production artifacts for all platforms (Windows, macOS, Linux) without human error.

## What does it forbid

It strictly forbids mixing CI (for developers) and CD (publishing for users). It also forbids creating official releases without an attached version git tag. Manual production builds on local machines are strictly prohibited.

## Where does it run

Only on GitHub Actions (`release.yml`) and strictly only upon pushing a git tag (`app-v*`).
The tag must exactly equal `app-v<package.json version>` or the release stops before signing.

## Shared check environment

Before building release artifacts, `release.yml` calls `.github/workflows/full-checks.yml`. This reusable workflow checks out the source and uses `.github/actions/bootstrap/action.yml` to install Node/pnpm, Rust, Task, frozen frontend dependencies, locked Rust dependencies, pinned Rust security auditors, Linux Tauri/GTK libraries, and Playwright Chromium with its system dependencies. It then runs `task check`, including the npm and Rust security gates. Cargo validation disables bundle-resource expansion during this check; the platform build prepares and verifies the real sidecars immediately afterward.

Each native build then runs `task media:prepare`. This downloads only the target-specific
FFmpeg, ffprobe, and yt-dlp executables declared in `tools/media-tools/manifest.json`,
verifies their SHA-256 digests and reported versions, and stages their licenses and source
provenance. After Tauri creates the installer, `task media:bundle:verify` extracts the
platform package and rejects missing, duplicate, modified, or stale media-tool resources.
Installer generation is explicitly enabled in `tauri.conf.json`; producing only the bare
application executable is not considered a successful release build.
On Windows the workflow installs the generated current-user NSIS package into an isolated
location; MSI remains covered by build, content, and signature verification. On macOS it
copies the generated app bundle into an isolated location. Both smoke paths launch the
installed executable, verify that it remains alive through startup, terminate it, and remove
the test installation.
Production tags additionally run focused SQLite and YouTube recovery/process-ownership
tests on Windows and macOS and require the signing configuration described in
`docs/release/002-signing.md`. Windows Authenticode and Apple signing/notarization are
verified after bundle creation; missing credentials are a hard release failure.

The application updater does not require a separate backend. Installed builds request the
static manifest at
`https://github.com/Ermolz69/auralis/releases/latest/download/latest.json`. Each platform
build produces Tauri v2 updater bundles and mandatory `.sig` files. The publish job combines
their signatures into `latest.json`; the manifest points directly to the assets attached to
the matching `app-v<version>` GitHub Release.

## Atomic publication

The platform matrix has read-only repository permissions and never creates a GitHub Release.
Each Windows, macOS, and Linux runner builds a fixed package set, verifies its signatures,
bundled media tools, and install/launch smoke where supported, and then uploads an immutable
workflow artifact. GitHub validates the artifact digest again when the publish job downloads
it.

The publish job depends on the complete build matrix and is the only job with
`contents: write`. It rejects a missing or duplicate package or updater signature, flattens
the verified assets, generates `latest.json` and `SHA256SUMS.txt`, and only then creates one
draft GitHub Release.
Consequently, a build, signing, package-content, or launch failure on any platform prevents
release creation entirely.

Draft releases are intentionally invisible to the `/releases/latest/` updater endpoint. A
maintainer reviews the draft and publishes it in GitHub; only that final GitHub action makes
the version discoverable by installed clients. Prereleases are not used as the stable update
channel.

Pull requests use selective frontend, Rust, documentation, repository-policy, and
dependency-security gates. They do not repeat the full release check or build three
native installers. The `Tauri Build` workflow remains available for explicit manual
pre-release package testing. Write permission is scoped to the final production-tag
publish job, which starts only after the full check and every platform build succeed.

Release and manual desktop build jobs use the same bootstrap with Node and Rust enabled but skip browser installation. Linux native packages are installed only on Linux runners; Windows and macOS retain their platform-specific build behavior.

## How to fix the error

If the release pipeline fails:

1. Check the `release.yml` logs in GitHub Actions.
2. Fix the error in the code (in the `main` or `develop` branch).
3. If publication never started, fix the failure and create a new patch-version tag. Published
   release tags must not be reused.
4. If the final publish command itself failed after creating an incomplete draft, delete only
   that draft release, keep the immutable source tag, fix the workflow, and publish a new patch
   version.

## When can an exception be made

There are no exceptions. Official releases must always go through this workflow.

## Who approves the exception

Tech Lead or DevOps Engineer in case of a complete CI infrastructure failure.
