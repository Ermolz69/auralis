# Pre-main consolidation and full verification

Date: 2026-10-28. All prepared coverage, dependency, component-pin and evidence
branches were consolidated into `feat/pre-main-integration` by fast-forwarding
`b29719b` to `4310b3664b2b4b7ee992a328a1a7800e69c10540`. The dedicated pre-main
worktree and pinned Translate checkout are clean. The primary shared checkout
was not switched, and main remains unchanged.

The subsequent CI-only commit
`2fdb0c38a6b3ace10a304fda6fd42bf41b05d821` enables manual dispatch of the existing
Full Checks workflow and updates its exact trigger assertion. The workflow still
uses read-only permissions, the shared environment bootstrap and `task check`.
There are no production or native-harness changes in this follow-up.

## Fresh local verification

All executions use root Taskfile entrypoints and private dependency, temporary
and result directories. Existing source inclusion, thresholds, snapshots and
security policies are unchanged.

| Command                                                                    | Tested source                                 | Exit | Result                                                                                              |
| -------------------------------------------------------------------------- | --------------------------------------------- | ---: | --------------------------------------------------------------------------------------------------- |
| `task check:all`                                                           | `4310b36`                                     |    0 | 906 frontend tests; 758 top-level Rust tests; complete PR, documentation, security and global gates |
| `task check:storybook`                                                     | `4310b36`                                     |    0 | Eight contracts, 120 Chromium cases, 13 ordinary visual comparisons                                 |
| `task fe:storybook`                                                        | `4310b36`                                     |    0 | Static catalogue build                                                                              |
| `task ui:fixture`                                                          | `4310b36`                                     |    0 | 32 workflow and 18 diagnostic steps; both cleanup steps passed                                      |
| `task fe:smoke:library EVIDENCE=<owned-directory>`                         | `4310b36`                                     |    0 | One production browser fixture                                                                      |
| `task fe:smoke:puza SCREENSHOT=<owned-file>`                               | `4310b36`                                     |    0 | One production browser fixture                                                                      |
| `task ui:check:runner`                                                     | `f0080b7`, documentation only after `4310b36` |    0 | 33 controls                                                                                         |
| `task components:pins:check PYTHON=<private-python312>`                    | `f0080b7`                                     |    0 | 13 controls                                                                                         |
| `task ui:components:check PYTHON=<private-python312> UV_BIN=<existing-uv>` | `f0080b7`                                     |    0 | 12 offline Git/wheel controls                                                                       |
| `task q:ci-bootstrap`                                                      | `2fdb0c3`                                     |    0 | 28 controls using private frozen dependencies                                                       |

The [complete local report](pre-main-full-check-all.md) retains test counts,
coverage artifacts and transcript identity. The [fresh frontend record](pre-main-full-frontend.json)
retains individual browser commands, fixture identities, warnings and all six
unchanged bundle budgets. Coverage is 91.28% statements, 83.92% branches,
90.02% functions and 93.91% lines over 263 production modules. Total gzip is
158.57 KiB against 160 KiB.

The 23 existing ignored Rust tests remain outside the passing count. Two nested
child-fixture successes are separate from the 758 top-level result. The full
Windows command does not execute `rs:glib:regression`, because that Task declares
`platforms: [linux]`; its platform skip is not a successful Linux regression.

## Full Linux workflow

[Full Checks run 40738665132](https://github.com/Ermolz69/auralis/actions/runs/40738665132)
was manually dispatched on `feat/pre-main-integration` at exact commit
`2fdb0c38a6b3ace10a304fda6fd42bf41b05d821`. The workflow and job completed
successfully on Ubuntu 24.04.5 (`ubuntu-latest`, image `ubuntu-24.04`). Checkout,
full bootstrap, root `task check` and post-steps passed. Job 122994013118 ran
from 17:45:08 through 17:57:20 UTC; the check step took 10 minutes 25 seconds.

The completed log explicitly records
`cargo test --locked --release -p glib-backport-regression`: the optimized
profile finished and all four regressions passed, with zero failed or ignored.
The cases cover the FFI out-pointer, mixed-direction boundaries, empty/out-of-
bounds iteration and reverse/skipping borrowed UTF-8 values. This closes the
Linux platform gap left by the local Windows invocation.

The same Linux gate passed 906 frontend tests in 159 files, production browser
E2E, Rust formatting/all-target Clippy/locked workspace tests, documentation,
global checks and complete dependency audits. Coverage remained
91.28/83.92/90.02/93.91. Rust scanned 570 dependencies against 1,381 advisories;
the six existing maintenance notices remain visible under unchanged policy.
The retained UTF-8 `gh run view --log` transcript has SHA-256
`16a0ad2f250a1bbcb8ea6119fad7014dbdf7b4e63d24086fc4140cee2defe1df`.

This workflow runs model-free checks; it does not establish packaged/native app,
macOS, real model, listening or clean-machine acceptance.

## Retained setup attempts and acceptance limits

The initial restricted CI control attempt returned 201 before assertions with
`spawn EPERM`. An exploratory control run returned 201 with 27 passed and one
failed because the old assertion allowed only the reusable workflow trigger.
The updated assertion then passed 28 controls, but those exploratory runs
resolved an ancestor `js-yaml` 5.2.2 and are not counted as isolated verification.
The coordinator repeated the complete Task with private `js-yaml` 5.4.3 after
integration: 28 passed, exit 0. Formatting the two changed CI files also returned 0.

The final documentation gate initially returned 1 for a Markdown link to the
existing non-document YAML Taskfile outside its link inventory. The report now
uses the same portable Taskfile path as a code reference. The unchanged gate
then passed all eight controls, the complete document contract and lint; the
separate documentation formatting gate also returned 0.

The [earlier native attempt](pre-main-candidate-verification.md) remains failed
with unconfirmed cleanup and its retained session lock. No native retry,
recovery, unowned signal or guard exception occurred during this rerun. The OS
boot predates that failed case, so no independent session reset is established.
PM-017 remains failed and PM-018 blocked. These non-native checks do not establish
real model, listening, long-source/video or clean-Windows acceptance.
