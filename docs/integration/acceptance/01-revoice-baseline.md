# Full revoicing baseline and speech pagination

Date: 23 October 2026. Scope: INT-R01 baseline, partial INT-R02 source selection
and partial INT-R07 UI. This is not a passed native, translation, voice-quality,
long-story or installation gate.

## Candidate and evidence

Tested parent Auralis `b4af6e192329f29de41d07ab931a42439d7e9cb3` plus the scoped
working-tree changes. Translate remains pinned at
`44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`; the sibling checkout was not adopted.
Pre-existing `docs/maintenance/`, Translate staged backlog/site work and unrelated
CONTRACT documents were preserved. No Caster WPF or source-voice file changed.

`tools/revoice/evidence/BASELINE_20261023.json`, SHA-256
`917e3d33908ba47d0236cd9d38f3fb1d090f291a9699236add7216cba71aaa44`, records
repository states, installed pins and actual asset hashes. All 192 reference WAV
hashes and all 18 retained neutral probe WAVs verified. There are 16 distinct
probed combinations, **0 qualified, 192 unverified, 0 rejected**. This is metadata
and byte-integrity evidence, not listening or qualification. The owner asked to
leave the new voices unchanged; no scores or reports were applied.

The browser-discovered source is
[千面演員 on YouTube](https://www.youtube.com/watch?v=HRaa8kb1EhE), channel
天天动态漫, duration 4,435 seconds. The player exposed Chinese CC. The retained
`YOUTUBE_HRaa8kb1EhE_METADATA_V1.json` records sandbox `spawnSync EPERM` before
any extractor process started. A separately predeclared v2, with the same pinned
yt-dlp hash/settings and reviewed launch access, completed in 3,662 ms with exit 0. `YOUTUBE_HRaa8kb1EhE_METADATA_V2.json` records manual Chinese `zh` and
`zh-Hant` tracks, each with SRT and VTT formats. Automatic-track presence is
reported separately. Raw signed metadata stays under ignored `target/revoice/`.
No media, subtitle-byte, rights, actual Chinese speech or native-import claim is
made from this metadata screen. License metadata is unknown.

## Implemented behavior

Speech source preview now pages cues and roles independently at 25, 50 or 100
rows (default 50). Cue ordinal and source ID remain global. Voice/emotion
assignments stay in the parent and survive navigation and ordinary parent
refreshes. A changed source revision resets both pages. Metadata/index creation
is linear and memoized; DOM work is bounded by the page size. Full source data
still arrives in one existing bounded snapshot. Backend 512-cue admission,
SQLite limits and streaming long-plan work are still open under INT-R07.

The frozen plan has 17 integration tasks and 24 mandatory acceptance cases,
including TXT, ASR, the full natural source, 192 assessments, three listening
scenes and clean Windows. Its checker rejects lost mandatory cases, dependency
cycles, accepted tasks without evidence and false whole-Goal completion. The
new TXT/ASR/input/timing contract precedes their implementation.

## Commands and observed outcomes

| Command                                                                                                                                                                                               | Outcome                                                                                                                           |
| ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- |
| `task revoice:plan:check`                                                                                                                                                                             | 3 tests passed; 17 tasks / 24 mandatory cases valid; Goal remains active                                                          |
| `task revoice:audit OUTPUT=target/revoice/baseline-20261023-v1.json`                                                                                                                                  | Exit 0, 192 audio hashes and 18 retained probes verified; zero read errors                                                        |
| `task revoice:youtube:inspect`                                                                                                                                                                        | Retained launch failure, no metadata process started                                                                              |
| `task revoice:youtube:inspect SPEC=tools/revoice/youtube-source-v2.json`                                                                                                                              | Exit 0, same exact video, manual Chinese tracks listed                                                                            |
| `task fe:test:components -- src/features/speech-catalog/ui/SpeechSourcePreview.test.tsx src/features/speech-catalog/ui/SpeechWorkflow.test.tsx src/features/speech-catalog/ui/SpeechCatalog.test.tsx` | 3 files / 8 tests passed, including ordered traversal of 600 cues, bounded role/cue rows, assignment retention and revision reset |
| `task fe:typecheck`                                                                                                                                                                                   | Exit 0                                                                                                                            |
| `task fe:lint`                                                                                                                                                                                        | Exit 0                                                                                                                            |
| `task fe:build`                                                                                                                                                                                       | Exit 0; production bundle and translation-delivery checks passed                                                                  |
| `task docs:integration`                                                                                                                                                                               | 5 link-checker tests passed; 61 Markdown files valid, Markdown lint and formatting passed                                         |
| `task revoice:fmt`                                                                                                                                                                                    | Scoped source/document formatting passed                                                                                          |

Initial Node/Vite/doc runs hit sandbox `spawn EPERM`; reviewed process access
resolved those environmental failures. The new plan-only tests use Node's
in-process test runner. The first pagination tests used unavailable jest-dom
matchers; they were corrected to the repository's existing Vitest assertions,
then all eight tests passed. No product behavior was changed to hide a failure.

## Open next boundary

Build a committed installer candidate, configure ordinary installed components,
exercise the selected YouTube URL in the native application and retain its
actual artifacts. Production TTS still requires real qualification evidence;
the owner's source-voice preference was not substituted for engine-output
listening. The owner confirmed no clean Windows target is currently available.
Required cases remain pending. No model was loaded during this slice.
