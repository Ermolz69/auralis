# Goal for complete Auralis module assembly

Updated: 10 October 2026. Submit the objective below for explicit execution.
Preparing this file does not start the Goal. Required integration IDs: INT-A01
through INT-A06, linked to Translate's existing audio acceptance A1 through A6.

## Objective to submit

Complete and audit one real native Windows Auralis workflow composed from the
accepted CONTRACT, Translate, TTS and Caster candidates. An immutable supported
subtitle source must yield an explicitly reviewed Russian result, an explicit
voice assignment, a versioned speech plan, real synchronized speech and a
separate playable media export. Preserve the standalone usability of every
component. Marker integration and the existing Caster WPF application remain
outside this goal.

Read E:/Anything/Projects/Commercial/auralis/docs/integration/goals/common.md,
the integration architecture, boundary contract, component acceptance records,
Auralis storage and job contracts, and Translate RELEASE_ACCEPTANCE.md and
DELIVERY_PLAN.md. Inspect the actual installed candidates and pinned submodule,
wheels, schemas, profiles, models and references. Missing component acceptance
is a prerequisite to finish, not something to replace with mock output.

Freeze one complete licensed or owner-provided representative source and three
distinct 10 to 20 minute listening scenes required by A4. Declare their supported
subtitle subset, selected language release, reviewed translation digest,
explicit roles/voices, qualified engine/reference combinations, target media
format, timing policy, listening coverage, resources and test budgets. Use
versioned owned copies; never write to original subtitle or media files.
Bind speech approval to the exact selected text/cast revisions.

Connect the accepted modules through the existing Rust application and job
runtime. No UI may call a worker directly or read a component database. Keep
project/job state in Auralis, translation detail in Translate and synthesis
checkpoints in the TTS workspace. Reconcile cross-component result publication
by stable identities and outbox-backed managed artifacts. Caster provides data;
the TTS run cannot depend on a live Caster process.

Implement deterministic timeline assembly with measured raw speech durations.
Declare how silence, source windows, overlapping cues, multiple speakers,
permitted pitch-preserving fit and impossible windows are handled. Unsupported
cases produce actionable conflicts before ready export. Default preservation
must not clip speech, silently move source windows, rewrite approved text or
apply unreviewed acceleration. Keep raw synthesis separate from fitted audio.

Use managed FFmpeg for complete audio mixing and a separate video export with
the new audio track. Keep originals and previous ready results. Verify output
streams, durations, cue coverage, absence of unexpected clipping and actual
playback. The final mux and timing functionality must be implemented and tested;
the current mock dubbing pipeline is not acceptable completion evidence.

Generated voice previews, full synthesis and translation all use one shared
machine admission. Switching engines waits for confirmed process exit and
resource release. The UI presents queued work and real persisted progress,
supports cancellation and retry/resume, and reloads authoritative state after
restart. Uncertain completion is reconciled before another attempt begins.

## Required acceptance

The listening gate retains A4's zero unresolved critical meaning or speaker
errors and at least 95% eligible utterances rated at least 4/5. The durability
gate retains A5's complete natural source requirement. Do not replace these
requirements with a short technical demonstration.

| Gate                        | Observable completion                                                                                                                                                           |
| --------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| INT-A01 lineage             | Exact source, reviewed translation, cast, plan, engine, reference and output identities are linked; no missing or extra utterances or silent text changes                       |
| INT-A02 workflow            | Native import, translate, review, assign, synthesize, resolve supported timing and export run against the accepted installed components                                         |
| INT-A03 audio               | Real speech for the frozen representative source passes declared listening and timing criteria; explicit A1 through A6 evidence remains separate from translation G1 through G9 |
| INT-A04 recovery            | Cancellation, application and worker death, restart, duplicate requests, changed inputs and publication gaps preserve accepted work without partial ready artifacts             |
| INT-A05 resources           | No overlapping model ownership; cold/warm/cache measurements, bounded memory working sets, cancellation latency and stage timings are recorded without weakening quality        |
| INT-A06 installed candidate | Clean Windows install, explicit acquisition, offline restart and the complete audio/video workflow pass for one committed, reproducibly pinned release candidate                |

Test a short complete case before the representative longer case. Include an
impossible-fit negative control, a stale review/cast control and a cache/recovery
case. Freeze the finite matrix, retain failures, and add Taskfile commands before
new native, timing, listening and installed-package probes. Run all affected
Auralis gates and component regressions after final candidate changes.

Update host and component documentation with the actual supported behavior and
link scoped acceptance records. Audit candidate pins, ready artifacts, review
evidence and gate claims. Do not mark the assembly complete while mandatory real
listening, translation review, timing or clean-install evidence is missing.
Report the exact final checks and limitations, including deferred Marker, WPF,
automatic casting, other languages and unsupported media/timing modes.
