# AUR-M02 acceptance report — translation integration documentation

## Candidate and ownership

- Repository: Auralis (`origin`: `git@github.com-personal:Ermolz69/auralis.git`).
- Branch/worktree: `docs/auralis-translation-guide` at `.cache/worktrees/auralis-translation-guide`.
- Recorded base: `43136f3fc00d0ad5754f293d7a535dfc47f75739` (maintenance packet present).
- Tested source candidate: `33427edca1cca8621a9e53eb8b51138e9c333406`.
- Pinned Translate gitlink: `44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`, unchanged. The sibling Translate HEAD was separate and was not used as evidence for this pin.
- Owned paths: `docs/translation/README.md`, `docs/translation/history.md`, and this report. No root documentation index, code, nested checkout or sibling Translate path was edited.
- Author and committer used the computer's verified global `Ermolz <00ermzahar@gmail.com>` identity; no local or environment override was present.

## Responsibility and behavior matrix

| Boundary              | Before                                                                                                                                | After                                                                                                                                                                    |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Current user route    | One 706-line README mixed setup, workflow, dated observations and open gates.                                                         | A 233-line current guide puts setup, import, run/pause/resume, committed progress, comparison, edits, history, explicit attachment, review and export in workflow order. |
| Historical evidence   | Dated observations and named checks were embedded in the main entrypoint.                                                             | A 712-line history page retains the original chronology and evidence links in order, with a date/scope notice.                                                           |
| Status interpretation | Earlier pending statements for events, result history and concurrent starts could be read as current despite later successful checks. | The guide cites later named successes and keeps the remaining concurrency, installation and language-quality gates open.                                                 |
| Runtime configuration | Package and manual setup appeared in distant chronological sections.                                                                  | The guide states selected checked package precedence, invalid-selection failure, and manual JSON fallback only with no selected package.                                 |
| Application behavior  | Documentation-only assignment.                                                                                                        | No code, command, UI, model profile, database schema, configuration default or observable behavior changed.                                                              |

## Old-to-new navigation

All 22 old README heading fragments still occur in the current README, so existing `README.md#...` links retain a local destination. Each section except the H1 points to the matching `history.md#...` heading. The historical H1 was changed only to identify the archive.

| Former README fragment                                     | Current guide destination | Dated record                  |
| ---------------------------------------------------------- | ------------------------- | ----------------------------- |
| `#auralis-translate-integration-status`                    | Current H1                | `history.md` introduction     |
| `#managed-runtime-setup-for-the-current-development-build` | Runtime setup             | Same fragment in `history.md` |
| `#implemented-boundary`                                    | Workflow                  | Same fragment in `history.md` |
| `#project-deletion`                                        | Project deletion          | Same fragment in `history.md` |
| `#startup-publication-recovery`                            | Recovery                  | Same fragment in `history.md` |
| `#checks`                                                  | Evidence/limits table     | Same fragment in `history.md` |
| `#pinned-package-repair-and-delivery-checks`               | Delivery limits           | Same fragment in `history.md` |
| `#guarded-attempt-admission`                               | Historical redirect       | Same fragment in `history.md` |
| `#cooperative-preparation-cancellation`                    | Historical redirect       | Same fragment in `history.md` |
| `#accepted-host-work-and-preparation-status`               | Historical redirect       | Same fragment in `history.md` |
| `#admission-completion-cleanup-precedence`                 | Historical redirect       | Same fragment in `history.md` |
| `#native-accepted-worker-preflight-pause`                  | Historical redirect       | Same fragment in `history.md` |
| `#native-initial-model-hash-pause`                         | Historical redirect       | Same fragment in `history.md` |
| `#native-concurrent-starts-during-preparation`             | Historical redirect       | Same fragment in `history.md` |
| `#committed-translation-events`                            | Current event section     | Same fragment in `history.md` |
| `#ready-result-history-and-explicit-selection`             | Current history section   | Same fragment in `history.md` |
| `#historical-edit-foundation`                              | Historical redirect       | Same fragment in `history.md` |
| `#manual-publication-ordering`                             | Historical redirect       | Same fragment in `history.md` |
| `#historical-application-save-and-recovery`                | Historical redirect       | Same fragment in `history.md` |
| `#historical-desktop-contracts-and-controls`               | Historical redirect       | Same fragment in `history.md` |
| `#native-historical-branch-and-reopening`                  | Historical redirect       | Same fragment in `history.md` |
| `#native-historical-core-commit-interruption`              | Historical redirect       | Same fragment in `history.md` |

