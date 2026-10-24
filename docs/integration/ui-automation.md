# Ordinary desktop UI automation

The external Node.js runner drives the ordinary release frontend inside Windows
WebView2 using Playwright CDP. It does not enable the native test-runner screen.
Business actions use visible controls; SQLite and file reads only corroborate
published results. The first endpoint is assembled WAV. Video export, linguistic
quality, listening, TXT, ASR and clean installation retain separate acceptance.

## Commands

- `task ui:build` builds and retains an ordinary release executable and resources.
- `task ui:fixture` drives production browser UI with explicit simulated IPC.
- `task ui:smoke` launches the retained Windows candidate without models.
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
install components, change voice qualification or fall back to the ordinary
profile. It closes only its application process and stops subsequent attempts
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
