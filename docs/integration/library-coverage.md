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

Implementation source: `ffed99f00caa6945f642cf1594fe447e58890678`.

- `e4ab55d` adds owner rollback, publication readiness, independent copying,
  cancellation, conflict, refresh ordering and listener lifecycle tests. It fixes
  duplicate import IDs when a mutation is replayed and releases each subscription
  independently, including late resolution and partial subscription failure.
- `ffed99f` adds creation and validation, filtering, selection, properties,
  ancestry, context menus, keyboard, drag/drop, preview failure/recovery, voice
  qualification, uncertain cast persistence and ProjectListPanel regressions.
- The branch adds 93 tests. IPC and provider fixtures exercise the production
  modules; no model execution, real media publication or voice qualification
  decision is performed by these tests.

All final measurements below were made on 2026-10-28. The unchanged thresholds
remain statements 90%, branches 80%, functions 90% and lines 92%. Production
inclusion, test discovery, Taskfiles, dependency manifests and lockfiles were
preserved.

### Comparable baseline and final coverage

After retaining the initial failures above, the clean isolated worktree was
temporarily checked out at `b29719b` and measured again with the exact final
command. It was then returned to `test/library-coverage`. All four test projects
executed in both comparable runs; the browser port collision did not recur.
Neither complete run passed its gate.

Command for both runs:

```text
task fe:test -- --maxWorkers=4 --coverage.reportOnFailure --browser.api.port=63427
```

| Measurement       | Baseline `b29719b`         | Final `ffed99f`            |
| ----------------- | -------------------------- | -------------------------- |
| Task exit         | 201                        | 201                        |
| Tests             | 673 passed, 2 failed / 675 | 767 passed, 1 failed / 768 |
| Scope statements  | 42.63% (333/781)           | 96.69% (761/787)           |
| Scope branches    | 32.81% (126/384)           | 91.70% (354/386)           |
| Scope functions   | 34.64% (79/228)            | 90.70% (205/226)           |
| Scope lines       | 42.45% (270/636)           | 97.66% (627/642)           |
| Global statements | 76.27%                     | 82.97%                     |
| Global branches   | 71.15%                     | 75.41%                     |
| Global functions  | 70.36%                     | 77.45%                     |
| Global lines      | 78.42%                     | 85.19%                     |

The scope is the 16 production modules in the table below. These are measured
counts from failed gate runs, not an assertion that the integrated application
is ready. Instrumented totals can differ between executed and unexecuted
modules; the machine records preserve the actual covered and total counts.
The historical 77.45/71.64/71.37/78.99 global baseline was not reproduced.

The complete baseline failed ProjectPage's Wide 1280 and Subtitles Wide stories
while waiting for `Video source configuration`. The final run failed only
Subtitles Wide while waiting for the Russian subtitle label. Those stories,
their fixtures and the ProjectPage implementation were not changed. Their
failure states vary across attempts and remain an integration blocker requiring
investigation; this branch does not classify them as passing.

Machine evidence:

- [Initial incomplete baseline](library-coverage-baseline.json).
- [Comparable baseline counts](library-coverage-baseline-comparable.json).
- [Final counts and residual LCOV locations](library-coverage-final.json).

The following percentages are statements / branches / functions / lines. Paths
are relative to `apps/desktop/src/`.

