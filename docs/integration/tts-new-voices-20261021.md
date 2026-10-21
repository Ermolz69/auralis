# PUZA speech integration checkpoint — 2026-10-21

## Scope and current status

Auralis now installs the eight current ElevenLabs v2 voices from CASTER. The selected engine, profile, neutral reference, reviewed text or approved subtitle translation, and exact cue lineage are frozen in the canonical speech plan before a run starts. Speech uses the existing application jobs, shared model admission, worker process ownership, checked segment cache, durable checkpoints, managed artifact outbox, and in-app WAV playback.

The voice manifests are real but remain `unverified`. The UI and application reject synthesis with an unqualified reference. No native speech run with a human-qualified new voice has completed, so the end-to-end user scenario is still pending the listening decision. Ciri and Geralt are not installation candidates.

## Evidence and installed components

`CONTRACT/Docs/tts/NEW_VOICES_AURALIS_20261021.md` records 18 sequential VoxCPM2/ESpeech probe WAVs, including a multi-sentence sample per engine. The exact input, seed, model revision, manifest and reference hashes, elapsed time, duration, signal measurements, and output SHA-256 for every case are in its versioned evidence JSON. The local listening page is `E:/Anything/Projects/Commercial/TTS/PUZA_TTS/.tmp_audio/new-voices-20261021/INDEX.html`.

`task setup:tts:local` copied the pinned native Python runtime, installed component wheels, copied the eight manifests and referenced files, and wrote `tts-runtime.json` in Auralis AppData. The installed worker listed all eight voices for both engines through the Rust adapter. The copied runtime's CONTRACT CLI passed installed-worker capability, plan, voice, result, process-fencing, and process-tree tests. The manifests retain `unverified` status.

`task tts:bundle:stage` created a standalone 48,367-file bundle in `target/tts-standalone-20261021`, and `task tts:bundle:install` installed it into `target/tts-standalone-install-20261021` using its own copied base Python. The new runtime and every configured dependency path are within that data root; it does not need the source checkouts, global Python, or the global Hugging Face cache at run time. The bundle manifest SHA-256 is `a0aa916d6d35ade15adef9a993cea92ff2edb4bcb476e201fa8b1b71e006cab6`. The installed CONTRACT and TTS wheel hashes are `0b345ff2fc3785b9b7828730045c6d9fab01cfba15ee36721326e43dfac790db` and `756f6ca0a75b8253c1c094e6c2293ce965e878c792619469fd11c7db80f10279`.

Four sequential real model probes from this installation (short and multi-sentence for each engine) reproduced the original WAV bytes exactly. `task tts:bundle:verify` checked their complete input identity, output hashes, durations, and WAV headers against the frozen source evidence. Its versioned record is `CONTRACT/VOXSCRIPT_CONTRACT/tests/evidence/STANDALONE_BUNDLE_20261021.json`.

The installed bundle's actual speech-plan renderer also passed twelve sequential technical cases, six per engine, with `task tts:bundle:renderer` and `task tts:bundle:renderer:verify`. The versioned record is `CONTRACT/VOXSCRIPT_CONTRACT/tests/evidence/RENDERER_CACHE_20261021.json` (SHA-256 `bd856028de9acbefd8a6f982ce1c8f9470ebec61ca5046de2fd5a023798b42a7`). For both engines, the baseline generated two segments, repeat reused both, a corrupted first cache file regenerated only that segment and reproduced the baseline WAV bytes, and changed text regenerated only its segment. Cancellation stopped after one durable checkpoint without a final manifest; a new worker process reused the saved first segment and generated the second. These are direct renderer evaluations with an explicitly unverified reference. They do not exercise native Auralis jobs or qualify the voice.

| Engine | Baseline WAV | Repeat / corrupt | Changed text | Cancel / resume |
| --- | --- | --- | --- | --- |
| VoxCPM2 | 5,920 ms, `8bbe9b05ba3ddb9fd71787783e7cf2e4ef4a7a28d65fe76255da4ca5adb0dfc6` | Byte identical to baseline | 5,440 ms, first segment only | One checkpoint; resumed WAV 5,440 ms |
| ESpeech | 5,739 ms, `9e928aba4be5e6fddc16e73d411de28627a4f140aaa5c4a9c106eeef38c2cf15` | Byte identical to baseline | 5,739 ms, first segment only | One checkpoint; resumed WAV 6,464 ms |

## Verification commands and limits

| Check | Result |
| --- | --- |
| `task rs:test:tts` | Passed: adapter framing, integrity, result verification, speech storage, application workflow. |
| `task rs:test:puza` | Passed: four contract tests and seven application speech workflow tests. |
| `task rs:test:tts-catalog:installed` | Passed against copied runtime and eight real voice manifests. |
| `task rs:test:contract-adapter:installed` | Passed all three installed and synthetic worker tests after installing current wheels into the copied runtime. |
| `task tts:bundle:install` and `task tts:bundle:verify` | Passed; four standalone real WAVs matched the frozen source hashes. |
| `task tts:bundle:renderer:verify` | Passed; twelve real renderer outcomes cover both engines, two-utterance WAVs, verified cache reuse, corruption, changed text, cancellation, and cross-process resume. |
| `task tts:test:admission` and `task tts:test:admission:metadata` | Passed with Translate/TTS shared Windows admission idle. |
| `task fe:test:components` | 57 files, 275 tests passed. |
| `task fe:build` and `task fe:smoke:puza` | Passed with a synthetic qualified UI fixture; this does not qualify a real voice. |
| Native app synthesis, cancel/restart/resume, corrupt cache, changed input, in-app WAV playback | Await human qualification of at least the two voices used by the native matrix. |
| Clean Windows installation | Not tested on a clean Windows environment. |

After a human decision is recorded through `task tts:qualification:apply`, commit the changed CASTER manifests and reports, update the pinned CASTER revision, rerun `task setup:tts:local`, rebuild the standalone bundle, then run `task desktop:e2e:native:speech`. Retain the native run's input plans, checkpoints, artifact hashes, WAV duration, and playback proof. A clean Windows check must be performed on a separate clean environment.
