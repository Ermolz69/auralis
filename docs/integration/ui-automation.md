# Ordinary desktop UI automation

The external Node.js runner drives the ordinary release frontend inside Windows
WebView2 using Playwright CDP. It does not enable the native test-runner screen.
Business actions use visible controls; SQLite and file reads only corroborate
published results. WAV-only cases remain supported. An `export_video` case adds
the [reviewed MP4 policy](revoice-video-export.md) after each engine's WAV.
Linguistic quality, listening, TXT, ASR and clean installation retain separate acceptance.

## Commands

- `task ui:build` builds and retains an ordinary release executable and resources.
- `task ui:assets SEED=new-id` copies installed components and pinned translator
  acquisition files into a new owned seed without executing components or models.
- `task ui:components:build ID=new-id` builds CONTRACT and retains the pinned
  streamed TTS wheel without loading models or changing voice files.
- `task ui:components:probe ID=new-id SEED=seed-id COMPONENTS=component-id`
  validates installed neutral-reference plans without importing model engines.
- `task ui:fixture` drives production browser UI with explicit simulated IPC.
- `task ui:smoke` launches the retained Windows candidate without models.
- `task ui:freeze:connected CASE=new-id SEED=seed-id COMPONENTS=component-id`
  freezes a model-free component smoke specification. Run it with
  `task ui:smoke SPEC=.cache/ui-e2e/specs/new-id.json`.
- `task ui:real SPEC=relative/case.json` consumes one frozen real case once.
- `task ui:freeze CASE=new-case-id` freezes a readiness-only case for the retained candidate.
- `task ui:report RUN=run-id` prints a report and its local HTML location.
- `task ui:check` checks runner policies and profile argument handling.
- `task ui:check:runner` runs JavaScript policy checks without native components.
- `task ui:check:processes` checks cancellation and confirmed process release with
  an inert Node child; reserve the native process window first.
- `task ui:check:pcm SEED=seed-id COMPONENTS=component-id` checks the pinned DSP
  verifier against synthetic multi-rate PCM, silent utterances and same-length
  substitutions, then exercises the real `verifyPcmReadOnly` entrypoint with
  offline wheels installed into a fresh profile. It imports no engines or models. `INPUT_ROOT` optionally
  locates retained inputs in another checkout; all output belongs to a fresh run.

Runs are retained separately under `.cache/ui-e2e/runs`. Each includes JSON,
JUnit and HTML reports, step screenshots, diagnostics and a Playwright trace.
Fixture results do not establish native or model acceptance. Blocked and unrun
required work return nonzero status; failed attempts are retained.

## Isolation and ownership

`--profile-dir <absolute-directory>` selects a separate application root and
WebView2 data directory. Missing, duplicate, relative and parent-traversing
arguments are rejected before bootstrap. Default startup keeps its existing
profile. The legacy native E2E build rejects the new override. Machine-wide
component admission is not relocated into the test profile.

The runner uses one session lock, a fresh profile and a hash-verified retained
candidate. Version 3 candidate receipts pin frontend, Rust, platform, capability,
task, media-tool and runner sources, locks and the initialized clean Translate
dependency. Candidate reuse compares the complete input manifest. Inference
also requires the current runner bytes to match that receipt. It does not inherit native E2E flags, disable the browser sandbox,
change voice qualification or fall back to the ordinary profile. Explicitly pinned
offline wheels may be installed only into the fresh profile before launch.
It tracks application and installer descendants by PID and creation time against
a prelaunch inventory. It closes only confirmed owned processes and stops subsequent attempts
when process release or the ancestry of a new process is unconfirmed. The session
lock and admission lease remain retained on uncertainty. Model and downloader work remains owned by
the application's existing shared admission.

## Real case contract

