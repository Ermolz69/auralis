# Explicit SRT candidate: installed upgrade, 23 October 2026

## Candidate and bounded local case

The production desktop bundle was built from committed Auralis
`72e58c84f17edc1e03227040140e2b86ca24e289`, with Translate pin
`44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`. The completed
`task desktop:bundle` verifies the real MSI media payload. Signing remains the
configured development-build skip; this is not a signed release candidate.

| Artifact                   | SHA-256                                                            |
| -------------------------- | ------------------------------------------------------------------ |
| NSIS installer             | `5c64b3ef28be7babffe9c69d20c8c6fabc0bfcb2867787db9e31aa5b6e560b26` |
| Installed NSIS application | `f3341ad4c1a7c5eb04689c4d9ce37ff58c801059f7dc4bb5746cb80c2c5a7618` |
| MSI installer              | `fea4b6689ccc7f4229d2c4e7b05bab5a47c7c51a9e5bf67db521ff441538e0a1` |
| Extracted MSI application  | `b2b2c78f6bf8f38914972a22a0c18a423a24c0ba01bb21b70cd101adf64c7e54` |

The distinct frozen case `tools/revoice/upgrade-selected-srt-v1.json` allowed one
ten-minute local upgrade. The engineering helper now accepts a validated case
identifier rather than labelling every future attempt as the old long-host case.
It verified the previous installed hash, zero projects, zero active jobs and
schema 18. Only the verified owned application tree was stopped. The SQLite
online backup includes WAL data, passes quick_check and has SHA-256
`3a86ba206301475dca91ff45c084c58d8b5f1fe2c19a81310ff083093422df79`.
The previous executable and configuration were retained beside that backup.

Installation exited 0 after 10,410 ms, and the installed executable matched the
NSIS payload. The TTS configuration remained exactly
`e80c4a3d0f8c6091547b1fcb14f61411b6e2d6402c79e4abcb3130fd73fb91a9`.
The earlier 70471a4 source executable, MSI and NSIS were also hash-verified and
retained under ignored `.cache/revoice-installers/70471a4/` before rebuilding.

## Observed fresh startup and limits

The official Windows UI helper launched registered `com.auralis.desktop`. The
newly discovered installed window was 63112944, titled Auralis. Fresh
accessibility showed the normal empty project page, enabled Create project and
Settings controls and no previous pin-import error. Read-only profile inspection
after launch still reported schema 18, zero projects and zero jobs.

Only launch, application discovery and accessibility reading were performed in
this fresh window. The previous helper screenshot/coordinate-input failure
remains unresolved; no repeated capture or blind keyboard actions were attempted.
This is startup evidence, not successful native interaction or a submitted video
URL. The owner already confirmed that the window and active desktop are visible.

Retained receipts are `tools/revoice/evidence/SELECTED_SRT_BUNDLE_V1.json`,
`SELECTED_SRT_UPGRADE_V1.json` and `SELECTED_SRT_NATIVE_STARTUP_V1.json`.
No model was loaded and no voice/qualification manifest was changed. Real URL
import, component installation, translation/edit/approval, both TTS engines,
playback, natural long-story assembly, TXT, ASR, listening and clean Windows remain
open. Independent work continues on bounded audio assembly in TTS; its prospective
policy is `CONTRACT/Docs/tts/LONG_AUDIO_ASSEMBLY_20261023.md`.
