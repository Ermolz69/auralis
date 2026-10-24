# Chinese story revoicing execution

Started: 23 October 2026. Status: active; no complete release candidate exists.
This execution record covers subtitles, Chinese TXT and subtitle-free video via
ASR. All three inputs require short and long acceptance scenarios. Caster WPF,
original media and existing accepted artifacts remain
outside the mutation scope.

## Completion and ownership

The finite matrix below is mandatory for **one committed installed candidate**.
An engineering test, direct renderer result or short native milestone passes
only its stated boundary. Missing listening, source admission, long-story or
clean-Windows evidence prevents completion. Translate task state stays in its
canonical `docs/IMPLEMENTATION_BACKLOG.md`; the bindings here do not change it.
G1–G9 and published-reference protocol v2 retain their existing thresholds and
AI/reference provenance requirements. An external bilingual reviewer is not a
prerequisite. A1–A6 remain additional audio gates.

Use the [input and timing contract](revoice-input-contract.md) before implementing
TXT or ASR. The machine-readable `revoice-plan.json` owns integration task
status, dependencies and evidence references. `task revoice:plan:check` checks its
integrity. Do not remove required cases or turn a missing resource into a pass.

## Baseline audit

Read-only inspection found:

- Auralis `b4af6e192329f29de41d07ab931a42439d7e9cb3`, branch
  `feat/puza-tts-desktop`.
- Pinned Translate `44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`; independent
  checkout `4319177b7e41512a2aabfc5cd30f49357b25d405` is a distinct candidate.
- CONTRACT `d9596ce435c55422ada7005c7b00427103b08c09` has unrelated documentation
  changes; TTS `22dcbb12a37bc1e54d0d8cf7f52adb9668de36a7`, CASTER
  `2dbab64abeb4d3e7eee7bc819a5578a7008b3c5d`, MARKER
  `fef5b027be2b7de8c50d5093d64151294b5c8383` were inspected separately.
- The normal Auralis data root has `tts-runtime.json` and a pinned component
  receipt. `translation-runtime.json` and `caster-runtime.json` were absent.
  A development executable or standalone TTS bundle is not installation proof.
- Installed voices were read on 23 October: eight manifests, 24 references each,
  **0 qualified / 192 unverified / 0 rejected**. Neutral reports remain unscored.
  Existing 18 probe WAVs cover 16 neutral pairs and two longer Marina samples.
- The 512-utterance restriction exists in source admission, storage validation
  and the SQLite schema. Speech preview currently mounts all source cues.
- Native speech tests explicitly disable translation. Their 13-run matrix does
  not establish the combined Chinese-to-Russian-to-WAV workflow.
- The historical TTS execution table predates substantial implementation. Use
  the [21 October checkpoint](tts-new-voices-20261021.md) and current code for
  scoped evidence; neither record passes current native or listening gates.

`task revoice:audit OUTPUT=<new-report-path>` records current repository/pin,
configuration, wheel, profile, reference and retained-probe hashes without
loading models or changing a profile. Use a new output path for every attempt.
An unreadable installed asset is an audit error, not an absent or qualified asset.
Initial sandbox denials reading installed voices were resolved by a read-only
escalation. Git's `submodule status` could not start `sh` in the sandbox; direct
`ls-tree HEAD` plus submodule `rev-parse HEAD` established the pin instead.

## Materials and experiments

No new model experiment is admitted until its source/model/profile/voice hashes,
code identity, expected invariants and budgets are frozen in a new case record.
Use owned copies, record both original and copied hashes, and never edit originals.

