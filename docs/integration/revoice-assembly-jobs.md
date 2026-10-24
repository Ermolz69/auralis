# Durable managed audio assembly workflow

Prospective INT-R08/INT-R09 contract, 23 October 2026. The endpoint is an actual
application workflow: select a ready raw speech run, inspect its complete
utterances, explicitly review continuous/source-window timing, start/cancel/
resume a separate assembly job and play a distinct outbox-published WAV. Keep
raw speech and previous ready assemblies. Do not accept this endpoint from a
reader, schema, mock executor or staging-only check.

## Ready source inspection

Resolve the raw WAV, raw manifest and optional `audio_layout` through the ready
speech publication, project-owned `ArtifactIndex` records and `ArtifactStore`.
Require one of each, exact stored sizes/hashes and the immutable saved speech
plan hash/order. Missing layout yields an explicit streaming-package/new-render
action; do not reinterpret old assembly policies.

A bounded adapter reads and hashes layout/manifest JSON (16 MiB each), verifies
their identities and complete ordered chunk coverage, hashes and parses PCM16
WAV (48 kHz, mono/stereo), and validates every range. Treat the layout's old
attempt path as provenance. Rebind it only to the resolved, verified managed
aggregate. Do not open cached chunk paths or require the old installed TTS/model/
voice runtime for assembly of an already accepted immutable result.

Calculate the exact future per-utterance WAV hash from its canonical 44-byte WAV
header plus the complete PCM range, with 64 KiB I/O and no intermediate audio
files. Use those genuine whole-utterance measurements in the domain timeline.
Return typed fit/overlap/coverage conflicts and durations. No synthetic hash may
stand in for an utterance measurement. Input inspection has a finite 120-second
ceiling, cancellation between blocks, and cancellation on caller drop. It loads
no model and runs no external process. Actual assembly retains shared machine
admission and process ownership.

## Immutable intent and job

Persist only managed artifact IDs, their hashes/sizes, source run/attempt,
project, saved plan hash, exact timing settings and measured timeline request;
do not store an executable filesystem path in an assembly intent. The preview
digest covers these exact values, excluding a newly allocated assembly UUID.
Changing a source, pause, duration or mode invalidates the preview and requires
another explicit review. The UI never changes approved text or cast implicitly.

Keep a separate `audioAssembly` common job with persisted intent/attempt and
publication rows. Register identical project/source/settings intents
idempotently. Fence every attempt by UUID and job revision, with at most one
active attempt. Start/resume re-inspects the same ready managed source and
compares exact immutable facts before process work. The initial lossless policy
uses tempo 1.0; requests requiring fitting remain actionable conflicts until a
separately reviewed permitted-tempo policy is implemented.

Use the common runtime reservation, cancellation and lifecycle events. Progress
reflects persisted stages (source verification, assembly, output verification,
publication), staying below 100 until every output is ready. Do not mutate the
project's unrelated dubbing pipeline state for this job kind. Uncertain child
cleanup retains admission and prevents another process attempt.

## Publication and recovery

Stage a distinct assembled WAV and assembly manifest, verify their hashes, then
commit job/publication/artifact/outbox changes in one short transaction gateway.
Finalize through the existing outbox and expose neither file until the complete
publication becomes ready. A schema migration adds assembly intents, attempts,
publications, normalized output links and discarded-artifact tombstones without
rebuilding source tables.
Keep readiness queries and late-finalization guards aware of the new publication.

Cancellation before commit leaves no ready artifact. Cancellation during pending
publication tombstones all its files and suppresses late finalization. Reconcile
interrupted attempts on restart; a committed valid pending publication can finish
without rerunning FFmpeg, while an incomplete/unaccepted attempt requires explicit
resume. Resume reuses verified raw speech and never resynthesizes it. Duplicate
commands cannot create a competing attempt or overwrite an earlier ready output.

## Commands and UI

Thin commands expose inspect, prepare/start/resume, state and project history.
An explicit playback verification command checks published hashes with a bounded
deadline. Routine state/history polling reads metadata and does not repeatedly
hash long WAV files. The assembly panel remains accessible when the TTS catalog
or old runtime is unavailable.
Compose the managed source reader, hash-pinned FFmpeg adapter, transaction gateway
and common runtime at the Tauri root. Show the exact source run and timing review,
full utterance coverage, explicit pauses, original windows, required/available
duration conflicts, persisted job state and distinct raw/assembled playback.
Derive media duration from a verified media probe when media is available; an
explicit standalone audio timeline duration must be labelled as such and cannot
prove a later video fit. Video mux must verify its actual source duration and
refuse a mismatch rather than move or trim speech.

## Finite verification

Add Taskfile commands first. Model-free source controls cover a retained attempt
path that no longer exists, exact PCM/header hashes, mixed-rate chunk-derived
boundaries, 600 utterances, corrupt metadata/audio, wrong source plan/order,
unknown policy, bounded JSON/bytes/deadline, cancellation and actionable timing
conflicts. No model, real FFmpeg or native UI is invoked by those controls.

Then test schema 18 upgrade/data preservation, idempotent intent registration,
cross-project/changed-source refusal, competing/stale attempts, cancellation and
late finalize, pending publication recovery, caller/worker death and actual
managed artifact visibility through application use cases with fixture process
work. Add IPC/UI review, stale preview, cancel/resume and playback controls. Run
affected storage/application/jobs, frontend, desktop composition, lint, docs/plan
and size gates. Freeze a new committed real-FFmpeg/native matrix before invoking
real tools; earlier fixture budgets are spent and cannot be reused.

Real installed worker/native playback, full natural stories, measured resources,
listening, video, TXT, ASR and clean Windows remain mandatory acceptance work.

## Current implementation checkpoint

The candidate connects ready managed inspection, schema 19, immutable review,
common-runtime attempts, verified cancellable copies, outbox group visibility,
discarded-output tombstones, startup reconciliation and desktop commands. The
panel offers continuous or original-window timing, paged measurements/pauses,
visible conflicts, saved plans, cancellation/resume and distinct assembled WAV
playback. A missing managed FFmpeg or legacy raw layout yields a repair action.
The Windows x64 tool is hash-pinned; PATH fallbacks are not accepted for assembly.

The model-free receipt
`tools/revoice/evidence/AUDIO_ASSEMBLY_JOBS_V2.json` records exact source hashes
and finite check outcomes, preserving the earlier checkpoint. Controls cover 600
utterances, vanished cache/worker paths, genuine whole-utterance WAV hashes,
queued cancellation, source/output corruption, schema upgrades, idempotency,
competing/stale attempts, partial/late publication, actual outbox visibility,
actor abort/panic and explicit resume. The application renderer in these controls
is a fixture. Raw files and the project's unrelated pipeline state are preserved.

No real FFmpeg/model/native experiment has run for this candidate. This workflow
has not installed the candidate or updated voice qualification data. Real installed assembly
and playback, declared-duration verification against actual media, separately
reviewed permitted tempo, video export, natural stories, TXT, ASR, listening and
clean Windows remain mandatory. No endpoint is accepted from these controls.
