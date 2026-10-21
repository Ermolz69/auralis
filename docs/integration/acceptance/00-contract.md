# INT-C01–INT-C05 shared contract acceptance

Candidate date: 10 October 2026. This record applies to the model-free native
Windows foundation defined in [the frozen scope](../goals/00-contract-freeze.md).
It does not qualify Marker output, translation language quality, synthesis,
listening quality or video timing.

## Candidate identity and installation order

| Repository | Implementation source                                                                                                                                              | Role                                                                                  |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------- |
| CONTRACT   | `ee85cc6e7c267e77a7b9ca4d4ce748be9da42b58` from `95d3efbbbe15767cf68cbb8995e19d08511ba681`                                                                         | Canonical schemas, validators, CLI, worker and reproducible wheel                     |
| Caster     | `837119efee6dfd36eb647053c4b8867f9d702134` from `e5030f53bc20e6e6e0e7d35b9acec6ee253c56b7`                                                                         | Public qualification and manifest operations, exact `voxscript-contract==0.2.0`       |
| Auralis    | `519cbe03ef49e9235ae2f51033d90ba98df61f6e` after implementation commit `78b039bd5d13c9e99da76cf96f64ca790bedd25c`, from `d96202532bbe546caa42288347572078afe23e70` | Typed boundaries, pinned validator adapter, shared admission and owned processes      |
| Translate  | pinned Auralis submodule `1b888d0b7eb78a67794445fcbc51eb06329f9b64`                                                                                                | Existing Rust API and separate immutable-source translation                           |
| TTS        | fetched `origin/main` baseline `f23224510d3b04718a1636a816c3d387be0d0a3f`; no candidate change                                                                     | Existing `source.md` plus cast v1 render and shared-project preflight remain separate |

CONTRACT must be installed before the dependent Caster and Auralis candidates.
The Auralis branch contains 96 pre-existing local commits after its current
`origin/main`; review or publication must use the intended integration base.
The primary global Git author and committer identity is
`Ermolz <00ermzahar@gmail.com>`; no local or environment override was used.

## Exact package and schema pins

The distribution is `voxscript-contract` 0.2.0. The model-free wheel
`voxscript_contract-0.2.0-py3-none-any.whl` has SHA-256
`22009323ebbbc1eca6935264da9781dfa4637d9b3177e6847aa04c06fa29290e`.
`task package:wheel` fixes `SOURCE_DATE_EPOCH` at `1791639837`; two consecutive
builds from the candidate source produced this same wheel hash.
Auralis embeds these exact schema SHA-256 values in
`crates/adapters-contract/contract-pin-v1.json` and requires the installed
worker to report them before validation:

| Packaged schema                   | SHA-256                                                            |
| --------------------------------- | ------------------------------------------------------------------ |
| `speech-plan-v1.schema.json`      | `9bb9889612007b9986e3bb46d08790bd6cca5ef408f9d094235f53fa0085f8dc` |
| `voice-snapshot-v1.schema.json`   | `ead637ac05c567cfbc90ca0a356ec23563cefd2986fdccc9d3e5fec1ab4bb0d2` |
| `segment-result-v1.schema.json`   | `b3f66e7e76be2575d979f13c6c09ca8ad30cadad7071900560f9d7e16cec25fa` |
| `synthesis-result-v1.schema.json` | `b1b4fdd09d6b41f435e8666bc17ef9a62aefc6ed26880ece33520a25a924da09` |

The positive reviewed inputs have SHA-256
`65deb43bd9d60286182f62a5b855cb1adcf5edebd486cfe11a60d1897e57ca93`
(subtitle) and
`e7c66c40423bce9107c4f97026bde48eaae9f6acfa6e803da1612920788be422`
(plain text). The generated plan files have SHA-256
`142c180a923cdfc4c63bedca8dcb1ad773c5d4c7ac1ae130a9d6d09848bb067e`
and
`e1f96e2d98f8f94343662813c24b73b0304ae429464633872fba866f5c7781f6`;
the synthetic complete results have SHA-256
`82c0d04f790f46cb0f150fcb85ee0a1b99faef078a76a6d161e67c9e26d44327`
and
`a4be5e778fa56995cd0b5321ebc288d95697cd559656c34a45475f7bb14ee1d1`.
These hashes identify synthetic fixtures, not real source media or audio.
The plans' internal `plan_sha256` identities are
`bdffa22cfe2443be60fddbcceb84ac5add6b1fd864319a49ea3db124eec543c1`
(subtitle) and
`62159fff481a88288335db0f5a610a1e6efd5c987aad861f2267f30c20d7bf67`
(plain text); they intentionally differ from the SHA-256 of each serialized
file.