| Material ID        | Required selection                                                                                   | Current disposition                                                                         |
| ------------------ | ---------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- |
| SRC-STORY-SHORT    | Genuine short Chinese story, matched video and Chinese SRT/VTT or selectable YouTube Chinese track   | First candidate: 0–90-second owned excerpt of the YouTube manhua below; screening pending   |
| SRC-STORY-LONG     | One complete natural long story, including more than 512 cues                                        | First candidate: complete 1:13:55 YouTube manhua below; cue count and accessibility pending |
| SRC-LISTEN-1/2/3   | Three distinct scenes of 10–20 minutes with approved Russian text                                    | Pending story selection; freeze exact windows before synthesis                              |
| SRC-TXT-SHORT/LONG | Chinese authored text with paragraph identity and provenance                                         | No admitted authored-text candidate; derived captions are engineering input only            |
| SRC-ASR-SHORT/LONG | Chinese speech video and independently trustworthy timed transcript                                  | Pending; existing machine ASR cannot serve as ground truth                                  |
| SRC-HOLDOUT        | At least 300 eligible sealed cues from independent works, matched published Chinese/Russian versions | DATA-04/05 open; existing exposed FLORES and technical screens are excluded                 |

Already retained Vivo (18:35.570), ASUS (14:42), Kirin (12:42), lesson and other
technical media may support private diagnostics after hash verification. They
are not Chinese AI stories or an independent subtitle holdout. Their historical
rights, alignment and review gaps remain open. Do not rerun old acquisitions
under old attempt IDs.

Per-case registration includes source group/split, provenance, original and copy
SHA-256, cue/paragraph count, duration, exact code plus dirty hashes, package and
model revisions, profile/seed, voice/reference/report hashes, expected coverage,
output consumers, limits, attempts and stop conditions. Russian holdout references
stay sealed from the translation model until raw/accepted output is durable.

Initial experiment ceilings are **limits, not measured throughput promises**:

- Existing short voice probes: one attempt per frozen input, 30 minutes for
  VoxCPM2 and 45 minutes for ESpeech; no semantic/quality retry. At most two
  classified transient retries, each separately retained, after reconciliation.
- Short complete native case: 2 hours total per engine; long case: 48 hours per
  engine, with durable checkpoints and a bounded stop at the limit. Refine the
  long-case budget prospectively after the measured short case, preserving both
  records. No unlimited background rendering or parameter search.
- Physical GPU target: RTX 3070 8 GiB, to be rechecked; baseline at most 2,048 MiB
  used and sampled total at most 7,680 MiB; model-process RAM ceiling 24 GiB.
  Record 1-second samples, observer failures and approximate peak limitations.
- One shared machine admission covers download/load/inference/doctor/render/ASR
  and translation. Prove absence of external competitors, record admission and
  wait for process-tree exit and resource release between engines.
- Stop on changed identity, invalid qualification, unknown process ownership,
  missing asset, exceeded memory/time, corrupt output or incomplete coverage.
  Cancellation uses the existing 15-second cooperative worker deadline followed
  by owned-tree termination; measure actual latency and retain late-event tests.

## Mandatory acceptance cases

