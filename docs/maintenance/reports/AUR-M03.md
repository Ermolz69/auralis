# AUR-M03 acceptance report — Translate executor responsibilities

## Candidate and scope

- Repository: Auralis. Branch: `refactor/auralis-translation-executor` in `.cache/worktrees/auralis-translation-executor`.
- Recorded base: `43136f3fc00d0ad5754f293d7a535dfc47f75739`, containing the maintenance packet. Source candidate: `10e3c521e4a3c2fe541e5ac42bd34c06c9204ca8`.
- Remote: `origin` at `git@github.com-personal:Ermolz69/auralis.git`. The branch was pushed successfully with report commit `e5a8606`; no PR was requested.
- Owned changes: `crates/adapters-translate/src/run_executor.rs`, private `run_executor/{frozen_input,attempt}.rs`, focused `crates/adapters-translate/tests/run_registration.rs`, and this report. No application, port, storage, root Taskfile, nested Translate, sibling Translate or other project files were edited.
- The Auralis gitlink and initialized embedded checkout both remain at `44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`. The checked model/runtime was not installed or started for this assignment.
- Verified global author and committer: `Ermolz <00ermzahar@gmail.com>`, with no local or environment identity override.

## Responsibility map and behavior

| Owner                 | Before                                                                                  | After                                    | Preserved boundary                                                                                                                      |
| --------------------- | --------------------------------------------------------------------------------------- | ---------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------- |
| `run_executor.rs`     | Public port, frozen validation, execution and progress lived in one 388-line file.      | 171-line facade and run-state dispatch.  | Public `TranslationExecutionPort`, constructors, async `spawn_blocking`, validated fast path and running busy error.                    |
| `frozen_input.rs`     | Identity/profile/policy, bounded source and Translate DB checks were inline.            | Private 131-line ordered preparation.    | Exact validation precedence, source/plan and stored run checks, operation-specific `PortError` mapping.                                 |
| `attempt.rs`          | Guard, preflight, request, stop, result commit and progress forwarding were inline.     | Private 159-line attempt lifecycle.      | Guard before server work, preflight before attempt, durable checkpoint progress, pause/failure stop reason and immutable result commit. |
| `run_registration.rs` | Seven integration cases covered success, reuse, pause and related registration/control. | Four focused cases added; 11 cases pass. | Owned temporary SQLite and mock/local HTTP only; source bytes remain unchanged.                                                         |

No public Rust/IPC API, serialization, error message, SQL/schema policy, model profile, retry/timeout, resource ownership or publication behavior changed. `ForwardProgress` stayed with the attempt because its sender exists only while that attempt executes; a third module would add navigation without separating a responsibility.

## Ordered lifecycle and error matrix

The call order remains: (1) active link/intent IDs, (2) typed IDs and language pair, (3) frozen profile digest/parse/checked requirement, (4) block policy/profile match, (5) bounded source parse and frozen hash/parser/policy, (6) Translate DB source and run checks, (7) run state, (8) captured or supplied start guard and stale check, (9) provider construction and controlled preflight, (10) checked guard, retry/control DB and guarded attempt, (11) committed checkpoint forwarding and stop on error, (12) immutable result commit or stop on commit failure. A validated run returns its verified existing result at step 7, before provider construction.

| Boundary                                             | Result and persistent state                                                                                                     | Coverage                                                                     |
| ---------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------- |
| Link/source/profile mismatch or invalid block policy | Original `Conflict`/`InvalidSource` string and precedence; no new attempt/checkpoint.                                           | New frozen validation case; existing registration cases.                     |
| Checked preflight failure                            | `ExternalToolFailed` with tool `llama.cpp preflight`; run remains `Requested`, no attempt, original unchanged.                  | New checked-profile preflight case.                                          |
| Validated run                                        | Existing result ID is returned without a server request or new attempt.                                                         | Existing local model/reuse case.                                             |
| Running run                                          | Exact `Busy` message; no second attempt.                                                                                        | New running case.                                                            |
| Stale supplied guard/paused admission                | `Cancelled` before provider work; no progress forwarded.                                                                        | New guard case and application admission/pause suite.                        |
| Mid-request pause                                    | `Cancelled`, stopped as `Paused`, no checkpoint or result from the interrupted request; only committed progress appears.        | Existing adapter pause case and application scheduler/pause cases.           |
| Model execution failure                              | Existing external-tool error; host job and Translate run fail without replacing source or result.                               | Application `translation_host_execution_failure` case.                       |
| Result commit failure                                | `Storage` at `commit_translate_result`; checkpoint retained, attempt stopped `Failed`, no result publishable, source unchanged. | New SQLite trigger/mock HTTP case.                                           |
| Successful commit                                    | One immutable `NeedsReview` result, committed checkpoint and separate verified output.                                          | Existing adapter success/reuse and application SRT/WebVTT publication cases. |

## Checks and change accounting

- Baseline `task rs:test:translate`: exit 0, 15 passed, two asset-dependent tests ignored.
- Before extraction, `task rs:test:translate -- --test run_registration`: exit 0, 11 passed after test-fixture corrections. The new fixture first had a test-only boxed-error conversion failure and then used the initial-attempt API for a paused state; both were fixed before the behavior baseline commit. No product-code failure was found.
- After each extraction, `task rs:test:translate -- --test run_registration`: exit 0, 11 passed.
- Final source candidate: `task rs:test:translate` exit 0 (19 passed, two asset-dependent ignored); `task rs:test:application` exit 0 (application unit and integration suites passed; real model/installed runtime and benchmark cases stayed ignored); `task rs:test:translation-events` exit 0 (2 passed).
- `task rs:fmt` exit 0; `task q:file-size` exit 0 (8 policy tests and source scan); `task check:docs` exit 0 (5 checker tests, 56 indexed Markdown files, 57 linted, zero lint errors).
- After adding this report, `task q:format-write -- docs/maintenance/reports/AUR-M03.md` exit 0; `task check:docs` exit 0 (5 checker tests, 57 indexed, 58 linted, zero lint errors); `task check:quality:docs` exit 0.
- Initial restricted `task q:ipc-contract` failed with Node child-process `spawn EPERM`. The same task with local process access exited 0 (6 tests and Rust/TypeScript parity). This was an environment restriction, not contract drift.
- `task rs:clippy` was run with `-D warnings` and exited 1 in unchanged `crates/jobs/src/manager/runtime_registry.rs:58`: `clippy::manual_unwrap_or_default`. That code is present verbatim at base `43136f3`; AUR-M03 cannot edit the jobs crate. No lint was suppressed. The full gate remains open for the receiving reviewer or its owner.
- Native, real model, clean-install, language-quality and audio checks were not run. Mock/compile results do not qualify those gates.
- Commits: `5895317` tests, 186 insertions; `a2fd72e` frozen verification, 149 insertions/98 deletions; `10e3c52` attempt extraction, 171 insertions/149 deletions. Against base, ordinary diff is 495 insertions/236 deletions across four source/test paths; `git diff -M50% -C50%` reports the same because these are extractions within a modified source, not whole-file renames. New report lines are additional. Generated diffs: none. `git diff --check` is clean.

## Reviewer handoff

Read the complete code and test diff, compare error text and the ordered SQL/preflight/guard boundaries with base, and rerun the focused adapter and application checks on the committed candidate. Resolve or separately scope the pre-existing jobs Clippy warning before treating the full lint gate as green. AUR-M02 can confirm that its current guide still names the correct host executor boundary after this private extraction; no documentation contract changed here. AUR-M06 must assess the Translate gitlink independently. This report is a candidate record, not a reviewer acceptance verdict or release qualification.