The dependency records are CONTRACT `ci/uv.lock` and
`ci/requirements-model-free.txt`, Caster `uv.lock` and `pyproject.toml`, and
Auralis `Cargo.lock`. The worker protocol and all four new data schemas are
version 1. Package, protocol and data versions advance independently. No
implicit migration of shared-project v1, cast v2, voice v2 or legacy TTS
cast v1 is performed.

## Compatibility matrix

| Consumer or producer | Accepted boundary                                                                                                  | Rejection or preservation evidence                                                                                                                      |
| -------------------- | ------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| CONTRACT CLI         | Reviewed subtitle and reviewed plain-text v1 inputs produce speech-plan v1; complete synthetic result v1 validates | Conformance and installed-wheel checks reject changed schema versions, identity mismatches and incomplete coverage.                                     |
| CONTRACT worker      | Protocol v1 capability negotiation and bounded validation requests                                                 | Process tests reject malformed, oversized, invalid UTF-8, duplicate-key and conflicting duplicate-request frames. No synthesis operation is advertised. |
| Auralis              | Pinned package 0.2.0, exact schema hashes, typed Rust speech ports and installed validator worker                  | Adapter tests reject wrong pins, stale attempts, EOF and unknown completion; host ownership and admission checks pass.                                  |
| Caster               | Public voice v2 and cast v2 qualification operations with exact CONTRACT 0.2.0 wheel                               | Its 43 model-free tests pass, including existing editor and worker behavior.                                                                            |
| Translate            | Existing pinned Rust library and immutable-source project behavior                                                 | `task rs:test:translate` passes; no Python worker routing is introduced.                                                                                |
| TTS                  | Existing `source.md` and legacy cast v1 inputs remain separate                                                     | No TTS source change is part of this candidate; no model or inference was launched and no qualification is claimed.                                     |

## Producer and consumer commands

From CONTRACT, with Python 3.12 and its recorded model-free requirements:

```text
task setup:model-free PYTHON_BIN=<python-3.12>
task fixtures:build
task check:conformance
task check:model-free
task package:wheel
AURALIS_ROOT=<auralis-integration-worktree> task check:installed
voxscript-contract --json speech build REVIEWED_INPUT PLAN_OUTPUT
voxscript-contract --json speech validate PLAN_OUTPUT --result SYNTHETIC_RESULT
```

From Auralis:

```text
task check:integration:contract
task rs:test:translate
task check:jobs
task check:docs
task check:quality:docs
task rs:fmt
task rs:clippy
task rs:clippy:native-e2e
task media:verify
```

From Caster, supply the exact wheel file above:

```text
task setup:model-free PYTHON_BIN=<python-3.12> CONTRACT_WHEEL=<exact-wheel>
task check:model-free
```

## Gates and observed evidence