| Cases           | Required observable evidence                                                                                                                                                               | Bindings                                                   |
| --------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ---------------------------------------------------------- |
| SUB-01/02/03    | Real short SRT, strict VTT and selected Chinese YouTube-track import; translate, edit, history/select, exact-version approve, assign, synthesize, ready WAV and in-app playback            | INT-TR01–03, INT-T01–06, INT-V01–04, INT-A01/02            |
| TTS-NATIVE      | Existing 13-run VoxCPM2/ESpeech matrix: text/subtitles, voices, publication/playback, cancel, app death, resume, repeat, corrupt cache and changed text                                    | INT-T01–06; direct renderer evidence is separate           |
| LONG-01/02      | More than 512 distinct cues and the full natural long story: paged UI, bounded working set, exact ordered coverage, no loss/duplicates                                                     | LONG-01/03/04/05/06, INT-A05                               |
| FIT-01/02/03    | Continuous narration, source-window fit with silence, and impossible-fit/overlap negative controls; raw speech retained                                                                    | VOICE-03/07, INT-A01/03                                    |
| EXPORT-01/02    | Ready WAV and separate MP4 with Russian audio, stream/duration/coverage/clipping checks; actual Auralis and external playback                                                              | G8, A2/A3/A6, INT-A02/03                                   |
| TXT-01/02       | Short and long Chinese text import/edit, persistent paragraph IDs, translation history/review, continuous Russian WAV, recovery and playback                                               | New INT-IN01/02 boundary; existing translation core reused |
| ASR-01/02       | Short and long subtitle-free video, real Chinese ASR, timed transcript editor/revision approval, translation, WAV/video and playback                                                       | Existing ASR-01, new INT-IN03/04, INT-A01–04               |
| LANG-01         | Same-candidate G1–G9, at least 300 independent eligible cues, >=95% adequacy >=4/5, zero unresolved detected critical facts, >=98% applicable frozen terms                                 | DATA-03/04/05, CTX-03/04, RELEASE-01/02/03/05              |
| REC-01          | Cancel, worker/app death, restart, duplicate commands, stale review, changed cast/profile/text, cache damage and publication gaps at applicable stages                                     | G7, A5, HOST-01/04, INT-T03/04, INT-A04                    |
| LISTEN-01/02/03 | Three 10–20-minute scenes: zero unresolved critical meaning/speaker defects; >=95% eligible utterances >=4/5 intelligibility and naturalness; actual listener decisions                    | A4, VOICE-04                                               |
| VOICES-192      | Every one of 8 voices x 12 emotions x 2 engines has a retained real probe and explicit assessment; rejected pairs remain unavailable                                                       | INT-V02, INT-T02/05                                        |
| INSTALL-01/02   | One pinned candidate on clean Windows; explicit acquisition, interrupted-install recovery, restart, complete offline subtitle/TXT/ASR workflow without checkout/global Python/seeded cache | G9, A6, INT-TR05, INT-T07, INT-A06                         |
| AUDIT-01        | Committed code/submodule/wheels/runtime/models/profiles/installer and all gate evidence agree after final fixes                                                                            | RELEASE-05, INT-A06                                        |

External WAV/video consumers for the final candidate are Auralis, Windows Media
Player and VLC when available. Record exact consumer versions and playback
decisions; successful FFprobe decoding alone does not establish playback.

## Next implementation slice

INT-R07 starts with bounded speech-preview pages and identity-safe navigation.
This removes eager DOM growth only. Raising the backend cap also requires a
prospective CONTRACT limit, configuration, a forward SQLite migration, bounded
plan/checkpoint/result handling and >512-cue integration tests. Do not claim the
whole long-story gate from the UI slice.

Neutral listening decisions, admitted story inputs and a clean Windows target
remain required acceptance resources. Model-free implementation can proceed
independently. No qualification scores or source suitability are inferred.