The Markdown checker resolves files but does not validate anchors. I manually compared the old heading list, both new heading lists and every `history.md#...` destination. The current guide owns the root-facing route; AUR-M01 should consider adding a separate history link to `docs/README.md` during its final pass. AUR-M02 did not edit that shared index.

## Current capability and evidence

| Capability                                            | Current source/evidence                                                                                                            | Limit                                                                                                          |
| ----------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------- |
| Strict SRT/WebVTT original import and separate output | Auralis import/UI paths and named two-database/native records retained in `history.md`; pinned Translate parser/storage contracts. | Supported grammar only; no source-language quality claim.                                                      |
| Checked runtime and package selection                 | `src-tauri/src/bootstrap/mod.rs`, package controls and the dated package/native records.                                           | Checked development profile; unseeded clean Windows install and signing open.                                  |
| Start, pause/resume and committed progress            | Registered IPC, workspace controls, Translate committed checkpoint ownership, named pause checks.                                  | No AUR-M02 model run; broader interleavings open.                                                              |
| Events, history and older-base edits                  | Later named native event, selection/reopen and result-gap records; corresponding UI/storage paths.                                 | Two named Start races do not close the full concurrency matrix; staged/journal interruption cases remain open. |
| Explicit review and verified export                   | `docs/architecture/008-translation-review-decision.md`, registered IPC, UI, application tests and pinned result contracts.         | Implementation evidence only here; no AUR-M02 native export or clean-machine acceptance.                       |
| Cross-database publication                            | Auralis ready-artifact/outbox code and dated recovery records; Translate result storage remains authoritative.                     | Pending and ready are distinct; publication and structural `needs_review` are not human approval.              |

## Checks and change accounting

- Baseline at `43136f3`: `task check:docs` exit 0 (5 validator tests, 56 Markdown files indexed, 57 linted); `task check:quality:docs` exit 0; `task docs:integration` exit 0 (12 linted/integration-index files).
- After history extraction: `task q:format-write -- docs/translation/README.md docs/translation/history.md` exit 0; `task check:docs` exit 0 (57 indexed, 58 linted).
- Source candidate `33427ed`: `task q:format-write -- docs/translation/README.md` exit 0 (unchanged); `task check:docs` exit 0 (5 tests, 57 indexed, 58 linted, zero lint errors); `task check:quality:docs` exit 0; `task docs:integration` exit 0 (12 files, zero lint errors). `git diff --check` was clean.
- With this report added: `task q:format-write -- docs/maintenance/reports/AUR-M02.md` exit 0; `task check:docs` exit 0 (5 tests, 58 indexed, 59 linted, zero lint errors); `task check:quality:docs` exit 0; `task docs:integration` exit 0 (12 files, zero lint errors).
- Native, model, clean-install, language-quality, speech, backend and frontend checks were not rerun for this documentation-only change. Existing dated results are cited as historical evidence, not claimed as new acceptance.
- Commit `b0c6ca068d42c8607df283466ab437186c86d28a`: history extraction, ordinary diff 741 insertions/706 deletions; copy-aware review showed the history page as a 99% copy with 7 insertions/1 deletion. The large raw count is the mechanical 706-line move.
- Commit `33427edca1cca8621a9e53eb8b51138e9c333406`: current guide, 233 insertions/29 deletions. Against base, ordinary diff is 898 insertions/659 deletions; move-aware view is 240 insertions/707 deletions, including the mechanical copy.
- Generated files: none. Retained failures: none from these docs checks; historical failure and open-gate records remain in `history.md`.

## Review and delivery

The receiving reviewer should read both documentation commits and this report, verify the pinned gitlink and unchanged public behavior, follow representative old fragments into the history page, and rerun the three required docs tasks on the committed candidate. Reconcile the guide after AUR-M03 and AUR-M06 are accepted; especially recheck executor wording and any final Translate pin. This report is not a release qualification or reviewer acceptance verdict.

Branch publication target: `origin/docs/auralis-translation-guide`. No PR was requested. Final push status is verified after committing this report and recorded in the delivery response.
