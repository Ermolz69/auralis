# Integrated candidate verification

Date: 2026-10-28. Tested and built source:
`5a5ca531c37fef4eade0633e276e5d6a12688b7e`.

The complete local PR gate and ordinary Windows build passed. The native
metadata smoke failed and could not confirm cleanup. **PM-017 is failed and
PM-018 is blocked; this candidate is not technically ready.** The failed case
and its retained session lock remain unchanged. No recovery or later native
attempt is recorded.

The [structured record](pre-main-candidate-verification.json) contains complete
identities, command outcomes, resource hashes and native step results. The
[coverage inventory](pre-main-candidate-coverage.json) retains all 263 production
modules, missed statement counts and uncovered line/function/branch locations.

## Commands and scope

All project checks and builds used the root Taskfile and private mutable Cargo,
pnpm, temporary and result directories. The final PR run used
`VITEST_MAX_WORKERS=2`, `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.

| Command                                                                                                                                                                                             | Source                                            | Exit | Result                                                                          |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------- | ---: | ------------------------------------------------------------------------------- |
| `task check:pr`                                                                                                                                                                                     | `5a5ca53`                                         |    0 | Complete frontend, Rust, documentation, security and global gates               |
| `task ui:build`                                                                                                                                                                                     | `5a5ca53`                                         |    0 | Ordinary Windows x64 release; 14 minutes 21 seconds                             |
| `task ui:assets:project SEED=<new-seed> INPUT_ROOT=<frozen-input-root> INPUT_SEED=real-components-20261024-v2 MANIFEST_SHA=<input-sha> PYTHON=<private-builder> CONTRACT_ROOT=<clean-media-source>` | `5a5ca53`                                         |    0 | New metadata-only seed, 3261 files and 59,324,512 bytes                         |
| `task ui:freeze:project CASE=<new-case> SEED=<new-seed>`                                                                                                                                            | `5a5ca53`                                         |    0 | New frozen case; one attempt and inference disabled                             |
| `task ui:smoke SPEC=<frozen-case>`                                                                                                                                                                  | `5a5ca53`                                         |  201 | Ten passed steps, two failed, one not run; cleanup unconfirmed                  |
| `task components:pins:check PYTHON=<private-builder>`                                                                                                                                               | Pending `5a5ca53` Taskfile content, before commit |    0 | Thirteen controls; external bytecode override absent; no untracked cache output |

The ordinary build cleared inherited `TAURI_CONFIG` and native-runner overrides;
the Task retained its normal `AURALIS_NATIVE_E2E=0` and `VITE_NATIVE_E2E=0` flags.
Media resources were preserved and verified. No models, model downloads, doctors,
existing projects, voice references or qualification statuses were changed.

## Complete gate results

Frontend: 159 files and 906 tests passed with no failures or skips. Production
browser E2E passed all 35 scenarios and the production smoke passed its one test.
Coverage thresholds remained 90% statements, 80% branches, 90% functions and
92% lines.

| Metric     | Recorded combined baseline at `c18b7f4` | Final `5a5ca53` | Final covered / total |
| ---------- | --------------------------------------: | --------------: | --------------------: |
| Statements |                                  90.92% |          91.28% |           5733 / 6280 |
| Branches   |                                  83.43% |          83.92% |           4322 / 5150 |
| Functions  |                                  88.88% |          90.02% |           1598 / 1775 |
| Lines      |                                  93.56% |          93.91% |           5063 / 5391 |

The [baseline evidence](pre-main-frontend-coverage.json) and fresh final inventory
preserve the same 263-module set. There remain 547 missed statements, 828 branches,
177 functions and 328 lines. JSON summary and LCOV were emitted; no statement
locations are fabricated from aggregate counts.

Rust: 758 top-level tests passed, zero failed and 23 existing tests remained
ignored. Two nested child-fixture successes make the raw harness-summary sum
760; they are not counted twice. Formatting and all-target Clippy with denied
warnings passed. [Earlier Rust evidence](pre-main-rust-security.md) and the
component-build evidence retain the unchanged implementation's separate checks.

The final audit scanned 570 dependencies against 1381 advisories, with no
vulnerability or unsound finding. Advisory/source policy passed. The same six
transitive maintenance notices remain visible in the unchanged policy and
[advisory inventory](pre-main-advisories.md). Eighteen existing frontend lint
warnings and the existing Vite/dynamic-import warnings remain recorded.

## Ordinary candidate and metadata inputs

- Executable: 23,408,640 bytes; SHA-256
  `1880d2ca6e7d8ee70aa249881ce05a0eef68752f092ac4a1e329e75d932dda41`.
- Version 3 ordinary-release receipt: SHA-256
  `21d0552f8be6ca0273c72714f94714ac684eb9c1e7e942f4be8b89a035f0d110`.
- Seven-resource inventory: SHA-256
  `6ce98d3adbe0e0a25de9e74c0f937a78e7f66cbc8d3887dddb03730ade1f46e9`
  over compact JSON in receipt order.
- Build inputs bind 1982 application and 1897 clean Translate source files.
  Translate is pinned to `5d98cb3ca341c25bd446f7fc9d98d4b62ef551f5`;
  the working-diff digest is the empty SHA-256.

The new seed and case are named `pre-main-metadata-20261028-final-v1`.
The seed manifest SHA-256 is
`1da7c8d53cb1d6fc56dedc4ba12b7080f6e8d950660d84c3a01cd818c0525916`.
The JSON record retains frozen input identity, tool pins, media source/tree and
the reproduced canonical media wheel hash.

Staging verified speech CONTRACT 0.2.0 metadata and configured media CONTRACT
0.3.0. Native launch verified SQLite schema 22 with zero projects and jobs.
The native log records speech metadata verification, but the failed
create/restart sequence does not establish the media-contract round trip.

## Failed native attempt

All thirteen step statuses are retained in the JSON record. Preflight, seed,
launch, empty-library/input/source/settings checks and zero page errors passed.
Creation failed when the page closed; restart was not run. Cleanup failed.
Resource observations were not required; session release retained the lock.

The observer recorded eleven known process identities and one ambiguous
`conhost.exe` identity whose parent was not attributed. The matching observer
error activates the existing page-close guard. The native log does not establish
an application crash; the prior settings-close action has a distinct exact
accessible name and its step passed.

A later read-only OS inventory found all twelve captured PID/birth identities
absent and zero competing component processes. This proves later absence only.
It cannot establish historical ownership of the unseen parent or retroactively
confirm the failed cleanup. The incomplete trace lacks a ZIP end-of-central-
directory record; it is retained as incomplete evidence, not a valid trace.

The failed case, report and session lock remain retained. No lock removal,
unowned process signal, guard relaxation or subsequent native admission was
performed. Historical sampler attribution was not established. A separate
reviewable observer-provenance and controlled-recovery effort is required before
another native case; PM-018 remains blocked.

## Retained earlier attempts and limits

The earlier `f1a5049` full PR run passed with coverage
91.32 / 84.01 / 90.02 / 93.91%. Those counters remain historical; the final
inventory uses the fresh lower statement/branch counters above. Its provisional
ordinary build was interrupted with exit 201 after identifying the bytecode-
writing Task. No successful candidate was produced. The Task correction was
verified without an external override before the final complete PR/build run.

Hosted Linux/macOS checks, clean-machine installer acceptance, real media,
inference and listening are not established by this Windows metadata attempt.
Main was not merged or updated by these verification tasks.
