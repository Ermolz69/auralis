# Pre-main integration verification

## Current integrated verification

The final implementation is `5a5ca531c37fef4eade0633e276e5d6a12688b7e` on
`chore/pre-main-completion`, verified on 2026-10-28. The ordinary Windows
candidate below was built from that committed, clean source. Native metadata
verification failed and cleanup remains unconfirmed; technical readiness is not
established.

Subsequent commits retain evidence and update this record only; they are not
claimed as a new complete PR or native run.

The original integration combines `690ce974` and `90e82df`, with `b29719b`
as the reproduced pre-main baseline. Its results below remain historical.
Translate now selects published commit
`5d98cb3ca341c25bd446f7fc9d98d4b62ef551f5`.

| Metric                                    | Reproduced `b29719b` baseline | Final integrated run |
| ----------------------------------------- | ----------------------------: | -------------------: |
| Frontend tests passed                     |                           675 |                  906 |
| Statements                                |                        77.45% |               91.28% |
| Branches                                  |                        71.64% |               83.92% |
| Functions                                 |                        71.37% |               90.02% |
| Lines                                     |                        78.99% |               93.91% |
| npm package/advisory records              |                            38 |                    0 |
| Rust vulnerabilities / denied unsoundness |                        14 / 1 |                0 / 0 |

The unchanged coverage thresholds are 90/80/90/92. The combined frontend
baseline at `c18b7f4` already included the three original scoped branches and
measured 90.92/83.43/88.88/93.56 over the same 263 production modules. The
completion adds behavioral coverage rather than changing inclusion or discovery.
Different dependency/instrumentation versions make its counters the comparable
before/after inventory; the original reproduced baseline is retained separately.

- [Studio baseline, implementation and failed attempts](pre-main-studio-coverage.md).
- [Library baseline and implementation](library-coverage.md).
- [Combined frontend commands and failures](pre-main-frontend-completion.json) and
  [complete paired module inventory](pre-main-frontend-coverage.json).
- [All baseline advisories, constraints and ownership chains](pre-main-advisories.md).
- [Rust security and component regression verification](pre-main-rust-security.md).
- [Source-bound component wheel verification](pre-main-component-build.md) and
  [active source pin verification](pre-main-component-pins.md).
- [Final candidate commands and failed native acceptance](pre-main-candidate-verification.md),
  [structured artifact and step identities](pre-main-candidate-verification.json), and
  [fresh final 263-module inventory](pre-main-candidate-coverage.json).

### Complete local PR gate

The literal `task check:pr` returned **0** on clean `5a5ca53`. It includes the
complete frontend gate, Rust PR gate, documentation, security and global policies.

| Check                                        | Exit | Observed result                                                                                                                            |
| -------------------------------------------- | ---: | ------------------------------------------------------------------------------------------------------------------------------------------ |
| Complete frontend coverage stage             |    0 | 906/906 tests in 159 files; no skipped tests; all four coverage thresholds                                                                 |
| Production E2E / CSP smoke                   |    0 | 35/35 and 1/1                                                                                                                              |
| Rust PR gate                                 |    0 | Formatting and denied-warning Clippy; 758 top-level tests, two additional nested child-harness successes, 23 existing ignored, zero failed |
| Complete npm audit                           |    0 | No known vulnerabilities; all dependency categories retained                                                                               |
| Rust audit / deny                            |    0 | 570 dependencies, 1,381 advisories; no vulnerabilities or denied unsoundness; unchanged six allowed maintenance notices; sources accepted  |
| Documentation / global quality               |    0 | Contracts, formatting, lint and source/storage/IPC policies passed                                                                         |
| `task ui:check:runner`                       |    0 | 33/33 isolation, lifecycle and evidence controls                                                                                           |
| `task components:pins:check PYTHON=<python>` |    0 | 13/13 source/wheel controls; Task-local `-B` leaves source clean without an external bytecode setting                                      |

Node 24.14.0, pnpm 11.25.0, Task 3.50.0, Rust/Cargo 1.95.0,
cargo-audit 0.22.2 and cargo-deny 0.20.2 were used. The final repeated gate uses
`VITEST_MAX_WORKERS=2`, two Cargo jobs and two Rust test threads. Dependencies,
targets, temporary files and outputs belong to separate owned worktrees.

The preserved RustSec database is
`40f5be6fb12ed93fc591bf4ab23b7d793b3f2239`. The six transitive maintenance
notices and their existing workspace policy are recorded in the advisory inventory;
no new ignore, severity reduction or advisory suppression was added.

### Visuals and model-free browser workflows

`task check:storybook` returned 0: eight contract tests, 120 Chromium
interaction/accessibility cases and all 13 visual comparisons. `task fe:storybook`,
library and speech production smoke each returned 0. `task ui:fixture` returned 0
with 32/32 workflow and 18/18 diagnostic steps; all ten captured browser process
identities were released. These use the same final frontend implementation at
`17a786d`; later changes are tooling/evidence only.

