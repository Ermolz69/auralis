# Studio workflow coverage verification

## Scope and source

This record covers PM-002, PM-005 and the Studio/workflow portion of PM-006.
It provides the relevant frontend baseline for PM-001, without closing the
dependency audits or the integrated PM-007 coverage gate.

- Base: `b29719b0dc784745b70275ec93968f5cbda69df3` on
  `feat/pre-main-integration`.
- Implementation: `aed50d386fb95c22e52718e47b0ba3d326372c26` and
  `102b5c6322ba1415551ef1bfdfb8d4000135dbcd` on
  `test/studio-workflow-coverage`. The final complete run uses `102b5c6`.
- Verification date: 2026-10-28.
- Dependencies, test discovery, production inclusion and coverage thresholds
  retain the base revision's policy.

The [coverage inventory](pre-main-studio-coverage.json) records original and
final totals, per-module missed statements, branches, functions and lines,
and the remaining scoped line/function/branch locations. Paths in that record
are repository-relative.

## Environment and isolation

| Tool                 | Version |
| -------------------- | ------- |
| Node.js              | 24.14.0 |
| pnpm                 | 11.25.0 |
| Task                 | 3.50.0  |
| React                | 19.2.7  |
| Vite                 | 8.1.3   |
| Vitest / V8 coverage | 4.1.11  |
| TypeScript           | 6.0.3   |
| Playwright           | 1.61.1  |

Frozen installation used `task fe:install`. pnpm 11 requires
`PNPM_CONFIG_STORE_DIR` and `PNPM_CONFIG_PACKAGE_IMPORT_METHOD=copy` for this
isolation; the older `npm_config_` prefix was ignored. The initial incorrectly
configured installation was stopped, its private dependency directories were
removed, and installation completed with a worktree-local store and copied
packages. Reported tests use those isolated dependencies and worktree-local
coverage, optimizer and test outputs.

Lockfile SHA-256 values before and after implementation:

- `pnpm-lock.yaml`:
  `c3d41a10318ee685c0e9c89435367e1d2db8b7851423ff6f4047b8e2490bfa2e`.
- `Cargo.lock`:
  `9aaddb2ac0cc78d036342e1649632239005445c96a5fd3cca2ce5a6eebd96738`.

## Reproduced baseline

No production files were changed before the original coverage inventory was
captured. New tests were held outside discovery for that run.

| Metric     | Covered / total | Baseline | Required |
| ---------- | --------------- | -------- | -------- |
| Statements | 5084 / 6564     | 77.45%   | 90%      |
| Branches   | 3904 / 5449     | 71.64%   | 80%      |
| Functions  | 1294 / 1813     | 71.37%   | 90%      |
| Lines      | 4148 / 5251     | 78.99%   | 92%      |

The successful test execution in the baseline was **675/675 tests**. The root
command still exited **201** because all four coverage thresholds failed.
The historical percentages were reproduced, rather than assumed.

Retained unsuccessful baseline attempts:

- Sandboxed `task fe:install`: exit 201, registry connections refused. The
  isolated installation with network access subsequently exited 0.
- Sandboxed `task fe:test`: exit 201, runner exit 3221226356 before test results.
- Sandboxed `task q:frontend`: exit 201, child-process `spawn EPERM`.
- Unrestricted `task fe:test`: exit 201; 468 passed, two failed, and 26 worker
  startup errors. `ProjectPage` stories `Wide 1280` and `Subtitles Wide` failed.
- `task fe:test -- --maxWorkers=2`: exit 201; 674 passed and `Subtitles Wide`
  failed. No coverage report was emitted on that failed test run.
- `task fe:test -- --maxWorkers=2 --coverage.reportOnFailure`: exit 201;
  675 passed, with the four coverage failures shown above.
- Unrestricted `task q:frontend`: exit 0 on the unmodified base.

Two workers bound process contention without changing test membership or
thresholds. `reportOnFailure` retains measurements while preserving the failed
exit code. The preliminary Storybook failures remain failed attempts; their
root cause was not established by this work.

## Behavior verified and repaired

- Command menus bind actions to the current project/material and reject stale
  removed or hidden targets. Keyboard navigation and menu dismissal restore
  focus. Dialog cancellation, whitespace validation, real TXT import, native
  picking errors and retry paths have assertions on visible state and admitted
  operations.
- Commands and version saves admit repeated requests once. Copy progress and
  cancellation failures remain visible. Failed/cancelled publication and a
  missing operation without a published link cannot close as success; late
  operation polls cannot resurrect finished progress.
  The copy observer is released when Studio unmounts, and a failed File Exit
  action is shown in the command alert with a working retry.
- Tool panels wait for the library and validate project/material response
  identities before mounting processing controls. Foreign or partial contexts
  are rejected, mapped artifacts are used, and verification failures have a
  retry action. Superseded context responses are ignored.