A version 1 specification pins `case_id`, `candidate_sha256`, `source_url`, a
manual Chinese SRT `track`, `edit_text`, `max_wall_seconds` (60 to 604800), and
`engines` ordered VoxCPM2 then ESpeech. Every engine pins `profile_id`, `voice_id`
and `reference_id`. Use the retained story URL from the source screening record.
Each case ID is consumed once; create a distinct prospective case after changes.
`source_subtitle_sha256` pins the retained Chinese subtitle bytes. Acquired media
and subtitle hashes are checked and recorded before translation, using the managed
artifact root and ready SQLite rows. A changed subtitle is a failed source check.
Inference additionally requires `allow_inference: true`, `case_scope`, the complete
candidate receipt hash, resource limits and an exact runtime seed
manifest. A generated readiness case explicitly forbids inference and uses
unconfigured engine selections; it never authorizes model work.

An optional `runtime_seed` identifies an owned directory below
`.cache/ui-e2e/seeds` and an exact `files` list containing `path`, `sha256` and
`size_bytes`. Only verified listed files are copied; runtime configuration paths
are remapped into the fresh profile. Symlinks, redirected files, outside paths,
duplicates and oversized manifests are refused. Voice manifests retain their
original bytes and qualification. Without an installed seed the real scenario
reports readiness blockers instead of downloading or fabricating components.
Only component configuration files and the `tts`, `translation-packages`,
`runtime`, `models`, `voices`, `profiles`, `assets`, `model-cache`, `caster` and `marker` trees are
accepted. Application databases, projects, job records and browser state cannot
be seeded. The first launch verifies zero projects and jobs. Configuration-file
presence is diagnostic evidence and does not establish runtime readiness.
The dedicated `cache/translation-downloads` seed directory admits only cached
acquisition files; installation and selection still use the ordinary UI.
Absolute native dependency paths and Python home are relocated inside each
profile, with a new verified inventory digest. Optional `component_wheels` pin
the two offline component wheels installed into a new profile-owned package
directory before launch. Source seeds and voice bytes remain unchanged.

`task ui:freeze:connected CASE=new-id SEED=seed-id COMPONENTS=component-id MODE=real
SCOPE=full-source` generates a prospective full-source draft. `SCOPE=diagnostic`
generates a bounded single-utterance draft. Both retain `allow_inference: false`;
freezing inputs does not authorize model execution. Run real cases only in the
designated real-test window. Each engine identity additionally pins profile bytes,
model revision, voice manifest and reference audio hashes. Applied profile
identities are checked before launch.

Full-source requires all 4004 original cues in their original order and timing,
Chinese SRT SHA-256 and timing digest, translator installation and selection,
the edited approved translation, and WAV/MP4 for both sequential engines. Its
default wall budget is 100 hours and host TTS budget is 172800 seconds per engine.
Diagnostic retains source acquisition/identity checks but voices one explicitly
reviewed Russian text per engine; translation and MP4 are recorded `not_run`.
Its default wall budget is five hours and TTS budget is 2700 seconds. The frozen
`tts_execution_budget_seconds` is written only to the copied `tts-runtime.json` as
`execution_budget_seconds`; the resulting configuration hash and value are
checked before every launch and reopen. The runtime must support this optional
finite budget. Legacy runtime configurations retain their existing timeout.

`install_translation: true` drives installation and selection through UI controls.
The generated full-source seed list excludes translator runtime and selection
configuration, so seeded selection cannot substitute for this UI stage. Cached
acquisition bytes may remain available. Failure to exercise installation or
selection is a blocker, with later stages left unrun.
`allow_unverified_neutral: true` checks an explicit technical-permission control.
The default remains strict: rejected and unverified emotional references cannot
use this exception. The permission is bound to the saved plan and shown again
after reopening; it neither qualifies voices nor establishes listening quality.
`export_video: true` requires ready videos for both engines and playback after
reopening. Fixture video is synthetic WebM/VP8; its UI playback does not test
the native H.264/AAC encoder.

The real workflow deliberately records its review as an automated workflow test.
It cannot establish linguistic correctness. System file dialogs are outside the
first URL-based scenario. Native cancellation, restart and playback assertions
remain distinct from simulated fixture assertions.

## State, evidence and failure semantics

Source import, translation, speech and assembly expose accessible status names.
The runner observes the specific job state and attempt identity. A timeout from
a click triggers observation of that action's result; the click is never repeated.
Resume waits for a new job ID before accepting its state. Errors win over ready
controls, and known infrastructure absence is distinguished from integrity or
contract failure. Unknown errors fail. A failed verifier cannot advance to
reopen playback. Missing stages cannot produce `passed`.