| Production module                                      | Comparable baseline           | Final                         |
| ------------------------------------------------------ | ----------------------------- | ----------------------------- |
| `entities/library/model/LibraryProvider.tsx`           | 82.01 / 47.61 / 76.31 / 83.01 | 90.78 / 75.00 / 80.00 / 92.66 |
| `entities/library/model/actions.ts`                    | 100 / 100 / 33.33 / 100       | 100 / 100 / 100 / 100         |
| `entities/library/model/catalog.ts`                    | 100 / 85.18 / 100 / 100       | 100 / 96.29 / 100 / 100       |
| `entities/library/model/context.ts`                    | 88.88 / 50 / 100 / 100        | 100 / 100 / 100 / 100         |
| `entities/library/model/operations.ts`                 | 0 / 0 / 0 / 0                 | 100 / 91.11 / 100 / 100       |
| `entities/library/ui/VideoPreview.tsx`                 | 7.84 / 0 / 0 / 7.69           | 98.03 / 91.42 / 100 / 100     |
| `features/voice-catalog/ui/useVoiceCatalog.ts`         | 70.09 / 61.53 / 70.58 / 74.22 | 100 / 97.43 / 100 / 100       |
| `widgets/app-shell/ui/app-shell/ProjectListPanel.tsx`  | 50 / 42.85 / 50 / 45.45       | 100 / 92.85 / 100 / 100       |
| `widgets/library-browser/ui/AssetCards.tsx`            | 38.09 / 38.88 / 37.50 / 36.84 | 100 / 83.33 / 100 / 100       |
| `widgets/library-browser/ui/AssetInspector.tsx`        | 2.73 / 0 / 0 / 1.72           | 98.63 / 93.33 / 97.22 / 98.27 |
| `widgets/library-browser/ui/AssetVersions.tsx`         | 4.76 / 0 / 0 / 5.88           | 95.23 / 90 / 83.33 / 100      |
| `widgets/library-browser/ui/LibraryBrowser.tsx`        | 82 / 76.47 / 56.52 / 79.48    | 94 / 97.05 / 78.26 / 92.30    |
| `widgets/library-browser/ui/LibraryCreateDialog.tsx`   | 4.76 / 0 / 0 / 3.03           | 100 / 96.66 / 100 / 100       |
| `widgets/library-browser/ui/LibraryNavigation.tsx`     | 51.42 / 65 / 44.44 / 48.27    | 100 / 100 / 88.88 / 100       |
| `widgets/library-browser/ui/LibraryProjectActions.tsx` | 0 / 0 / 0 / 0                 | 86.79 / 91.66 / 73.68 / 92.85 |
| `widgets/library-browser/ui/controls.ts`               | 100 / 100 / 100 / 100         | 100 / 100 / 100 / 100         |

### Focused verification

| Command                                                                                                                                                                              | Exit | Result                                                                  |
| ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---- | ----------------------------------------------------------------------- |
| `task fe:test:unit -- src/entities/library --maxWorkers=2`                                                                                                                           | 0    | 35 tests passed.                                                        |
| `task fe:test:components -- src/entities/library src/widgets/library-browser src/features/voice-catalog src/widgets/app-shell/ui/app-shell/ProjectListPanel.test.tsx --maxWorkers=2` | 0    | 62 tests passed.                                                        |
| `task fe:storybook:library`                                                                                                                                                          | 0    | All 3 StudioShell stories passed in Chromium.                           |
| `task q:frontend`                                                                                                                                                                    | 0    | Architecture and frontend policies passed.                              |
| `task fe:typecheck`                                                                                                                                                                  | 0    | TypeScript project build passed.                                        |
| `task fe:lint`                                                                                                                                                                       | 0    | No errors; four hook-dependency warnings in unchanged production files. |

Before the fixes, the new operations tests had 1 failure / 30 tests and the
provider tests had 2 failures / 9 tests, both tasks exiting 201. They reproduced
duplicate import IDs, double cleanup of late subscriptions and a leaked
subscription after another subscription failed. Later test-development failures
in context-event delegation and the pin retry spy were corrected; the final
focused commands above supersede those failed attempts.

The lint warnings refer to VideoPreview, ResultsTool, StudioWorkspace and
StudioToolPanel dependency arrays. They were retained without suppression.

### Remaining scope

The final scope still misses 26 statements, 32 branches, 21 functions and 15
lines. Exact missed lines, branch lines and function entry locations are in the
final JSON. Residual paths include:

- LibraryProvider's asset-validation warning, `openTool`, workspace activation
  and defensive event-refresh rejection callbacks.
- LibraryBrowser's responsive inspector selection callbacks, local dialog close
  path and delegated-operation rejection callbacks; LibraryNavigation's rejected
  project-open callbacks. Hidden responsive controls were not used to satisfy
  interaction assertions.
- AssetInspector's project selector change, AssetVersions' rejected-open
  callback, and LibraryProjectActions' populated project cleanup filters,
  cancellation controls and removal failure.
- Branch-only edges such as preview-cache capacity eviction and additional
  empty/missing catalog combinations. The voice-catalog hook has full statement,
  function and line coverage; one branch remains uncovered.

PM-007 must reconcile these residual locations with the integrated workspace
and command branches and recalculate all four global gates. The ProjectPage
Storybook failures also remain open. No main merge, publication, native-runtime
acceptance or real-model acceptance is established by this evidence.
