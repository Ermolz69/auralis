# Goal for Translate readiness in Auralis

Updated: 10 October 2026. Submit the objective below for explicit execution.
Preparing this file does not start the Goal. Required integration IDs: INT-TR01
through INT-TR05, mapped to the existing canonical Translate task IDs.

## Objective to submit

Finish Auralis Translate as a reusable Rust library and working desktop module
for the existing declared Chinese-to-Russian plain-SRT and strict plain-WebVTT
scope. Complete the required translation quality, durability, desktop, package
and clean-install gates for that finite candidate. Keep standalone CLI behavior
and all supported immutable file invariants. Deliver reviewed, versioned results
that Auralis can explicitly approve and export to a future speech consumer.

Read E:/Anything/Projects/Commercial/auralis/docs/integration/goals/common.md
and the integration architecture. Read Translate AGENTS.md, docs/README.md,
PRODUCT_PLAN.md, DELIVERY_PLAN.md, IMPLEMENTATION_BACKLOG.md, AGENT_WORKFLOW.md,
RELEASE_ACCEPTANCE.md, IMPLEMENTATION_STATUS.md, regression policy 008 and
relevant architecture, model and experiment records. Inspect both the independent
Translate checkout and Auralis's pinned submodule, current diffs, profiles,
hardware and runtime identities. Do not assume the submodule matches sibling
uncommitted work.

Freeze the selected endpoint under PLAN-03 and map all mandatory G1 through G9
requirements to canonical backlog IDs and acceptance evidence. Separate supporting
library integration checks from semantic language release. This goal excludes
Japanese, additional subtitle grammars, ASR, TTS/audio gates A1 through A6,
Marker and WPF. The owner's 10 October scheduling request authorizes the defined
desktop integration, while unrelated translation experiments keep their current
workflow. Do not silently widen training, model or hardware scope.

Reuse the existing Translate Rust core, formats, SQLite and model provider through
Auralis adapters-translate. Preserve domain dependency direction. No subprocess
CLI wrapper, copied implementation, shared project-text database or second
scheduler may replace this integration. Translation inference remains in its
managed model process with machine admission compatible with the TTS boundary.

Work through the ready canonical tasks and missing gates: retained failures and
regressions, source-aware context and long-file quality, frozen model/profile
selection, durable controls, result history, migration, package installation,
native recovery and explicit review/export. A structurally valid needs_review
result cannot become approved speech input without an explicit review decision
bound to that result digest. A later edit invalidates that approval for the new
revision, while preserving the prior result and its review history.

Use real source/model evidence and the existing evaluation rules. Freeze input
splits, run budgets and stopping criteria before comparisons. Keep historical
profiles and raw rejected candidates. Do not fix benchmark cases with text
substitutions or repeatedly regenerate until a desired answer appears. Model
changes and optional training need their declared evidence branches.

## Required acceptance

| Gate                | Observable completion                                                                                                                                                            |
| ------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| INT-TR01 library    | CLI and Auralis call the same reusable core; exact cue identity, protected bytes, input rejection and independent builds pass for the declared formats                           |
| INT-TR02 durability | Pause, cancellation, worker/application death, stale attempts, resume, edit branches and cross-database publication gaps have retained regression and native evidence            |
| INT-TR03 desktop    | Native import, inspection, start, saved progress, pause/resume, paged history, editing, selection, review and immutable export work against isolated real data                   |
| INT-TR04 language   | The selected Chinese release satisfies its required G1 through G9 map, including natural long-file and independent source-aware review rather than mock or AI-only assertions    |
| INT-TR05 delivery   | A pinned committed candidate installs on clean Windows, acquires the declared model explicitly, restarts, translates offline and exports without developer checkout dependencies |

Use the existing exact Taskfile commands selected by scope, including task
docs:check and task plan:check, plus affected report/site checks in Translate.
Select Auralis Rust, job, frontend and native acceptance tasks from its Taskfile;
add bounded tasks before new probes. Do not run all model-dependent tasks together.

Record required external review or clean-machine gaps honestly, continue independent
work, and keep incomplete gates open. Preserve the owner identity for commits.
Commit and test Translate before updating Auralis's submodule pin to that exact
committed candidate. Audit the final package and gate map before completion.
This goal can finish without TTS, Caster, CONTRACT or Marker processes running.
