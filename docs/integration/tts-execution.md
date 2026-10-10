# TTS integration execution record

Started: 10 October 2026. Scope: native Windows, Python 3.12, explicit reviewed
Russian text or ordered subtitle utterances, VoxCPM2 and ESpeech, neutral
references first. Additional emotions require an exact qualified v2 reference
and separate evidence. Source cue windows are lineage, not automatic duration fit.

## Task and gate map

| ID      | Deliverable                                                          | Required evidence                                                                           | Current status                                                                                                                                |
| ------- | -------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------- |
| INT-T01 | Reusable installed library, legacy CLI and worker on one render core | Installed wheel, baseline comparison, exact profile/model pins                              | Candidate TTS commit `99f2979`; installed import smoke passed; baseline comparison open                                                       |
| INT-T02 | Canonical plan, strict references and complete lineage               | CONTRACT fixtures, malformed plan/profile/reference/split rejection, manifest coverage      | Model-free CONTRACT checks and independent Rust result verifier pass; real plan candidate open                                                |
| INT-T03 | Owned sequential execution and recovery                              | Admission, process exit, durable progress, cancel, crash, exact resume, stale attempt tests | Bounded worker transport and durable speech attempt/checkpoint fencing pass model-free checks; host admission and native lifecycle tests open |
| INT-T04 | Verified cache reuse                                                 | Changed input identity and corruption matrix with unaffected cache entries retained         | Audio/metadata corruption covered; full matrix open                                                                                           |
| INT-T05 | Real audio from both engines                                         | Short and longer cases, hashes, measured durations, actual listening                        | Four real legacy CLI probes passed technical verification; speech-plan cases and listening open                                               |
| INT-T06 | Auralis desktop workflow                                             | React, Tauri, Rust, worker, managed publication, restart, cancel, resume and playback       | Rust worker transport compiles; host and desktop flow open                                                                                    |
| INT-T07 | Installed offline Windows delivery                                   | Exact wheels/runtime/assets, interrupted acquisition recovery and clean install             | Local wheel import smoke only; clean install open                                                                                             |

The current new worker operations are `capabilities`, `catalog`, `compose`,
`validate`, `execute` and `cancel`. Worker frames are limited to 1 MiB and
speech-plan files to 16 MiB. The cooperative cancellation deadline is 15
seconds; the host must enforce process-tree exit and hold machine admission
until termination. The host will stage one attempt under a project-owned root
and publish only after verifying the full manifest and final WAV. The Rust
adapter now independently rechecks plan and profile hashes, utterance and chunk
order, exact text concatenation, cue and role lineage, reference IDs, segment
audio hashes, model revision, final output hashes, and PCM16 WAV measurements.
The host now has a version 11 SQLite run-intent, attempt and checkpoint
migration. Run intents preserve exact plan bytes and their digest, profile
identity and expected counts. A host attempt and its job are admitted in one
transaction, and saved checkpoint progress is written with its attempt ID and
job revision in one transaction. Closed and superseded attempts reject late
events. Completion deliberately remains unavailable in this store until the
managed artifact publication transaction is implemented. The host publication
path has not been implemented yet. Semantic,
identity and quality failures have zero automatic retries. A transient
transport or storage failure permits at most two retries after reconciliation
of the prior attempt; each retry gets a fresh attempt ID. No model job may
overlap any other model server, doctor, loader or render on this machine.

The candidate source profiles are `native-plan-voxcpm2` and
`native-plan-espeech`. The former pins `openbmb/VoxCPM2` revision
`32279effe8c19989596f05d353d1447f51d9e915`, six inference steps,
low-memory loading and no denoiser. The latter pins
`ESpeech/ESpeech-TTS-1_RL-V2` revision
`f582b6e5897fe8a5835059405a8439d13bdf7684`, vocoder
`charactr/vocos-mel-24khz` revision
`0feb3fdd929bcd6649e0e7c5a688cf7dd012ef21`, and 64 NFE steps.
These settings are candidates until real tests and listening pass. The
standalone CLI remains the unchanged behavioral baseline.

## Real-audio experiment gate

