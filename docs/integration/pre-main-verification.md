# Pre-main integration verification

This candidate combines the release integration at `690ce974` with the media
library and project contract changes at `90e82df`. Translate remains pinned to
submodule commit `44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`.

## Integration decisions

- SQLite schema 22 preserves speech limits (18), WAV assembly (19), video
  assembly (20), the workspace library (21), and media receipts (22).
  Migration also recognizes the earlier library/media branch schemas 18 and 19,
  preserves their data, and rejects partial or unknown table sets.
- Speech retains CONTRACT 0.2.0. Media validation uses a separate CONTRACT 0.3.0
  installation and `project-runtime.json`; it does not replace speech receipts.
  Media package source: CONTRACT commit `0cc6898abb149eeb983c767655e5c88c3f3f854a`.
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

## Model-free verification

| Command                | Result                                                                                                    |
| ---------------------- | --------------------------------------------------------------------------------------------------------- |
| `task rs:pr`           | Passed: formatting, denied-warning Clippy and 759 Rust tests; 23 ignored tests remain outside this scope. |
| `task fe:e2e`          | Passed: all 35 guided workflow browser regressions.                                                       |
| `task ui:fixture`      | Passed: 32 full-workflow and 18 diagnostic fixture steps through the library.                             |
| `task ui:check:runner` | Passed: 33 runner isolation, evidence and lifecycle checks.                                               |
| `task q:global`        | Passed: media policy, dependency integrity, IPC, formatting and storage guards.                           |
| `task q:frontend`      | Passed: architecture, IPC parity, bounded modules and desktop policies.                                   |
| `task fe:test`         | All 675 tests passed. The overall command failed its unchanged coverage thresholds.                       |

The coverage gate currently measures 77.45% statements, 71.64% branches,
71.37% functions and 78.99% lines. Required thresholds remain 90%, 80%, 90%
and 92%, respectively. New library, asset and command interactions need more
instrumented tests before this branch is eligible for main. Production browser
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

- The instrumented frontend coverage gate fails at the percentages above.
- The dependency audit on 2026-10-27 reports 38 npm vulnerabilities (4 critical,
  11 high, 17 moderate, 6 low). `task sec:audit:rust` reports 14 vulnerabilities
  and one denied unsoundness warning. `task sec:deny:rust` accepts dependency
  sources but rejects advisories. Resolve these against the published advisories
  and rerun the complete gate before considering main.
- Real model execution, long-source playback, video acceptance and clean Windows
  remain separate acceptance stages.
