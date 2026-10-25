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
candidate. It does not inherit native E2E flags, disable the browser sandbox,
change voice qualification or fall back to the ordinary profile. Explicitly pinned
offline wheels may be installed only into the fresh profile before launch.
It closes only its application process and stops subsequent attempts
when process release is unconfirmed. Model and downloader work remains owned by
the application's existing shared admission.

## Real case contract

A version 1 specification pins `case_id`, `candidate_sha256`, `source_url`, a
manual Chinese SRT `track`, `edit_text`, `max_wall_seconds` (60 to 28800), and
`engines` ordered VoxCPM2 then ESpeech. Every engine pins `profile_id`, `voice_id`
and `reference_id`. Use the retained story URL from the source screening record.
Each case ID is consumed once; create a distinct prospective case after changes.
`source_subtitle_sha256` pins the retained Chinese subtitle bytes. Acquired media
and subtitle hashes are checked and recorded before translation, using the managed
artifact root and ready SQLite rows. A changed subtitle is a failed source check.
Inference additionally requires `allow_inference: true` and an exact runtime seed
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

`install_translation: true` drives installation and selection through UI controls.
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
