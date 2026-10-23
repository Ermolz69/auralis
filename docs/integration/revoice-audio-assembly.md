# Audio assembly policy, 23 October 2026

This prospective INT-R08 slice implements the pure deterministic timeline before
connecting managed FFmpeg, jobs, storage and native controls. It does not accept
FIT-01/02/03, audio/video playback or listening gates by itself.

## Timeline v1

Require exact ordered utterance coverage, unique IDs, raw audio hashes and
positive measured PCM frame counts/rates/channels. Bind the result to the exact
source speech-plan and raw-result manifest digests. Work only on bounded metadata
(at most 8,192 utterances); never open or decode audio in the domain layer.

Each fact describes one complete untimed utterance audio artifact. The current
worker can split an utterance into several technical chunks; the adapter must
verify their exact lineage, approved text coverage and order, then combine every
chunk and measure the complete artifact before calling this planner. A single
chunk cannot stand in for the utterance. That adapter bridge is not implemented
by this domain-only slice.

Output is 48 kHz PCM16 with one or two channels matching every raw fragment.
Raw rates are 8, 11.025, 12, 16, 22.05, 24, 32, 44.1, 48, 88.2, 96, 176.4 and
192 kHz. Reject channel mismatch and other raw sample rates. Resampled frame counts
are the integer ceiling of `raw_frames * 48000 / raw_rate`, preserving every
source frame. All arithmetic is checked; reject PCM16 RIFF size overflow.
The adapter must subsequently measure the rendered output rather than infer its
actual frame count from this plan alone.

Continuous mode places full speech in declared order, inserting only explicitly
declared pauses after utterances. It is an independent narration WAV and does
not imply alignment with source video. No punctuation-derived pause is inserted.

Subtitle-window mode requires a known positive source duration, both cue times,
strictly positive windows within that duration and no overlapping source windows,
even with different speakers. Source milliseconds map exactly to 48 frames per
millisecond. Place each full raw utterance at its original window start and add
silence to the next start and the media end. Reject any measured full utterance
that exceeds its source window. Declared continuous pauses are invalid in this
mode. Never move a window, truncate a tail, rewrite approved text or combine
overlapping speakers silently.

V1 policy is `preserve_full_speech_48k_v1`, at tempo 1.0 only. Optional explicit
pitch-preserving acceleration up to 1.15x remains separate future work requiring
a versioned policy, native selection, actual FFmpeg measurement and listening.
The planner exposes actionable typed conflicts; a conflicting timeline cannot
become a ready timed export. Continuous audio remains independently available.

## Frozen model-free matrix

Before the first timing probe, add Taskfile commands and cover:

1. Exact speech order, hashes, complete frame coverage and explicit pauses in
   continuous mode, including more than 512 utterances.
2. Leading, interior and trailing silence in source-window mode; exact fit and
   adjacent windows are accepted without shifting or clipping.
3. An impossible fit reports both required and available frames with the
   utterance ID. Overlap, reversed/empty/missing windows, out-of-media windows
   and nonzero subtitle pauses produce typed conflicts and no timeline.
4. Missing, extra, reordered or duplicate utterances; zero/unsupported rates,
   empty audio, channel mismatch, arithmetic overflow and RIFF overflow fail.
5. Planning the same immutable facts twice yields the same timeline; inputs and
   raw cache identities are unchanged.

## Observed implementation checks

The pure domain planner and typed conflicts are implemented. The frozen ten-case
timing suite passed, including 600 ordered utterances and the exact 8,192-item
ceiling. All 21 domain library unit tests passed. Scoped Clippy passed with
warnings denied. No media process, model, database, native command or original
audio file was involved; identical input facts produced identical timelines.
Normal desktop composition also passed, and the architectural file-size check
passed without exceptions. The receipt and exact source/log hashes are retained
in `tools/revoice/evidence/AUDIO_TIMELINE_DOMAIN_V1.json`.

These checks prove metadata rules only. Integration still needs verified measured
raw facts and the complete chunk bridge, manifest/digest ownership, managed FFmpeg rendering and stream/coverage/
peak verification, persisted jobs/cancellation/recovery, outbox publication,
native conflict controls, separate playable video and actual listening. The
ordinary installed application remains unchanged by domain-only development.