The fixture starts with English selected and explicitly chooses manual Chinese
SRT. It checks exact-version approval invalidation, duplicate starts, delayed
cancel/resume, error recovery, stale timing previews, timing conflict, damaged
WAV/video refusal and recovery after reload. It retains two distinct speech runs
and reopens each recorded WAV and MP4 assembly, including both engine outputs.
A separate diagnostic fixture checks the single-text route and explicitly leaves
translation/video unrun. These are simulated IPC tests; fixture WAV/video cannot
establish real signal, native codec, cancellation or interrupted-job recovery.

Real artifact checks read ready SQLite publications and managed files only.
Full-source approval is tied to the visible translation/result IDs, selected
published SRT, Chinese source identity, latest approval event and exact edited
cue text. Both engines must use the same approval. Ordered technical chunks cover
every approved utterance without lost text or timing; their published bytes,
layout, raw WAV and assembly ranges are cross-checked. Each nonblank utterance
requires a nonzero PCM signal.

For inference, a separate supervised read-only verifier additionally reproduces
the pinned resampling, edge fade, global peak gain and PCM16 quantization in
memory, comparing every byte with the published aggregate. Its helper and
numerical-library hashes are verified before import. It uses isolated Python
without site initialization, imports no engine/model modules, calls no TTS or
`join_audio`, and writes no replacement WAV. Its JSON receipt records helper,
library versions, policy and matching PCM digests. MP4 publication must retain the
complete WAV/manifest/MP4/video-manifest set; playback requires progress and
decoded video dimensions before and after ordinary restart.
Playback uses an explicit accessible play button for the loaded media. It is
idempotent when autoplay has already started, avoiding a play/pause keyboard race.

Resource checks sample the sum of confirmed owned-process RSS, GPU memory and
free disk every second, with a separate bounded profile-size audit every 30
seconds and after seeding/cleanup. Default limits are 24 GiB RSS, 7680 MiB GPU,
2048 MiB baseline GPU, 64 GiB profile and at least 64 GiB free disk. Reports retain
sampled high-water marks and maximum sampling gaps; these are not continuous
peak measurements. Exceeded limits or observer failure abort the pending UI
observation, request visible cancellation and retain diagnostics even if logging
fails. Hard wall deadlines also cover copying and supervised component work.

Native cancellation, corruption injection and restart during an active job are
recorded separately as `not_run` by the regular real runner. The default real
path starts each engine once; it does not spend an additional model attempt to
reproduce fixture fault cases. Listening, translation quality and other formats
retain separate acceptance. A new integrated ordinary candidate and model-free
native smoke are required after changing the backend or runner.

## Model-free runner verification: 2026-10-26

`task ui:fixture` passed 32 full-workflow steps and 18 separate diagnostic steps
after adding explicit playback controls. An earlier playback-race failure and a
DSP harness packaging failure remain failed in their original reports; subsequent
checks used fresh run IDs and profiles.

The synthetic DSP regression passed its positive multi-rate, multi-block PCM
case and rejected same-length nonzero substitution, one silent nonblank
utterance, changed helper/library hashes and unknown policy. It verified 289614
frames, two utterances and three chunks against the retained streaming helper,
without engine imports, `join_audio` or models. This verifies the checker with
synthetic media; it does not establish real synthesized speech acceptance.

The installed-config wrapper fixture also passed on runner commit `29b30b6`,
using the same `verifyPcmReadOnly` entrypoint as a real case. It resolves the
single installed `src/tts/audio/joiner.py` inventory entry, verifies its exact
hash and pins numerical dependencies including typing extensions and the CFFI
parser. The oracle and wrapper produced the same PCM digest. Its three supervised
Python process trees, admission guardian and session lock were confirmed released.
Earlier wrapper failures remain failed; they do not establish wrapper acceptance.