- Source controls require the selected project source identity. Tests exercise
  ready downloaded sources, foreign materials, track selection/filtering,
  stale media resolution and actionable structured API errors.
- Translation preparation admits only the selected subtitle artifact. Speech
  uses only a selected translation output from the current project. SRT/VTT
  source identity, saved text, artifact reads, independent text-version lineage,
  failed publication and input replacement have instrumented assertions.
- Speech plans use the accepted decision ID and source digest. Editing removes
  approval; a late review from an older edit or a different project cannot
  restore it. Tests exercise the real speech workflow through SpeechTool.
  Approval scope initialization runs before those controls become interactive.
- Results include only ready managed artifacts from the selected material's
  lineage or project results. Playback begins only after successful resolution,
  admits one pending resolver and ignores an old material/project reply.
- Tabs preserve shared speech/text context, support keyboard navigation and
  confirm discard even for an empty edit or an edit made before autosave.
  Drafts flush during panel cleanup, failed persistence preserves accepted
  state and can be retried, and standalone material changes open their own
  workspace without replacing the original tabs.
- Guided navigation retains ordinary settings return context, returns home
  after project removal and releases event listeners. Update notifications
  retain one owner across views, including actions created before switching.
  Native window tests cover browser fallback, handle/subscription/action
  failures, resize ordering, repeated actions and late cleanup.

The components, providers, menu models, draft hook and speech review logic are
real in these tests. IPC, native window APIs, file picking, notification output
and media URL conversion are mocked external boundaries. Fixture audio and
catalog entries establish no model execution or voice qualification.

## Final verification

The final complete run on `102b5c6` passed **793/793 tests in 151 files**, including
118 additional tests. Its exit code remains **201** because all four aggregate
coverage thresholds fail. Both snapshots include the same 263 production modules.

| Command                                                                                                           | Exit code | Result                                         |
| ----------------------------------------------------------------------------------------------------------------- | --------- | ---------------------------------------------- |
| `task fe:test:unit -- src/app/studio --maxWorkers=2`                                                              | 0         | 22 tests passed                                |
| `task fe:test:components -- src/app/studio src/features/speech-catalog/ui/SpeechWorkflow.test.tsx --maxWorkers=2` | 0         | 71 tests passed                                |
| `task fe:test:integration -- src/app/studio src/app/workflow --maxWorkers=2`                                      | 0         | 35 tests passed                                |
| `task fe:test -- --maxWorkers=2 --coverage.reportOnFailure`                                                       | 201       | 793 tests passed; four coverage failures       |
| `task q:frontend`                                                                                                 | 0         | Complete frontend quality gate passed          |
| `task fe:typecheck`                                                                                               | 0         | TypeScript passed                              |
| `task fe:lint`                                                                                                    | 0         | React hook/export warnings remain unsuppressed |

| Metric     | Baseline | Final covered / total | Final  | Required |
| ---------- | -------- | --------------------- | ------ | -------- |
| Statements | 77.45%   | 5807 / 6745           | 86.09% | 90%      |
| Branches   | 71.64%   | 4422 / 5567           | 79.43% | 80%      |
| Functions  | 71.37%   | 1504 / 1830           | 82.18% | 90%      |
| Lines      | 78.99%   | 4730 / 5392           | 87.72% | 92%      |

For the scoped inventory, including App and the speech workflow hook, statements
increased from 40.87% to 93.76%, branches from 33.59% to 87.39%, functions from
32.69% to 87.65%, and lines from 39.54% to 95.02%. The scope still contains 67
missed statements, 94 branches, 41 functions and 42 lines; it is not claimed to
exhaust every scoped path. The JSON inventory retains their exact locations.

An earlier complete run on `aed50d3` exited 201 with 783/785 tests passing and
the same two preliminary `ProjectPage` Storybook failures. It measured 85.20%
statements, 79.00% branches, 80.98% functions and 87.28% lines. That failed attempt
is not replaced by or represented as the final successful test execution.

## Remaining integration work

The shared task index and testing configuration are not edited by this branch.
PM-007 must recalculate the integrated result after the other coverage and npm
branches are combined. This branch does not establish a passing aggregate gate,
security audit, frontend production build, native candidate, model acceptance
or eligibility to merge main.

Residual library operations/providers and browser/asset controls belong to
PM-003/PM-004. Remaining `useVoiceCatalog` and legacy `ProjectListPanel` paths
belong to the remaining PM-006 integration work. Other residual shared
validation, feature and legacy UI paths belong to PM-007. The final inventory
enumerates remaining measured gaps instead of treating untested code as passed.
Scoped residuals include WorkspaceAssetBrowser interactions, TranslateTool
refresh callbacks, remaining workspace/tab branches and speech workflow actions;
these remain explicit PM-007 follow-up alongside the other areas.

No models were loaded or downloaded, and no model doctors were run. Existing
project data, voice references and qualification decisions were not modified.
No changes were merged or published to main.
