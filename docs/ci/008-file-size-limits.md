# File Size Limits

## Why is it needed

Prevents the creation of huge, unreadable files and "god objects". Strict limits encourage developers to properly decompose code early on.

## What does it forbid

It rejects production files above the configured limit:

| Source area                   | Maximum lines |
| ----------------------------- | ------------: |
| Frontend pages                |           120 |
| Frontend widgets and features |           250 |
| Frontend entities             |           300 |
| Shared UI                     |           200 |
| Shared libraries              |           250 |
| Rust application crate        |           300 |
| Rust adapter crates           |           400 |

A small explicit ratchet list preserves existing oversized files at their current
size while they are split; those files may not grow.

## Where does it run

In the CI pipeline via the `task q:file-size` command. The task first runs `tools/scripts/check-file-size.test.mjs`, then runs `tools/scripts/check-file-size.mjs` against production source.

## How to fix the error

Split the file using the following strategies:

1. Extract pure data transformation functions into `lib/` or `utils/`.
2. Break down complex UI into smaller private sub-components.
3. Extract state management logic into `model/`.
4. Extract DTO mapping into the `api/` layer.
5. Move long constants and static data into `config/`.

## When can an exception be made

Limits do not apply to generated code, API types (`api-types`), lock files, SVG
icons, declarations, tests, snapshots, and explicit static-data file suffixes such
as `data.ts` and `constants.ts`. Rust `#[cfg(test)] mod tests` blocks are excluded
from production line counts.

Production filenames are still checked even when their basename contains words such as `mock`.

## Who approves the exception

Tech Lead. A new exception must be explicit and narrow inside `tools/scripts/check-file-size.mjs`; broad subtree, `mock`, or production-source waivers are not allowed.
