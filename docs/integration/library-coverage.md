# Library coverage evidence

Scope: PM-003, PM-004 and the library, voice-catalog and ProjectListPanel
portion of PM-006. The integrated global gate remains PM-007's responsibility.

## Baseline

Source: `b29719b0dc784745b70275ec93968f5cbda69df3` on
`feat/pre-main-integration`, measured on 2026-10-27 before implementation.
Work branch: `test/library-coverage`.

Versions: Node 24.14.0, pnpm 11.25.0, Task 3.50.0, Vitest and coverage-v8
4.1.11, Vite 8.1.3, React 19.2.7, TypeScript 6.0.3, Playwright 1.61.1,
Storybook 10.4.6, Prettier 3.9.4.

Lockfile SHA-256:

- `pnpm-lock.yaml`: `c3d41a10318ee685c0e9c89435367e1d2db8b7851423ff6f4047b8e2490bfa2e`.
- `Cargo.lock`: `9aaddb2ac0cc78d036342e1649632239005445c96a5fd3cca2ce5a6eebd96738`.

The worktree has its own pnpm store, copied packages, virtual store, Vite caches
and coverage output. pnpm 11 requires `PNPM_CONFIG_STORE_DIR` and
`PNPM_CONFIG_PACKAGE_IMPORT_METHOD=copy`. Initial installation attempts using
the obsolete npm configuration prefix were interrupted; the frozen isolated
installation through `task fe:install` completed with exit 0.

| Command                                                           | Exit | Observed result                                                                                                                            |
| ----------------------------------------------------------------- | ---- | ------------------------------------------------------------------------------------------------------------------------------------------ |
| `task fe:test:unit -- src/entities/library/model/catalog.test.ts` | 0    | 2 tests passed.                                                                                                                            |
| `task q:frontend`                                                 | 0    | All architecture and frontend policies passed.                                                                                             |
| `task fe:test`                                                    | 201  | 418 passed, 2 ProjectPage Storybook failures, 39 worker-start errors; no coverage report.                                                  |
| `task fe:test -- --maxWorkers=4`                                  | 1    | Interrupted to enable coverage reporting on failures; no usable result.                                                                    |
| `task fe:test -- --maxWorkers=4 --coverage.reportOnFailure`       | 201  | 555 Node tests passed; Storybook API port 63315 was occupied. Coverage thresholds failed; shutdown also reported outstanding FS callbacks. |

The first full run could not find `Video source configuration` in ProjectPage's
Wide 1280 and Subtitles Wide stories. Worker errors were timeouts waiting for
forks to start. The bounded run did not execute Storybook, so its 73.81%
statements, 68.70% branches, 68.70% functions and 77.00% lines are **incomplete
global coverage**, not a replacement for the historical 675-test result.

[Baseline module counts](library-coverage-baseline.json) retain covered, total
and missed statements, branches, functions and lines for this scope. Security
audits are outside this branch's assigned PM-001 slice and were not run here.

## Implementation and final verification

Pending implementation and fresh checks. No integrated readiness is claimed.
