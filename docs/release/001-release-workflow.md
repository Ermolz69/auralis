# Release Workflow

## Why is it needed

Ensures a predictable, safe, and automated build of production artifacts for all platforms (Windows, macOS, Linux) without human error.

## What does it forbid

It strictly forbids mixing CI (for developers) and CD (publishing for users). It also forbids creating official releases without an attached version git tag. Manual production builds on local machines are strictly prohibited.

## Where does it run

Only on GitHub Actions (`release.yml`) and strictly only upon pushing a git tag (`app-v*`).

## Shared check environment

Before building release artifacts, `release.yml` calls `.github/workflows/full-checks.yml`. This reusable workflow checks out the source and uses `.github/actions/bootstrap/action.yml` to install Node/pnpm, Rust, Task, frozen frontend dependencies, locked Rust dependencies, pinned Rust security auditors, Linux Tauri/GTK libraries, and Playwright Chromium with its system dependencies. It then runs `task check`, including the npm and Rust security gates.

CI also calls this full-check workflow for changes to CI, release, or repository tooling configuration. These PR checks have read-only repository permissions and do not create tags or publish releases. Write permission is scoped to the release build job, which starts only after the full check succeeds.

Release and manual desktop build jobs use the same bootstrap with Node and Rust enabled but skip browser installation. Linux native packages are installed only on Linux runners; Windows and macOS retain their platform-specific build behavior.

## How to fix the error

If the release pipeline fails:

1. Check the `release.yml` logs in GitHub Actions.
2. Fix the error in the code (in the `main` or `develop` branch).
3. Delete the faulty tag locally and remotely (`git push --delete origin app-v0.1.0`).
4. Commit the fix and push the tag again (or create a new patch tag).

## When can an exception be made

There are no exceptions. Official releases must always go through this workflow.

## Who approves the exception

Tech Lead or DevOps Engineer in case of a complete CI infrastructure failure.