One stale compact icon baseline initially failed: three icons registered after
the baseline added a 24-pixel row. Expected/actual/diff review confirmed the exact
change before that single image was refreshed. Ordinary comparison subsequently
passed; assertions, pixel thresholds and the other twelve baselines are unchanged.
Total bundle gzip is 158.57 KiB against the unchanged 160 KiB limit. Existing
lint/build warnings and remaining uncovered paths are retained in the inventories.

### Ordinary Windows candidate

`task media:verify` and `task ui:build` returned 0. The latter used its ordinary
release flags with inherited native-test and resource-stripping overrides cleared.
The build froze 1,982 application and 1,897 dependency source files and confirmed
an empty working diff. It did not bundle an installer.

| Artifact                           | Identity                                                                                     |
| ---------------------------------- | -------------------------------------------------------------------------------------------- |
| Executable                         | 23,408,640 bytes; SHA-256 `1880d2ca6e7d8ee70aa249881ce05a0eef68752f092ac4a1e329e75d932dda41` |
| Version 3 ordinary-release receipt | SHA-256 `21d0552f8be6ca0273c72714f94714ac684eb9c1e7e942f4be8b89a035f0d110`                   |
| Seven-resource array               | SHA-256 `6ce98d3adbe0e0a25de9e74c0f937a78e7f66cbc8d3887dddb03730ade1f46e9`                   |
| Implementation / Translate         | `5a5ca531c37fef4eade0633e276e5d6a12688b7e` / `5d98cb3ca341c25bd446f7fc9d98d4b62ef551f5`      |

The full receipt retains individual executable, provenance and license hashes.
Source/wheel validation separately confirms speech CONTRACT 0.2.0 at `13f4383`,
media CONTRACT 0.3.0 at `6ee129c`, TTS at `4966226` and CASTER at `2dbab64`.
All selected source refs are published. Media wheel SHA-256 remains
`1d7760d6784fc89a143c7d9fd0c7bf230f6315691a082bfa44823c920d32a20e`.

### Retained integration attempts

- A complete PR gate also passed on preceding source `f1a5049`. Its runtime
  coverage measured 91.32/84.01/90.02/93.91. The final source above is the
  authoritative run; the two-statement/five-branch counter difference is retained.
- A preflight returned 1 after a Python pin test emitted two untracked bytecode
  files. Those owned generated leaves were removed. A subsequent ordinary build
  was interrupted during release compilation (Task exit 201) before any candidate
  receipt existed, so the Task entrypoint could be corrected with `-B`.
  The final full gate and build then passed on `5a5ca53`.
- The original documentation check found generated-cache Markdown and false
  broken JSON evidence links. The corrected validator still rejects missing or
  misplaced evidence; the integrated documentation gate passes. A restricted
  Node child-process attempt exited 1 before assertions; the normal Task passed.
- Earlier coverage, bundle, fixture, visual and component-build failures remain
  recorded in their linked scoped reports. None is reclassified as a pass.

### Final-candidate native attempt

`task ui:assets:project` returned 0 with a new metadata-only seed: 3,261 files,
59,324,512 bytes, manifest SHA-256
`1da7c8d53cb1d6fc56dedc4ba12b7080f6e8d950660d84c3a01cd818c0525916`.
Its input manifest is pinned to
`fa328c73f77970d52cb0ff7881157bc83a59dac56caf03fed13bd51cfa38842f`.
Dependency/import checks verified speech CONTRACT 0.2.0 and separate media
CONTRACT 0.3.0. No engine dependencies, weights or voices were copied.

`task ui:freeze:project` returned 0 for new one-attempt case
`pre-main-metadata-20261028-final-v1`, bound to the executable above.
`task ui:smoke SPEC=<frozen-spec>` returned **201**: ten steps passed, two
failed and one was not run. Preflight, seed validation, ordinary launch and
schema 22 checks passed. Project creation failed after the browser page closed;
restart verification was not reached.

The ownership watcher recorded an ambiguous console-host process whose parent
identity had not been observed, then deliberately closed the page. The exact
settings-close locator and retained screenshot distinguish that action from the
native window-close button; an application crash is not established.
Cleanup failed with an observation-gap error, `cleanup_unconfirmed=true`, and
the session lock was retained. Read-only inspection found all twelve sampled
PID/creation identities absent and no component processes, but this alone cannot
prove that no unsampled descendant escaped. No retry, lock removal, unowned
termination or guard exception has been performed. The failed report and
incomplete diagnostic trace remain retained as failures.

## Historical integration decisions

## Integration decisions

- SQLite schema 22 preserves speech limits (18), WAV assembly (19), video
  assembly (20), the workspace library (21), and media receipts (22).
  Migration also recognizes the earlier library/media branch schemas 18 and 19,
  preserves their data, and rejects partial or unknown table sets.