The subsequent [long-source host slice](acceptance/04-revoice-long-host.md)
adds configurable admission, schema 18, bounded plan/result reads, indexed cast
lookup and one-plan-at-a-time history. Its 600-cue application test passes; it
does not pass LONG-01/02 because decoded audio assembly is still eager and the
natural installed story has not run. The existing CONTRACT schema has no
512-item cap; the [host policy](revoice-input-contract.md#speech-admission-policy)
documents the new safety quotas without changing CONTRACT semantics.

## Source and environment status, 23 October

The subtitle-first scenario uses a YouTube manhua source through its video URL.
The selected first
candidate is [the Chinese manhua recap 千面演員](https://www.youtube.com/watch?v=HRaa8kb1EhE)
from 天天动态漫. The browser player showed 1:13:55 and `Subtitles/CC Chinese`.
The frozen one-attempt metadata specification is
`tools/revoice/youtube-source-v1.json`. Selection does not establish downloadable
tracks, media access, alignment, source rights or translation quality. A 90-second
excerpt is a short diagnostic, not a claim of a complete short-story gate.

Keep source voice stores and qualification manifests unchanged. Per-engine
listening decisions must be recorded separately; no scores or qualified reports
may be inferred from successful synthesis. The native qualification prerequisite
remains visible. No clean Windows acceptance target has been admitted.

The [explicit SRT candidate upgrade](acceptance/07-revoice-selected-srt-upgrade.md)
is now installed with exact package/executable hashes and a retained profile
backup. Its fresh normal startup exposes the empty project page through
accessibility. Native URL input remains unproven because the earlier helper
capture/input problem is unresolved. Long-audio assembly continues as an
independent TTS slice without modifying voices or qualification decisions.

The [streamed raw-audio slice](acceptance/08-revoice-streamed-raw-audio.md) is now
committed and verified with 94 model-free unit checks and isolated wheel smoke.
It bounds decoded audio and preserves cancellation/atomic output, but has not
replaced the installed runtime or passed a natural story. Managed timing/export,
real recovery, measured working sets and listening remain open.

The next [audio timing policy](revoice-audio-assembly.md) has a pure domain
planner with complete frame accounting, explicit silence and typed overlap/fit
conflicts. Its ten timing controls and 21 domain unit checks pass. It is not yet
wired to verified worker chunks, managed FFmpeg, durable jobs or native controls,
so it does not accept any timing/export gate.

The [managed PCM assembly slice](revoice-ffmpeg-assembly.md) now implements a
hash-pinned owned FFmpeg adapter, bounded source snapshots, exact expected PCM
coverage/peak verification and confirmed shared-admission release. Its
model-free checks pass. The [first real FFmpeg fixture matrix](acceptance/09-revoice-managed-ffmpeg.md)
also passed three one-attempt cases with exact PCM hashes and confirmed resource
release, including 600 entries. These remain synthetic staging results; the
real worker/assembly integration, durable publication, native workflow and
real-story quality/recovery gates remain open.
The [verified chunk bridge](revoice-chunk-bridge.md) now derives complete
utterance ranges from the known streaming TTS policy and extracts exact PCM for
the same adapter. Its synthetic controls cover 600 utterances, mixed-rate chunk
ceilings, hashes, cancellation and fit conflicts. Installed real-worker evidence
and application orchestration are still required.
The [managed layout publication slice](revoice-audio-publication.md) connects the
known-policy layout to the existing raw-speech transaction/outbox, including
cancellation and restart. Assembly orchestration and native timing/export are
still open.
A fresh installed window was tested with two bounded capture attempts. Both
timed out; no project/input or native workflow was confirmed. The capture helper
binary version was not independently verified. The normal profile remains an
empty test profile.

## Native admission compatibility slice

The resource baseline found no model process and measured 1,144 MiB of GPU use.
The old admission guard recognized only one official Windows capture-helper
distribution layout. The six-case path control verifies a second supported
versioned runtime layout while rejecting model names and untrusted paths.

The compatibility fix admits the exact helper executable name and constrained
runtime path shape in addition to the existing distribution layout. It rejects
unrelated applications, arbitrary helpers, Python/model processes, unknown GPU owners or extra
leases. Mutex/file admission, process-tree release, WSL/container refusal and GPU
memory limits remain in force. Run path controls and both existing real admission
checks before further asset/model work. The first asset-copy attempt also exposed
an unavailable Windows PowerShell `Get-FileHash`; the bounded script uses streaming
.NET SHA-256 instead. Failed admission/hash-tool attempts remain recorded and did
not complete the asset-copy phase.

## Durable assembly application slice

The [managed assembly workflow](revoice-assembly-jobs.md) now connects ready raw
speech, immutable timing review, separate common jobs, verified staging and
transaction/outbox publication. Schema 19 preserves existing source tables and
adds normalized outputs with cancellation tombstones. The desktop panel shows
whole-utterance measurements, continuous pauses or declared original-window
timing, typed conflicts, saved plans, cancellation/resume and separate assembled
WAV playback. It remains accessible without a working TTS catalog.

The model-free receipt `tools/revoice/evidence/AUDIO_ASSEMBLY_JOBS_V2.json`
binds source hashes to bounded tests and retains distinct failed attempts.
Fixture execution and synthetic PCM demonstrate orchestration/storage/UI
boundaries only. No new real FFmpeg or installed-native experiment was performed.
INT-R08/INT-R09 remain in progress; all required endpoint cases remain pending.
Video export, actual media-duration verification, permitted tempo, TXT, ASR,
natural-story evaluation, listening and clean Windows remain open.
