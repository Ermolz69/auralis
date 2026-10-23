# Bounded raw TTS assembly, 23 October 2026

The next INT-R07 slice replaces the TTS joiner's list of complete decoded clips
and whole-story concatenation with bounded decoding, a private float32 disk
spool and a staged PCM16 WAV. This is raw speech concatenation; source-window fit,
managed FFmpeg ready audio and video publication remain separate open work.

Implementation is committed in TTS at
`e18f64d1bad80012b085d907673451c9e5b886f7`; wheel evidence is committed at
`e79edfe5adb69173431ef8ba26f6309e5e902e2b`. The prospective policy, checks and
limitations are in the scoped CONTRACT document committed at
`b1664dda1826af9c09b9e511be9d75bff92d8da0`:
`CONTRACT/Docs/tts/LONG_AUDIO_ASSEMBLY_20261023.md`.
No source/installed voice or qualification manifest was modified.

Each output core is <=65,536 frames. Mixed-rate clips use an explicit Kaiser-5
FIR with finite halos and denominator-aligned windows, preserving the previous
whole-array phase. The second pass applies the previous global 0.89125 peak
target and publishes by atomic replacement after flushing the candidate.
Cancellation checks run in metadata admission and each decode/write block.
Existing outputs survive cancellation, write failures and sharing violations.
Unique private temporary files are cleaned on cooperative exit; hard process
death can leave unaccepted temporary files, never a newly published partial WAV.

Supported rates are the declared standard 8–192 kHz set, with at most eight
channels. Unsupported properties, changed metadata, output/input aliases,
non-finite samples, incomplete frames and PCM16 RIFF overflow fail explicitly.
The assembly policy is recorded in manifest field `audio_assembly`; raw cache
identity remains unchanged. The host's existing manifest reader accepts this
additive field; no worker protocol fields or submodule pins were changed.

## Evidence and exact boundary

The nine initial focused checks passed, followed by 93 full unit tests. Adding
disk-write and replace-error controls produced a final **94/94 full-suite pass**
in 95.088 test-reported seconds. These checks use synthetic audio and mock
engines. They compare output against the previous independent whole-array
response within one PCM16 LSB, verify 600 ordered fragments and exact frame
coverage, prohibit whole-file reads and bound source reader requests including
FIR halos to 71,536 frames. Both cancellation stages and failure cleanup passed.
The initial sandbox fixture-access failure remains retained separately.

Docker Compose configuration passed. Docker Desktop is stopped and its engine
pipe is unavailable, so the conditional image build/GPU doctor was not run.
The diagnostic GPU snapshot is not a model-admission lease or model-performance
result. No real inference occurred in this slice.

Offline wheels built successfully. Isolated installation/import, bundled profile
model pins and the worker capabilities request passed. ZIP entries for the
joiner, renderer and manifest writer matched their tested source hashes.

| Wheel          | SHA-256                                                            |
| -------------- | ------------------------------------------------------------------ |
| PUZA TTS 0.3.0 | `40af9aba28b42c02220fe941805a114c23ad905737b5dc5df54d89e873e82723` |
| CONTRACT 0.2.0 | `55635e6411ff507a9f38e1e5f7eb58300871bf3bf2a62e92c2a9a21a1a6ce241` |

Current and previous wheel bytes are retained by hash outside Git. Immutable
source/test/package receipts are in `TTS/ci/tts/evidence/audio-stream-*-20261023.json`.
The ordinary installed application is still the 72e58c8 desktop candidate, with
its previous TTS package. A managed runtime upgrade, natural 4,004-cue source,
measured end-to-end RSS/disk/time, timing conflicts, real cancellation/recovery,
both engines and listening remain required before LONG-01/02 or INT-R07 can pass.
All 24 endpoint acceptance cases remain pending.