- Speech retains CONTRACT 0.2.0. Media validation uses a separate CONTRACT 0.3.0
  installation and `project-runtime.json`; it does not replace speech receipts.
  Historical media package source was CONTRACT commit
  `0cc6898abb149eeb983c767655e5c88c3f3f854a`; the current portable source is
  `6ee129cfcf534a7ca865c17ca5fa8195e8a028be` with identical package code.
  Wheel SHA-256:
  `1d7760d6784fc89a143c7d9fd0c7bf230f6315691a082bfa44823c920d32a20e`.
- The library keeps dedicated source, translation, speech and results tools.
  Downloaded source identity is checked against ready project artifacts before
  offering its subtitle controls. Import history remains visible.
- **Пошаговая переозвучка** opens the existing project workflow, including import
  recovery, pins and the complete job queue. **Медиабиблиотека** returns to the
  library. Both are ordinary production interfaces with the same IPC boundary.
  Application update notifications have one owner across both views.
- Frozen evidence JSON and vendored source archives are excluded from prose
  formatting. Their original bytes and separate integrity checks are preserved.

## Historical model-free verification

| Command                | Result                                                                                                    |
| ---------------------- | --------------------------------------------------------------------------------------------------------- |
| `task rs:pr`           | Passed: formatting, denied-warning Clippy and 759 Rust tests; 23 ignored tests remain outside this scope. |
| `task fe:e2e`          | Passed: all 35 guided workflow browser regressions.                                                       |
| `task ui:fixture`      | Passed: 32 full-workflow and 18 diagnostic fixture steps through the library.                             |
| `task ui:check:runner` | Passed: 33 runner isolation, evidence and lifecycle checks.                                               |
| `task q:global`        | Passed: media policy, dependency integrity, IPC, formatting and storage guards.                           |
| `task q:frontend`      | Passed: architecture, IPC parity, bounded modules and desktop policies.                                   |
| `task fe:test`         | All 675 tests passed. The overall command failed its unchanged coverage thresholds.                       |

The historical coverage gate measured 77.45% statements, 71.64% branches,
71.37% functions and 78.99% lines. Required thresholds remain 90%, 80%, 90%
and 92%, respectively. At that point new library, asset and command interactions
needed more instrumented tests. The current results above supersede this failed
coverage gate. Production browser
tests are additional evidence and do not replace the coverage gate.

## Native metadata check

`task ui:assets:project` builds a new owned seed from an explicitly pinned source
manifest. It copies Python base files, speech package and dependency metadata,
and installs the separately pinned media CONTRACT. It does not copy model
weights, engine modules or voice references. Such a seed cannot establish TTS
readiness or synthesis acceptance.

`task ui:freeze:project CASE=<new-id> SEED=<seed-id>` freezes the retained
ordinary candidate and seed. `task ui:smoke SPEC=<frozen-spec>` drives project
creation and reopening through the library. Read-only checks additionally bind
the ready database receipt to the manifest hash and both contract versions.
Each case has one attempt and must confirm process release before another run.

The metadata seed resolves CONTRACT's complete base dependency closure from
installed distribution metadata, verifies copied files against the pinned source
manifest, and checks pure imports before allowing a native case. This includes
JSON Schema, locking and process-metadata dependencies; engine dependencies are
not installed. `task ui:diagnose:project PROFILE=<retained-profile>` performs only
read-only import and hash diagnostics.

The first two native cases failed because their metadata seeds lacked required
validation dependencies. Both retained failure reports and confirmed process
release. After replacing the incomplete seed, case
`PRE-MAIN-METADATA-20261027-v3` passed all 13 steps, including project creation,
restart, both contract versions, identical manifest hashes and graceful cleanup.
It used the ordinary candidate built from `85eb782`, SHA-256
`fb677065607eb1d195259b7504542a23dc78684730c0f4a7324e46aecabd70b1`.
The final backend guards were then verified with case
`PRE-MAIN-METADATA-20261027-v4`: all 13 steps passed on the ordinary candidate
built from `bb725d8`, SHA-256
`587cab055096af8a60313d5f83d00247e89b838e6b031438efd21cc7977ba27b`.
Both launches released their complete process trees without forced termination.

No model inference, real YouTube-to-WAV or MP4 acceptance, listening decision,
or clean Windows result is established by these checks. Existing reference
qualification remains unchanged.

## Remaining release gates

The [pre-main readiness tasks](pre-main-tasks.md) define implementation scope,
dependencies, verification commands and completion criteria for these gates.

- Native metadata verification of the new ordinary candidate failed and
  cleanup remains unconfirmed. Admission stays blocked until ownership/release
  is established; previous successful cases cannot substitute for this executable.
- Hosted Linux/optimized platform CI is not established by the local Windows
  gate. Existing ignored tests are not counted as passing.
- Real model execution, listening decisions, long-source playback, video
  acceptance and clean Windows remain separate acceptance stages.
- Main, existing projects, voice references and qualification decisions remain
  unchanged. Publishing working branches does not authorize a main merge.
