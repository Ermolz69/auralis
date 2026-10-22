# Explicit SRT retrieval for the Chinese story

Date: 23 October 2026. Scope: partial INT-R02/05. Installed application is still
`70471a4`; this correction needs a new committed bundle. No native import,
inference, speech quality, long-story or clean Windows gate is passed here.

## Genuine source

The earlier VTT failure remains unchanged. A separate frozen case requested
the provider's SRT representation of the same manual `zh` track of
[HRaa8kb1EhE](https://www.youtube.com/watch?v=HRaa8kb1EhE). No automatic caption
fallback, header stripping or local VTT conversion was used.

`task revoice:youtube:captions SPEC=tools/revoice/captions-srt-v1.json` completed
under the real shared model-free guardian: HTTP 200, 412 ms, guardian released.
The SRT is 236,229 bytes, SHA-256
`868dc24073183cfd9e5f41ded3eb0cca34b6faad53e008a8b50ddb6f79c49735`.

`task rs:test:source-inspect SPEC=tools/revoice/source-inspect-srt-v1.json`
passed through the pinned production FormatInspector in 109 ms: **4,004 cues**,
89,188 UTF-8 text bytes, first start 100 ms, final end 4,435,266 ms. Every cue ID
and ordinal is unique and ordered; original bytes are unchanged. The ordered
mapping digest is
`eccf2156d523653ff4de35fb6fd33e7e45c32ae6abbac07a27796ff153b99ef5`.
Metadata's integral duration is not a frame-accurate media duration; actual
downloaded-video probing and alignment remain required.

Both originals were hash-verified and copied to private
`.cache/revoice-materials/HRaa8kb1EhE/<sha256>.<format>` using
`task revoice:material:retain`. They survive Cargo target cleanup, remain outside
Git and were not rewritten. Retention receipts are in
`tools/revoice/evidence/RETAINED_SRT_V1.json` and `RETAINED_VTT_V1.json`.
Acquisition/admission/inspection receipts are `YOUTUBE_SRT_V1.json`,
`YOUTUBE_SRT_ADMISSION_V1.json` and `YOUTUBE_STRICT_SRT_V1.json` in that directory.
This exposed diagnostic source group is not a sealed quality holdout or trusted
ASR ground truth. Chinese audio has not been independently assessed.

## Product correction

The subtitle list now exposes only supported SRT/VTT representations and sorts
same-language/type SRT before VTT. The UI labels their formats distinctly,
supports format search and sends the exact selected track ID. It no longer
displays a fictitious fixed `subtitles/source.vtt` result path.

Both legacy and cancellable yt-dlp paths honor the selected format and discover
that extension. Unsupported representations fail before allocation/spawn.
Legacy lower-level callers with no recorded representation retain their VTT
default; the existing unselected-track VTT policy is unchanged. This slice adds
**explicit SRT selection**, not a silent switch of track or language.

The durable job descriptor accepts SRT or VTT. The queued and direct runners
carry an injected format-inspection port; normal desktop composition supplies
the existing Translate FormatInspector. Selected SRT is parsed strictly, its
hash is checked against the bounded workspace read and its source lines/times
are mapped into the transcript. The original SRT is staged with its correct
extension. Missing inspection wiring or malformed SRT refuses the import; no
partial transcript/artifact is accepted. The VTT grammar is unchanged.

Workspace tests also exposed duplicate sibling React keys for speech and source
import. Their keys now have distinct prefixes, retaining the same project reset
semantics and removing the React warning.

## Checks and retained failures

| Command                                                                      | Observed result                                                                                      |
| ---------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------- |
| `task rs:test:youtube:srt`                                                   | 2 real fixture-process routing controls, 1 queued SRT/negative-grammar test and 19 port tests passed |
| `task rs:test:queue`                                                         | 25 passed across queue, recovery and process-lifetime checks                                         |
| `task rs:test:youtube`                                                       | Passed; runner summaries report 41 including fixture children                                        |
| `task rs:test:application`                                                   | Passed; runner summaries report 194 passed / 6 ignored, including fixture children                   |
| `task rs:clippy:queue`                                                       | All library and relevant test targets passed with warnings denied                                    |
| `task rs:check:speech:desktop`                                               | Normal desktop composition passed                                                                    |
| `task fe:test:components -- src/pages/project/ui/SubtitleWorkspace.test.tsx` | 7 passed; repeated after the key correction without warnings                                         |
| `task fe:typecheck`, `task fe:lint`, `task fe:build`                         | Passed; production bundle/delivery budgets passed                                                    |

The process controls execute a local Windows fixture in a path with spaces
through both downloader APIs and assert requested format, resulting extension
and bytes. They never contact YouTube. The queue control uses real temporary
SQLite, dispatcher, workspace, artifact staging and strict parser with a fake
download source. It verifies the selected Chinese SRT, cue times, line breaks,
byte count and extension; malformed grammar leaves no accepted transcript or
artifact. None of these fixtures qualifies voices, audio or real native input.

Development compile failures in the new tests used the wrong public adapter
import and omitted an error import / optional artifact field; these were fixed
before their actual execution. The broad application build then caught a
separate mistake in the previous host slice: `speech_workflow_long.rs` was a
child-module helper placed at the test root, so Cargo also tried to run it as a
standalone integration target. It now lives under `tests/support/`, and both the
specific speech workflow and the full application test graph pass. Old failure
logs remain; passing logs use new paths under `target/revoice/srt-*-check-*`.

## Native boundary and next step

The installed 70471a4 app still cannot be captured. A bounded keyboard system-menu
diagnostic also produced no confirmed menu/focus state; accessibility returned
null after Alt+Space and after a cancelling Escape attempt. No menu item was
selected and no project command was confirmed. The nullable response was
handled; stop further unchanged native input attempts. This is not a claim that
the desktop is locked or that the application backend is hung.

Build the newly committed SRT candidate, preserve the existing package/profile
and verify the upgrade separately. The real video URL import, component setup,
translation/review, real TTS, playback, streaming long audio, TXT, ASR, full
quality/listening matrices and clean Windows remain open. Source voices and
qualification reports were not changed.
