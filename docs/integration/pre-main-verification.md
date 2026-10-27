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
| `task rs:pr`           | Passed: formatting, denied-warning Clippy and 757 Rust tests; 23 ignored tests remain outside this scope. |
| `task fe:e2e`          | Passed: all 35 guided workflow browser regressions.                                                       |
| `task ui:fixture`      | Passed: 32 full-workflow and 18 diagnostic fixture steps through the library.                             |
| `task ui:check:runner` | Passed: 33 runner isolation, evidence and lifecycle checks.                                               |
| `task q:frontend`      | Passed before the final formatting pass; final validation is pending.                                     |
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

Native candidate validation is pending. No model inference, real YouTube-to-WAV
or MP4 acceptance, listening decision, or clean Windows result is established by
these checks. Existing reference qualification remains unchanged.