| Gate                  | Evidence                                                                                                                                                                                                                                                                                                                                                                                                                                                 |
| --------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| INT-C01 data          | `task check:conformance`: 32 passed. Both reviewed modes, verified voice projection, qualified reference identity, schema versions, cast and plan digests, result identity and exact coverage have positive and negative fixture checks. Full committed CONTRACT suite: 386 passed, 1 skipped.                                                                                                                                                           |
| INT-C02 compatibility | CONTRACT CLI and package use the same validators as the worker. Committed Caster reinstalled the exact 0.2.0 wheel: `task check:model-free`, 43 passed. `task check:installed` from a separate environment accepted both reviewed producers and rejected changed versions; Auralis adapter parity passed. Existing CONTRACT public facade and legacy tests are included in its full suite. TTS compatibility is a source-level observation only.         |
| INT-C03 transport     | Python worker tests cover version negotiation, duplicate replay/conflict, exactly 32 requests, malformed/partial/oversized/invalid UTF-8 and duplicate-key frames. Auralis synthetic worker tests cover stale attempts, premature/partial EOF, malformed events, unknown completion and stderr isolation. Direct installed adapter repeat: 3 passed.                                                                                                     |
| INT-C04 ownership     | `task check:integration:contract` passed: 19 port tests, one pin test and five synthetic admission tests; the installed tests were run separately. `task check:installed` passed the Rust/portalocker cross-process lock test and three installed adapter tests. The synthetic owned process tree exits before timeout is returned. `task rs:test:translate` passed, and `task check:jobs` passed its Rust stages and 89 frontend tests across 11 files. |
| INT-C05 delivery      | The five implementation commits, reproducible wheel, schema and fixture hashes, lockfiles, producer/consumer commands and compatibility matrix are recorded here. `task rs:fmt`, `task rs:clippy`, `task rs:clippy:native-e2e`, `task media:verify`, `task check:docs` and `task check:quality:docs` pass. This acceptance record is committed with the candidate.                                                                                       |

## Resource and failure record

No model server, model doctor, Marker product or native TTS inference was
started. All new fixtures contain synthetic text, digests and audio metadata.
The checked runtime was Windows x86_64 with Python 3.12.14, Rust/Cargo 1.95.0,
Node 24.14.0, pnpm 11.25.0 and Task 3.50.0. No model runtime identity or
inference timing applies. Observed model-free test durations were 10.14 s for
CONTRACT conformance, 44.11 s for its full suite, 50.48 s for Caster and
11.89 s for the Auralis frontend job suite; these exclude setup and Rust
compilation and are not throughput benchmarks.
A point-in-time process sample during Rust compilation showed individual
`rustc` working sets up to approximately 654 MiB and Python processes up to
approximately 260 MiB; this is not a peak measurement or a model-capacity claim.
Hardware inventory via CIM was access denied and is not used as evidence.

The first Translate checkout `git pull --ff-only` stopped before writing because
the original checkout had overlapping local edits. The Auralis pinned submodule
and a clean fetched Translate worktree were used instead. An initial
`task rs:test:translate` failed because an old Auralis assertion expected
SQLite schema version 5 while the pinned Translate commit is version 6; the
assertion was corrected and the task passed. An initial `task check:docs`
failed with sandbox `spawn EPERM`; the authorized rerun passed. The first
`task check:quality:docs` found formatting only in the new freeze record; it
was formatted. The first `task check:jobs` reached Vitest and failed to spawn
within the sandbox; its authorized rerun passed all Rust stages and 89 frontend
tests. The first final-wheel installed check failed while competing Cargo
builds were active; a direct repeat of its three adapter tests and a clean
full rerun passed. An unpinned wheel rebuild produced a different archive hash;
setting `SOURCE_DATE_EPOCH` made two consecutive builds byte-identical.
Caster's first model-free rerun was blocked by pytest temporary-directory
permissions in the sandbox; the authorized rerun passed 43 tests. One new
voice-projection test initially matched error text instead of the stable code;
it was corrected and conformance passed. A TTS unit run was stopped after its
dummy render began so this goal would not exercise the TTS product; no model
was loaded and the isolated worktree was removed. The original TTS checkout
received independent local edits during this goal; it was left untouched.

Initial `task rs:clippy` attempts exposed three denied warnings in new code;
the explicit lock-file preservation, simplified stop branch and test error
branch were committed, then workspace and native-e2e Clippy passed. Tauri's
build script first needed the pinned media binaries in the isolated worktree.
They were copied read-only from the main Auralis checkout with the identical
media manifest; `task media:verify` passed their hashes and versions. The first
verification attempt was blocked from executing those binaries by the sandbox.

The first contract set proves a model-free producer and validator path only.
Real voice bytes, model profiles, generated audio, listening review, subtitle
fit and final media publication require their separate acceptance goals.
