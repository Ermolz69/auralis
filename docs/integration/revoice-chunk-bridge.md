# Verified TTS utterance bridge, 23 October 2026

This INT-R08 slice connects the retained TTS result verifier to managed audio
assembly. It is staging infrastructure; native playback, durable assembly jobs,
outbox publication and natural-story acceptance remain required.

## Policy fixed before verification

Accept only the TTS policy `pcm16_stream_fade10ms_peak089125_kaiser5_v1` for
frame-range extraction. Existing result verification remains compatible with old
manifests; requesting this bridge for an unknown policy fails with an upgrade
action. Do not change installed runtimes or voice qualification in this slice.

Verify the exact reviewed plan, cast, model/profile identity, ordered technical
chunks, text coverage, cache hashes and retained WAV/manifest hashes first.
Measure each PCM16 cache chunk and derive its assembled frame count with integer
ceiling `raw_frames * 48000 / raw_rate`, using the streaming joiner's thirteen
supported rates. Sum consecutive chunks into complete utterances. Require all
ranges to partition the entire 48 kHz mono/stereo aggregate exactly, without gaps,
overlaps or duplicated utterance IDs. Preserve source windows and bind the layout
to the exact saved plan and raw manifest hashes. No timing is inferred from the
rounded chunk duration in milliseconds.

Under the existing shared machine admission, copy and hash the aggregate to a
private bounded snapshot. Extract complete utterance PCM ranges with 64 KiB I/O
and fresh WAV headers, hash each file and feed those measurements to the existing
timeline/FFmpeg adapter. Preserve samples, global normalization and existing chunk
fades exactly. Do not resynthesize, renormalize, resample, add fades, trim or change
tempo. Include the raw source layout in the staging manifest. Original cache,
aggregate and earlier results remain unchanged.

Extraction and assembly share cancellation and the existing total deadline.
Bound both the snapshot and extracted WAV bytes together by `max_input_bytes`;
the existing assembly input and output caps still apply. With the intermediate
snapshot/extraction and normal assembly staging, temporary audio is bounded by
twice this input allowance plus the output allowance, excluding bounded metadata
and RIFF headers for inserted silence. This conservative input accounting can
reject an aggregate near half the configured input cap. Partial work stays in
temporary directories and never becomes a ready artifact.

## Finite model-free matrix

Add Taskfile commands before running controls. Cover a reviewed utterance with
multiple technical chunks, per-chunk ceiling at 44.1/24 kHz, exact ordered ranges,
old/unknown policy refusal, mismatched aggregate coverage, changed chunk/hash/
properties, invalid rates/channels, duplicate IDs, source windows and 600 complete
utterances. On the assembly side cover stereo sample-exact extraction, preserved
originals, safe generated names, stale aggregate hash, malformed ranges, input
budget, cancellation and impossible subtitle-window fit. No models or real
FFmpeg are invoked by this matrix. Retain failed checks separately.

Run the existing TTS/storage/application and audio assembly regressions, scoped
Clippy, file-size/docs/plan checks and normal desktop composition. A real worker
and FFmpeg combined probe needs its own committed candidate, frozen sources,
one-attempt case budgets and receipts; synthetic tests do not close that gate.

## Model-free implementation results

`verify_result_audio_layout` shares the existing retained-result validator and
returns a typed `SpeechAudioLayout`. The saved plan hash is calculated from the
exact bytes that were parsed; the manifest's parsed bytes must match worker
completion. Chunk IDs preserve the valid 64-character utterance-ID boundary even
when the derived chunk ID is longer. `SpeechAudioAssemblyPort::assemble_speech`
on the existing FFmpeg adapter accepts this layout, stages complete utterance
WAVs and records the raw layout in `assembly.json`. Its shared admission, owned
process, cancellation, release and final PCM verification remain the same as the
file-input assembly port.

`task rs:test:audio:bridge` passed 11 retained-result/layout tests and 19 assembly
tests, including the pre-existing process lifecycle controls. The tests use
synthetic PCM and controlled Rust fixture children; they invoke no model or real
FFmpeg. All thirteen supported rates, per-chunk ceiling, a two-chunk utterance,
600 utterances, stereo samples, exact source hashes, legacy policy compatibility,
unknown policy refusal, stale/corrupt inputs, range gaps/overlaps, byte budgets,
cancel after the first 64 KiB PCM block, source-window silence and impossible fits are
covered. The original aggregate stays unchanged and failed extraction removes
its temporary directory.

The first model-free run passed all 9 then-existing layout tests and 15 assembly
controls but the 600-item extraction reached its 20-second deadline. It flushed
every intermediate WAV to physical storage. Intermediate files now use a normal
flush, matching the existing verified-copy path; only final staged WAV/manifest
files require `sync_all`. After that code change, the expanded 18-test assembly
suite passed in 7.56 seconds with the same 20-second extraction deadline. This is
test runtime, not a natural-story performance measurement. Both logs are retained
separately; no experiment budget or threshold was widened.

The installed app/runtime, original audio/cache and voice reference or
qualification manifests were not changed by this code slice. It does not accept
native, natural-story, listening, video-export or clean-Windows cases.

`task rs:test:audio:assembly` passed shared WAV, timeline, domain, TTS transport,
speech storage/migration and application workflow regressions. Scoped
`task rs:clippy:audio:bridge` passed production/test targets with warnings denied.
File-size, documentation and full-goal plan checks passed; all 24 endpoint cases
remain pending. The final source hashes and separate failed/successful logs are
recorded by `task revoice:bridge:evidence` in
`tools/revoice/evidence/TTS_CHUNK_BRIDGE_V1.json`, with log copies outside Cargo
target in `.cache/revoice-checks/chunk-bridge-v1/`.
