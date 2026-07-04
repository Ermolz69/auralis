# File Size Limits

## Why is it needed

Prevents the creation of huge, unreadable files and "god objects". Strict limits encourage developers to properly decompose code early on.

## What does it forbid

It forbids committing files whose line count exceeds the fixed limits for their layer (e.g., more than 120 lines for `pages`, 250 for `widgets`, 400 for Rust adapters).

## Where does it run

In the CI pipeline via the `task q:file-size` command (runs the `check-file-size.mjs` script).

## How to fix the error

Split the file using the following strategies:

1. Extract pure data transformation functions into `lib/` or `utils/`.
2. Break down complex UI into smaller private sub-components.
3. Extract state management logic into `model/`.
4. Extract DTO mapping into the `api/` layer.
5. Move long constants and static data into `config/`.

## When can an exception be made

Limits do not apply to generated code, API types (`api-types`), lock files, SVG icons, test snapshots, and files with large arrays of static data (containing words like `mock`, `data`, `constants`).

## Who approves the exception

Tech Lead. A new exception must be added to the `isExcluded` function inside `tools/scripts/check-file-size.mjs`.