Before inference, register four immutable case IDs: one short continuous case
and one longer ordered subtitle case for each engine. The short case must have
at most 250 reviewed characters. The longer case must have 4–8 utterances,
include a technical split, use at least two distinct compatible qualified
references and contain at most 1,500 reviewed characters. Record every
utterance's exact text, cue ID/window where applicable, source hash, reference
IDs and byte hashes, profile/model hashes, runtime identity and seed. Run one
declared attempt per case; the time budget is 30 minutes per VoxCPM2 case and
45 minutes per ESpeech case, with an 8 GiB physical GPU and at most 7 GiB
additional VRAM use plus a 24 GiB RAM ceiling. The local GPU inspection on
10 October reported an RTX 3070 with 8,192 MiB total and 898 MiB used before
model work. Stop
at a competing model process, missing asset, invalid qualification, memory or
time ceiling, incomplete WAV, or changed candidate identity. Retain failures.

A human listener must review every generated short and longer utterance for
intelligibility, pronunciation, clipped or repeated speech, audible artifacts,
reference leakage and voice consistency. The currently inspected CASTER store
has legacy entries and an `unverified` v2 reference; no production-qualified
reference for both engines is available. Therefore the four case registrations
and model runs remain pending. Synthetic qualified fixtures are restricted to
model-free contract tests and cannot satisfy this gate.

The user chose Ciri and Geralt for both engines. TTS `task voices:candidates`
created separate v2 evaluation copies without changing CASTER. The neutral
source references are mono 48 kHz PCM WAVs of 8.516 s and 15.359 s. Both are
still `unverified`. Four legacy CLI qualification probes were predeclared as
inputs for a later human decision; they do not replace the four required
speech-plan integration cases. The first two Ciri/VoxCPM2 probe identities
retained pre-model failures from a mixed voice store and then a transient GUI
process classification. The corrected registry has SHA-256
`57bef4a86ff6b7af372e20d6fc1d92f8eade8866623d2b367584177c97ae6024`.
All four corrected probes then completed on native Windows with real VoxCPM2
and ESpeech models. `task voices:inspect-probe -- CASE_ID` verified their frozen
inputs, model revisions, seeds, output WAVs, manifests and SHA-256 hashes. Their
durations were 3,840/3,200 ms for VoxCPM2 Ciri/Geralt and 6,432/2,880 ms for
ESpeech Ciri/Geralt. Peak additional VRAM was at most 6,051 MiB, below the
declared 7 GiB limit. `task voices:prepare-review` produced four CONTRACT
`awaiting_review` reports. The user has been asked to listen and score each
recording; no v2 reference has been qualified yet. These legacy CLI probes do
not satisfy the required speech-plan short and longer cases.

## Observed checks

- CONTRACT `task check:python -- -x --tb=short`: 375 passed, 1 skipped.
- CONTRACT `task check:compile`: passed.
- TTS `task test:unit`: 83 passed after the worker catalog/compose and model-pin changes.
- TTS `task compose:config`: passed; no image build.
- TTS `task package:wheel`: built CONTRACT 0.2.0 and TTS 0.3.0 wheels.
- TTS `task package:smoke`: installed both wheels into disposable staging and
  confirmed imports, a framed worker subprocess and bundled profile pins.
- TTS `task runtime:check`: the existing native GPU environment reported no
  broken installed requirements; this is not a clean-install test.
- TTS `task runtime:assets`: both pinned model revisions and the ESpeech
  vocoder were found in the local cache offline, without model loading.
- TTS `task voices:prepare-review`: four native real-audio qualification probes
  and awaiting-review reports verified; human scores pending.
- Auralis `task rs:fmt:tts`: passed for the isolated adapter sources.
- Auralis `task rs:lock:refresh`: passed offline after adding adapter dependencies.
- Auralis `task rs:test:tts`: protocol, framing, lineage tampering and WAV
  verification tests, speech job progress, SQLite attempt fencing and version
  10 migration tests passed on
  native Windows; the execution path compiled but has not run a model or native
  lifecycle test.
- Auralis `task rs:check`: passed for the workspace after a sandbox access denial
  in Tauri's bundled ffmpeg build step was resolved by an unsandboxed run.
- Auralis `task fe:typecheck`: passed after adding the generated speech artifact
  wire kind.

These are engineering checks. Auralis Rust, frontend, native, real-audio and
clean-install checks must be added here with exact observed outcomes before
their gates can be closed.
