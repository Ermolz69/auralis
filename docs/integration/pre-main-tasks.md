# Pre-main readiness tasks

Close frontend coverage and dependency-security failures, then rebuild and
verify one ordinary Windows candidate. This backlog implements the remaining
pre-main gates in the [verification record](pre-main-verification.md).

Historical planning baseline: `4ca12b6`, checked on 2026-10-27. PM-001 reproduced
the implementation baseline at `b29719b`; the scoped inventories retain its
commands, tools and failures. Earlier candidate tests remain historical evidence;
dependency changes require fresh checks.
All 18 tasks initially had status `pending`. The execution index below includes
the [fresh full rerun](pre-main-full-rerun.md) after consolidation into
`feat/pre-main-integration` at `4310b36`, with CI-only follow-up `2fdb0c3`.
The complete local gate and retained ordinary Windows build at `5a5ca53` passed;
native metadata acceptance
failed and process cleanup remains unconfirmed. See the linked verification record for exact commands, failures,
hashes and scope. This record does not authorize a merge to main.

## Baseline and completion rules

| Metric              | Baseline                                              | Required result                          |
| ------------------- | ----------------------------------------------------- | ---------------------------------------- |
| Frontend statements | 77.45%                                                | At least 90%                             |
| Frontend branches   | 71.64%                                                | At least 80%                             |
| Frontend functions  | 71.37%                                                | At least 90%                             |
| Frontend lines      | 78.99%                                                | At least 92%                             |
| npm audit           | 38 findings                                           | Passing complete audit                   |
| Rust audit          | 14 vulnerabilities and one denied unsoundness warning | Passing audit and source/advisory policy |

The audit counts are a dated baseline, not a fixed list of findings. Reconcile
current advisories with the exact resolved dependency graphs in PM-001.
Each task closes only with its implementation commit, commands, exit codes and
relevant evidence. Record failures and their causes alongside the final result.

- Keep coverage thresholds, production-code inclusion, test discovery and
  security policies intact. Do not obtain a passing result through exclusions,
  empty assertions, generated happy-path cases or advisory suppression.
- Use separate work branches based on `feat/pre-main-integration`; preserve
  unrelated work and original Git history. Integrate completed changes before
  freezing the candidate.
- This workstream runs without model inference, model downloads or model
  doctors. Preserve voice files, qualification decisions and existing projects.
  Native work uses owned profiles and ordinary production controls.
- Business actions in UI acceptance run through the interface. File, database
  and hash checks provide additional evidence. Retain process-ownership and
  cleanup guards; unconfirmed cleanup prevents another native attempt.
- Passing these tasks establishes technical pre-main readiness. Real revoicing,
  listening, long-story quality, TXT, ASR and clean Windows retain their separate
  acceptance criteria.

## Task index