`task ui:check:runner` passed 33 policy tests. `task ui:check:frontend` passed
type checking, 12 unit and 29 component tests; `task docs:integration`,
`task fe:lint` and `task q:frontend` passed. Broader checks passed 235 unit and
32 component regressions and the existing 35 browser tests. An inert Node-child
cancellation probe confirmed owned process release. No real translation, TTS or
MP4 model run was performed for these checks. Native smoke for the newly
integrated candidate remains a separate release verification.

## Verification checkpoint: 2026-10-24

The ordinary Windows candidate was built from Auralis commit
`0b9f65c2598e574bc938b615a6845f0eff7de770`, with Translate pinned to
`44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`. Its executable SHA-256 is
`85aac8da263482866811d5cb3e9445cf4c67268b5124c2a369a6797c65d58285`.
The driver used Node.js 24.14.0 and Playwright 1.61.1.

| Layer                               | Result               | Scope                                                                                                                 |
| ----------------------------------- | -------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Production browser with fixture IPC | 23 steps passed      | Both engine workflows, editing, stale approval, duplicate start, cancellation, timing conflicts and playback recovery |
| Ordinary Windows release            | 9 steps passed       | Fresh profile, WebView2 connection, input checks, visible source refusal, navigation and confirmed process release    |
| Frozen real source case             | Blocked, exit code 2 | `UI-REAL-SOURCE-20261024-v1`; downstream work remained `not_run`                                                      |

The real case observed `Operation not supported` during source creation. Its
isolated profile had no TTS or translation runtime configuration. Source creation
requires a CONTRACT snapshot before URL validation or metadata acquisition, so
this attempt does not establish YouTube availability. Prepare a verified
component seed and a new frozen case ID before the next real attempt. The spent
case forbids inference and must not be rerun as a model acceptance test.

Additional checks passed: `task ui:check` (profile handling and runner policies),
`task ui:regression` (234 unit and 31 component tests), `task fe:e2e` (35 existing
browser tests), `task ui:lint` and `task docs:integration`. No real translation,
TTS, assembled speech or video acceptance follows from this checkpoint.

## Connected-component preparation: 2026-10-25

The next candidate adds explicit neutral-reference technical permission, optional
reviewed MP4 publication, profile-relative runtime relocation and pinned offline
component installation. Strict qualification remains the default; the permission
neither changes reference manifests nor accepts speech quality.

`task ui:fixture` passed 27 steps with simulated IPC, including translator setup,
both engine workflows, persisted permission, four-file publication, corrupt-video
refusal and synthetic video playback. `task ui:components:probe` validated four
neutral-reference plans against the installed CONTRACT wheel built from
`3f3498e87b4cfb68c67d0d718497c6286330f6b8`, with streamed TTS pinned to
`e18f64d1bad80012b085d907673451c9e5b886f7`. No engine module or model was loaded.

The connected smoke case submits an invalid non-YouTube host through ordinary UI.
Source creation verifies the real CONTRACT receipt before rejecting the host;
the report requires the corresponding verified-boundary diagnostic. URL refusal
alone cannot establish that installed components are connected. This case forbids
inference and creates no project or job. Actual source acquisition, translation,
TTS, native MP4 encoding and real-media playback require a distinct real case.

The ordinary connected Windows smoke passed all ten steps for
`UI-COMPONENT-SMOKE-20261025-v1`. Its candidate was built from
`f1d31f3cfe2ca583171ab3e9ea40866ca6ff5e75` with executable SHA-256
`4e893efa00597dad1a11cd25b1438a2e03a329e2d4216e7d9c46ade9e9812ebd`.
The application verified the installed CONTRACT boundary, refused the invalid
host, retained an empty isolated profile and closed without forced termination.
No inference or real-media acceptance was performed.

The real artifact verifier resolves engine/profile from the plan and voice from
its cast, matching the persisted CONTRACT schema. It compares cue order, text and
timing with the saved approval snapshot, requires the matching translation
approval, and verifies both immutable plan hashes. `task ui:check:runner` passed
13 checks, including changed source, timing, cast and selection refusal.
`task ui:lint`, `task ui:check:video`, `task ui:check:frontend`,
`task ui:regression`, `task fe:e2e` and `task docs:integration` passed without
model inference. The existing browser suite retained 35 passing tests.
