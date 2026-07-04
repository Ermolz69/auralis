# Release Workflow

## Why is it needed
Ensures a predictable, safe, and automated build of production artifacts for all platforms (Windows, macOS, Linux) without human error.

## What does it forbid
It strictly forbids mixing CI (for developers) and CD (publishing for users). It also forbids creating official releases without an attached version git tag. Manual production builds on local machines are strictly prohibited.

## Where does it run
Only on GitHub Actions (`release.yml`) and strictly only upon pushing a git tag (`app-v*`).

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