| ID                                                                      | Status   | Task                                          | Owner role            | Dependencies          |
| ----------------------------------------------------------------------- | -------- | --------------------------------------------- | --------------------- | --------------------- |
| [PM-001](#pm-001-record-the-current-gate-baseline)                      | verified | Record coverage and advisory inventories      | Developer             | None                  |
| [PM-002](#pm-002-test-studio-commands-and-dialog-control)               | verified | Studio commands and dialog control            | Frontend Developer    | PM-001                |
| [PM-003](#pm-003-test-library-operations-and-persistence)               | verified | Library operations and persistence            | Frontend Developer    | PM-001                |
| [PM-004](#pm-004-test-library-browser-and-asset-actions)                | verified | Library browser and asset actions             | Frontend Developer    | PM-001                |
| [PM-005](#pm-005-test-source-translation-speech-and-results-tools)      | verified | Source, translation, speech and results tools | Frontend Developer    | PM-001                |
| [PM-006](#pm-006-test-workspaces-drafts-and-window-lifecycle)           | verified | Workspaces, drafts and window lifecycle       | Frontend Developer    | PM-001                |
| [PM-007](#pm-007-close-all-instrumented-coverage-gates)                 | verified | Close all four coverage gates                 | Frontend Developer    | PM-002–PM-006, PM-008 |
| [PM-008](#pm-008-remediate-npm-dependencies)                            | verified | Remediate npm dependencies                    | Dependency Developer  | PM-001                |
| [PM-009](#pm-009-remediate-rust-networking-and-trust-dependencies)      | verified | Rust networking and certificate trust         | Rust Developer        | PM-001                |
| [PM-010](#pm-010-remediate-tokio-unsoundness)                           | verified | Tokio unsoundness and runtime behavior        | Rust Developer        | PM-009                |
| [PM-011](#pm-011-remediate-archive-extraction-dependencies)             | verified | ZIP dependencies and safe extraction          | Rust Developer        | PM-010                |
| [PM-012](#pm-012-resolve-remaining-advisory-and-source-policy-findings) | verified | Remaining advisory and source-policy findings | Dependency Developer  | PM-008, PM-011        |
| [PM-013](#pm-013-pass-the-complete-security-gate)                       | verified | Complete security gate                        | Security Reviewer     | PM-012                |
| [PM-014](#pm-014-verify-the-integrated-pr-gates)                        | verified | Integrated PR gates                           | Integration Developer | PM-007, PM-013        |
| [PM-015](#pm-015-verify-visuals-accessibility-and-fixture-workflows)    | verified | Visuals, accessibility and fixture workflows  | Frontend Developer    | PM-014                |
| [PM-016](#pm-016-build-and-pin-the-final-windows-candidate)             | verified | Build and pin the Windows candidate           | Release Developer     | PM-015                |
| [PM-017](#pm-017-verify-the-final-candidate-through-native-ui)          | failed   | Native UI verification of that candidate      | Release Developer     | PM-016                |
| [PM-018](#pm-018-complete-the-pre-main-acceptance-record)               | blocked  | Final evidence and readiness decision         | Maintainer            | PM-017                |

Coverage tasks may progress independently in their respective areas. npm and
Rust remediation may progress independently. PM-009–PM-011 intentionally run
in sequence because they update the same Cargo manifests and lockfile.
PM-014–PM-018 operate on the integrated result, not separate task branches.

## Coverage work

### PM-001 Record the current gate baseline

- Reproduce `task fe:test`, `task sec:audit:npm`, `task sec:audit:rust` and
  `task sec:deny:rust`. Run audits separately so one failure does not hide another.
- For coverage, retain totals and missed statements, branches, functions and
  lines per production module. Identify residual gaps outside the initial areas.
- For security, record advisory ID, resolved version, dependency path, package
  owner, affected use and patched-version constraints. Separate duplicate paths
  from distinct advisories. Include development tooling and the full Cargo lock.
- **Done when:** both inventories are tied to the baseline commit and lockfiles,
  every finding has an implementation task, and no gate is reported as passing
  merely because its command did not run.

### PM-002 Test Studio commands and dialog control

- Cover `src/app/studio/ui/StudioCommands.tsx` and the command-menu model under
  `apps/desktop`: file/project/tool menus, contextual targets, material selection,
  rename, removal, import and copy actions.
- Exercise keyboard navigation, focus restoration, cancellation, operation
  progress, busy guards, failed publication and actionable errors. Assert that
  ambiguous or repeated clicks do not start a second operation.
- **Verify:** `task fe:test:components -- <affected-tests>` and
  `task fe:test:integration -- <affected-tests>`.
- **Done when:** assertions cover visible state and exact admitted operations,
  including failure and recovery paths; no production behavior is mocked away
  at the boundary under test.

### PM-003 Test library operations and persistence

- Cover `src/entities/library/model/operations.ts`, `LibraryProvider.tsx`,
  `actions.ts` and related catalog behavior under `apps/desktop`.
- Verify owner creation and rollback, supported file imports, cancelled picking,
  publication readiness, independent copies, failed/cancelled copies, stale
  revisions, bounded conflict handling, refresh ordering and listener cleanup.
- **Verify:** `task fe:test:unit -- <affected-tests>` and
  `task fe:test:components -- <affected-tests>`.
- **Done when:** persistence failures preserve accepted state and expose recovery;
  failed or partial assets never become successful library results.

### PM-004 Test library browser and asset actions

- Cover `src/widgets/library-browser/ui` and
  `src/entities/library/ui/VideoPreview.tsx` under `apps/desktop`.
- Verify creation validation, filtering, selection, properties, version history,
  project navigation, context menus, keyboard actions and drag/drop copies.
  Include unavailable runtime, missing preview and failed mutation states.
- **Verify:** `task fe:test:components -- <affected-tests>` and
  `task fe:storybook:library`.
- **Done when:** user-visible operations and failure states have instrumented
  assertions; inaccessible controls and hidden elements cannot satisfy them.

### PM-005 Test source translation speech and results tools

- Cover `StudioToolPanel.tsx` and `ui/tools/{Source,Translate,Speech,Results}Tool.tsx`
  in `apps/desktop/src/app/studio`.
- Verify project/material ownership, ready source identity, translation-to-speech
  lineage, version-bound approval, saved drafts, selected results and verified
  playback resolution. Include stale responses, missing artifacts and API errors.
- **Verify:** `task fe:test:components -- <affected-tests>` and
  `task fe:test:integration -- <affected-tests>`.
- **Done when:** tests reject foreign or partial source data, invalid approval
  reuse and unverified playback; fixture audio does not imply real synthesis.

### PM-006 Test workspaces drafts and window lifecycle

- Cover Studio workspace/tab behavior, `tabDraft.ts`, `useWindowChrome.ts` and
  `src/app/workflow/GuidedWorkspace.tsx` under `apps/desktop`.
- Verify tab and material switching, draft persistence, confirmed close/discard,
  settings return, project removal, native window failures and the single owner
  of application update notifications. Audit residual uncovered legacy flows,
  shared validation, voice-catalog UI and project controls from PM-001.
- **Verify:** focused `task fe:test:unit`, `task fe:test:components` and
  `task fe:test:integration` commands with `-- <affected-tests>`.
- **Done when:** switching, asynchronous cleanup and error handling preserve the
  intended context; the remaining gap list has no unassigned production modules.

### PM-007 Close all instrumented coverage gates

- Integrate PM-002–PM-006 with the npm remediation. Recalculate uncovered paths
  and add behavior-driven tests for remaining gaps, including branches and
  functions, rather than targeting line coverage alone.
- **Verify:** `task fe:test`, `task check:frontend` and `task q:frontend`.
- **Done when:** all four unchanged thresholds pass in one complete run, test
  projects and production inclusion remain intact, and no skipped cases or
  removed assertions conceal a regression.

## Dependency security work

### PM-008 Remediate npm dependencies

- Resolve every current npm advisory through compatible direct-dependency
  updates or narrowly justified transitive overrides. Check the complete
  Storybook, Vitest, Vite, bundling and documentation dependency paths.
- Select versions from current upstream advisories and release notes. Generate
  the lockfile through `task fe:install-update`; do not edit resolved entries
  manually. Keep runtime and development dependencies within audit scope.
- **Verify:** `task fe:install`, `task sec:audit:npm`, `task check:frontend` and
  `task check:storybook` after integration with coverage changes.
- **Done when:** a frozen install resolves the reviewed versions, npm audit
  passes and affected build/test APIs work without severity downgrades or ignores.

### PM-009 Remediate Rust networking and trust dependencies

- Resolve the reported `quinn-proto`, `rustls`, `rustls-webpki` and
  `webpki-root-certs` advisories and any findings in the same dependency chains.
- Trace ownership with `task rs:tree -- --invert <crate> --target all`. Use
  reviewed `task rs:update -- --package <crate> --precise <version>` updates;
  change parent requirements when resolution alone cannot select a safe version.
- **Verify:** `task rs:pr`, `task sec:audit:rust` and `task sec:deny:rust`.
- **Done when:** these advisory paths are removed from the complete lockfile and
  TLS, download, package-delivery and cancellation regressions still pass.

### PM-010 Remediate Tokio unsoundness

- Resolve `RUSTSEC-2026-0386` using an upstream-supported dependency change.
  Check toolchain compatibility and all dependent runtime features.
- Preserve bounded cancellation, job ownership, cleanup failure propagation and
  process-tree confirmation. Do not dismiss the advisory solely because the
  current developer host is Windows.
- **Verify:** `task rs:pr`, `task rs:test:component-cleanup`,
  `task rs:test:job-tree`, `task sec:audit:rust` and `task sec:deny:rust`.
- **Done when:** the unsoundness finding is resolved and lifecycle regressions
  pass without disabling checks or treating unknown process exit as success.

### PM-011 Remediate archive extraction dependencies

- Resolve all reported `zip` extraction advisories through the responsible
  dependency chain. Assess major-version/API changes before selecting an update.
- Retain regressions for traversal, escaping symlinks, unsafe permissions,
  incomplete archives, interrupted installation and publication only after
  verification. Keep managed extraction destinations confined to owned roots.
- **Verify:** `task rs:pr`, `task sec:audit:rust`, `task sec:deny:rust` and
  the affected model-free package/extraction tests through Taskfile entrypoints.
- **Done when:** vulnerable archive packages are absent from the lockfile and
  archive failures cannot publish ready packages or alter external files.

### PM-012 Resolve remaining advisory and source policy findings

- Rerun both ecosystems' audits and reconcile every residual or newly published
  finding with PM-001. Review maintenance notices according to the existing
  direct/transitive policy; do not silently classify a vulnerability as a notice.
- If a responsible dependency requires a Translate submodule change, prepare and
  verify a committed component candidate first, then advance the submodule pin
  in Auralis. Keep unrelated sibling changes and source archives untouched.
- Preserve the SQLite-only graph and the maintained GLib/process ownership
  backports. Review provenance and platform checks when their sources change.
- **Done when:** no unexplained finding or disallowed source remains. An upstream
  or Maintainer dependency stays explicitly blocked; it cannot be closed by a
  suppressed advisory, a stale database or an uncommitted component pin.

### PM-013 Pass the complete security gate

- Run a fresh audit against the final integrated manifests and lockfiles.
  Record scanner/advisory-data versions, resolved graphs and command outcomes.
- **Verify:** `task sec:audit:npm`, `task sec:audit:rust`,
  `task sec:deny:rust` and `task check:quality:security`.
- **Done when:** every command succeeds under the unchanged policy. Retain any
  policy-allowed maintenance notices in the report with their exact scope.

## Integrated candidate verification

### PM-014 Verify the integrated PR gates

- Integrate all implementation branches and pin the final source revision.
  Run from frozen dependencies; preserve failure evidence and repair actual
  regressions before repeating the affected checks.
- **Verify:** `task check:pr` and `task ui:check:runner`.
- **Done when:** the complete local PR gate and runner checks pass on the same
  source and lockfiles; separately passing task branches do not qualify.

### PM-015 Verify visuals accessibility and fixture workflows

- Verify both the media library and guided workflow, menus, tabs, dialogs,
  settings, errors, progress, keyboard navigation and narrow/wide layouts.
  Review any visual differences before changing screenshot baselines.
- **Verify:** `task check:storybook`, `task fe:storybook`, `task fe:e2e`, `task fe:smoke`,
  `task fe:smoke:library EVIDENCE=<evidence-directory>`,
  `task fe:smoke:puza SCREENSHOT=<screenshot-file>` and `task ui:fixture`.
- **Done when:** visual/accessibility checks and fixture workflows pass with
  traces/screenshots retained and synthetic results clearly labelled.

### PM-016 Build and pin the final Windows candidate

- Commit the integrated implementation before building. Verify the pinned
  Translate checkout, media tools, ordinary build flags and both CONTRACT
  installations. Freeze source inputs for the entire build.
- **Verify:** `task media:verify` and `task ui:build`.
- **Done when:** the retained executable, resources, source revision, dependency
  versions and build-input receipt have verified hashes. Reject source drift
  during the build; an earlier candidate cannot stand in for the new one.

### PM-017 Verify the final candidate through native UI

- Prepare a new metadata seed with `task ui:assets:project` using explicit pinned
  inputs. Its dependency-closure/import checks must pass before freezing a case.
  Reuse an existing seed only after verifying its complete frozen manifest.
- Run `task ui:freeze:project CASE=<new-id> SEED=<verified-seed>` followed by
  `task ui:smoke SPEC=<frozen-spec>`. Drive creation, validation and reopening
  through the ordinary WebView2 interface in an owned profile.
- **Done when:** all 13 existing native metadata steps pass on PM-016's exact
  executable; both CONTRACT versions, schema 22, manifest hashes and complete
  process release are confirmed. Retain reports, screenshots and traces. A failed
  attempt stays failed; repair its cause and freeze a new case before retrying.

### PM-018 Complete the pre-main acceptance record

- Update the verification record with final implementation commits, dependency
  changes, candidate hashes, all required gate outcomes and remaining scope.
  Confirm that the native evidence belongs to the same runtime source inputs.
- Check that main, existing projects, voice references and qualification
  decisions were preserved. Keep real-model and clean-Windows acceptance separate.
- **Done when:** every prerequisite is complete, working changes are committed,
  no required command is failed/blocked/not run, and the Maintainer has a concrete
  reviewable candidate. Record technical readiness; merging or publishing remains
  a separate Maintainer action.
