# Caster integration execution record

Frozen: 10 October 2026. Scope: manual catalog and explicit role assignment only.

## Task and gate map

| ID      | Deliverable                                                 | Required evidence                                                                                                     |
| ------- | ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| INT-V01 | Installable, model-free Caster facade and standalone worker | External-install catalog and cast commands; existing entry points pass                                                |
| INT-V02 | Strict voice identity and compatibility                     | Legacy and unsupported input rejection, exact hashes, qualification and journal checks, immutable WAV source          |
| INT-V03 | Recoverable manual cast revisions                           | Concurrent and stale writes, interruption at each publication boundary, idempotent replay, previous revision retained |
| INT-V04 | Project-scoped Auralis adapter and native UI                | Typed IPC, bounded pages, inspected and playable prepared audio, complete managed cast artifact                       |
| INT-V05 | Reproducible delivery                                       | Pinned CONTRACT/Caster wheels and runtime, restart evidence, unchanged tracked WPF files                              |

The mandatory operation set is `capabilities`, catalog page, voice inspection,
reference inspection, supported reference import or creation, explicit compatibility
check, role snapshot, manual assignment and revision inspection. Auralis supplies
roles or receives them as explicit input. No speaker inference, LLM autocasting,
Marker, synthesis, or generated preview is part of this candidate. The current
`caster.ps1 autocast` behavior remains supported independently.

Accepted manifest is CONTRACT voice v2; accepted logical assignment is CONTRACT
cast v2. Legacy manifests are visible as unsupported entries until an explicit
existing import operation creates a separate v2 copy. Engine IDs are the existing
`voxcpm2` and `espeech`; compatibility requires an exact qualified reference for
the requested engine and emotion. Qualification requires a genuine reviewed
report and a clean journal. No test fixture may promote a user voice.

Engineering fixtures use isolated voice-store and project copies with legally
usable generated PCM WAV bytes. One catalog page is at most 50 entries, a worker
frame is at most 1 MiB, and a request has a 30 second deadline. Tests perform no
model load, synthesis, TTS doctor or WPF launch. Retry budget is zero for semantic,
identity and stale-revision failures; transport uncertainty must reconcile by
idempotency key before any repeat. Existing user voice stores are read-only.

The frozen CONTRACT candidate from INT-C01–C05 is a production prerequisite.
Until it is accepted, Caster engineering checks cannot establish production
release readiness. Actual qualified-reference acceptance additionally requires
a genuine existing human-reviewed report; the currently inspected Caster store
contains no such report. The assembly goal owns cross-module generated preview.

## Candidate verification, 10 October 2026

- CONTRACT manual cast source: `5c33e2d`; Caster library source: `2756ead`.
  The installed runtime was built from clean archives of these commits with
  pinned package versions `0.2.0` and a SHA-256 wheel receipt.
- `task check:manual-cast -- -x --tb=short`: 8 passed. `task check:python --
-x --tb=short` in CONTRACT: 375 passed, 1 skipped. `task check:python -- -x
--tb=short` in CASTER: 47 passed. These use synthetic qualification fixtures.
- `task package:build`, `task package:install:check`, and `task package:install`
  passed from the clean source archives outside developer checkouts. The install
  check reported catalog count 1 and persisted cast revision
  `c8b0e6d4dc12b3962df68fe3a584f00fa41a4dbfa2e9ea0fc815acb65c9f7c4b`.
- `task rs:test:caster` passed its application artifact and desktop manifest
  tests. `task rs:test:caster:installed` passed against the installed worker:
  catalog, prepared WAV hash, compatibility, assignment, managed revision file,
  source immutability, restart read, and invalid project path rejection.
- `task fe:typecheck`, `task fe:lint`, `task fe:test:unit --
casterValidation.test.ts` (3 passed), and `task fe:test:components --
VoiceCatalog.test.tsx` (2 passed) passed.
- `task desktop:e2e:native` and `task desktop:e2e:native:caster` passed with
  isolated data. The Caster scenario exercised native catalog controls,
  reference audio metadata, explicit role selection, typed IPC, the installed
  worker, a ready managed SQLite artifact, and the same revision after restart.
- `task rs:fmt:caster`, `task fe:fmt:caster`, and `task docs:integration` passed.
  The workspace-wide `task rs:fmt` still reports the concurrently edited
  `crates/adapters-storage/src/sqlite/translation_links/history.rs` outside this
  candidate. Caster's tracked WPF files were unchanged.

This verifies engineering integration with an isolated synthetic voice store.
Production qualification remains open until an existing genuine human-reviewed
report and the frozen CONTRACT candidate are accepted. Automatic casting and
generated TTS preview remain separate work.
