# Managed PCM assembly slice, 23 October 2026

This prospective INT-R08 implementation connects the pure timeline to an owned,
hash-pinned FFmpeg process. It produces verified staging files only. It does not
mark managed artifacts ready, replace the mock pipeline, or accept native,
natural-story, listening or clean-install gates.

## Frozen rendering policy

Input is a verified complete untimed utterance WAV per timeline item: 48 kHz,
PCM16 little endian, one or two channels matching the whole plan. The worker's
technical-chunk bridge, resampling, optional tempo changes and final video mux
remain separate required integration work. Reject other input properties here;
never infer missing chunks or silently convert a waveform in this stage.

The adapter rechecks the domain plan and exact ordered input IDs. Copy each input
under a unique owned staging directory using bounded blocks, compare its whole
file hash and measured WAV properties with the immutable facts, then use only
these verified copies. Source files and previous outputs are never overwritten.
Use generated relative ASCII filenames and an ffconcat list with safe paths;
input labels containing spaces, quotes or Unicode never become commands.

Generate exact zero PCM for declared silence. Managed FFmpeg performs concat
streamcopy to a new WAV, at tempo 1.0 with no gain, fades, mixing or resampling.
This policy is `pcm16_48k_exact_concat_v1`, distinct from raw TTS assembly. Final
audio level preparation and explicit fit policy still need their own controls.
The concat requirement for matching streams and streamcopy semantics follow
[FFmpeg's concat documentation](https://ffmpeg.org/ffmpeg-formats.html#concat) and
[streamcopy documentation](https://ffmpeg.org/ffmpeg.html#Streamcopy).

Before accepting staging, parse the complete RIFF layout, verify channels/rate/
frame count and compare the output's entire PCM-data SHA-256 with the ordered
verified input PCM plus exact silence. Measure peak and full-scale sample count;
require them to equal the source expectations. This proves no added clipping,
missing/extra/reordered samples or clipped tail for this lossless policy. It does
not approve an already clipped source or establish listening quality.

Acquire shared model-free machine admission before process work. Use the existing
Windows Job Object owner, no shell, no stdin, bounded stderr, finite process and
total deadlines and cooperative cancellation during copying/verification. Wait
for owned process exit and confirmed admission release before returning staged
success. Cancellation, timeout, excessive stderr and process/verification failure
never produce a successful staged result. A hard application death can leave an
unaccepted temporary directory; recovery must reconcile it before publication.

Default ceilings: 8,192 utterances, 16,385 timeline spans, 4 GiB minus RIFF header
headroom for output PCM and 4 GiB total source-file bytes; 64 KiB I/O blocks,
64 KiB stderr, ten-minute process and thirty-minute total deadline. User-facing
configuration/jobs and disk-budget admission remain future integration work.

## Finite verification before real media invocation

Add Taskfile commands first. Model-free controls cover shared WAV parsing,
non-destructive domain-module splitting, ordered/hash/property validation,
cancellation before work, malformed RIFF, path ownership, generated safe concat
entries and controlled child-process cancellation/timeout/stderr limits.

Then commit the candidate and freeze synthetic fixture hashes before one admitted
real FFmpeg matrix. Use the existing bundled FFmpeg 8.1.2, SHA-256
`4044b3924c977ad31229d504c5d5b8685f9553124fbaff6e9c99048b42830341`.
Retain separate continuous, source-window and >512-item cases; each permits one
attempt, at most 180 seconds and 64 MiB generated data. Sources are synthetic
diagnostics, never voice qualification or real-story quality evidence. Do not
retry failures under the same case ID or change thresholds after seeing output.

## Implementation and model-free checks

The adapter and typed staging port are implemented. WAV inspection is shared
with the TTS verifier through `common::wav`, preserving complete RIFF/frame checks
and ties-to-even duration rounding. Invalid channel alignment no longer uses
saturating multiplication, and metadata is bounded to 4,096 RIFF chunks. The
timeline's entities, request, spans, measurements and conflicts now follow the
project's module-splitting rule with a small export-only `mod.rs`.

`task rs:test:audio:assembly` passed its shared-WAV, initial assembly, timeline,
TTS, storage and speech-workflow regressions. After adding two further assembly
controls, `task rs:test:audio:assembly:core` passed 7 shared-WAV, 9 assembly,
10 timeline and 21 domain-library tests. Owned fixture children verify cancellation
after observed start, timeout, bounded stderr and nonzero exit; no real FFmpeg
or model is invoked by these controls. Inputs containing Unicode/quotes retain
their original bytes and use only generated safe concat names.

Scoped production/test Clippy passed with warnings denied, the frozen real-media
driver compiled without running, normal desktop composition passed, and the
architectural file-size gate passed. `Cargo.lock` changed only three dependency
edges for existing workspace packages; no package version was updated. The first
compile failure after moving timeline types was an omitted explicit import and
was corrected; its log remains separate from successful checks.

The [first real managed-FFmpeg matrix](acceptance/09-revoice-managed-ffmpeg.md)
then passed all three frozen cases once, including stereo, source windows and
600 ordered entries. Exact PCM hashes, frames, peaks and clipping counts matched.
Verified WAVs, manifests, sources and receipts are retained outside Cargo target.
This does not accept the natural-story, native, publication or listening gates.

Publication still requires application
ownership/review checks, complete chunk provenance, durable assembly intents and
jobs, outbox staging/finalization, replay/recovery, native controls, actual model
speech, final video export and playback/listening verification.

The [verified technical-chunk bridge](revoice-chunk-bridge.md) fixes the next
integration slice's frame extraction policy and finite model-free matrix.
