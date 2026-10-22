# Chinese story revoicing execution

Started: 23 October 2026. Status: active; no complete release candidate exists.
This is the execution record for the owner's full three-input Goal. It extends
the older subtitle-only prompts; their previous ASR/TXT exclusions do not apply
to this Goal. Caster WPF, original media and existing accepted artifacts remain
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
  `feat/puza-tts-desktop`; pre-existing untracked `docs/maintenance/`.
- Pinned Translate `44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`; independent
  checkout `4319177b7e41512a2aabfc5cd30f49357b25d405` has pre-existing staged
  backlog/site changes. They are not interchangeable candidates.
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
| SRC-TXT-SHORT/LONG | Chinese authored text with paragraph identity and provenance                                         | Pending owner material; derived captions are engineering input only                         |
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

The owner has been asked for the existing neutral listening JSON, actual story
paths and a clean Windows target. Continue model-free implementation while those
answers are pending. No qualification scores or source suitability are inferred.

The subsequent [long-source host slice](acceptance/04-revoice-long-host.md)
adds configurable admission, schema 18, bounded plan/result reads, indexed cast
lookup and one-plan-at-a-time history. Its 600-cue application test passes; it
does not pass LONG-01/02 because decoded audio assembly is still eager and the
natural installed story has not run. The existing CONTRACT schema has no
512-item cap; the [host policy](revoice-input-contract.md#speech-admission-policy)
documents the new safety quotas without changing CONTRACT semantics.

## Owner steering, 23 October

The owner has no story folder and explicitly requested finding a webtoon on
YouTube and exercising the application through its video URL. The selected first
candidate is [the Chinese manhua recap 千面演員](https://www.youtube.com/watch?v=HRaa8kb1EhE)
from 天天动态漫. The browser player showed 1:13:55 and `Subtitles/CC Chinese`.
The frozen one-attempt metadata specification is
`tools/revoice/youtube-source-v1.json`. Selection does not establish downloadable
tracks, media access, alignment, source rights or translation quality. A 90-second
excerpt is a short diagnostic, not a claim of a complete short-story gate.

The owner said the new voices are good and requested leaving them alone. Keep
the source voice store and current manifests unchanged. This feedback is source
preference, not six per-engine listening scores; do not manufacture a qualified
report. Keep the native qualification prerequisite visible and do independent
work. Clean Windows is currently unavailable, as explicitly confirmed by the owner.

## Native admission compatibility slice

After the owner closed VALORANT/Riot, GPU use fell from 3,123 to 1,144 MiB and
no model process was found. The remaining native-UI helper is the installed
Codex `codex-computer-use.exe` under the user's versioned
`OpenAI/Codex/runtimes/cua_node/<16-hex-id>/bin/node_modules/@oai/sky/bin/windows/`
directory. The old guard recognized only the WindowsApps distribution path.
The model-free six-case path control reproduces rejection of this legitimate
location while rejecting model names and untrusted paths correctly.

The compatibility fix admits that exact executable name and constrained runtime
path shape in addition to the existing distribution path. It does not admit
games, arbitrary helpers, Python/model processes, unknown GPU owners or extra
leases. Mutex/file admission, process-tree release, WSL/container refusal and GPU
memory limits remain in force. Run path controls and both existing real admission
checks before further asset/model work. The first asset-copy attempt also exposed
an unavailable Windows PowerShell `Get-FileHash`; the bounded script uses streaming
.NET SHA-256 instead. Failed attempts remain recorded; no assets were copied by
the two refused attempts.
