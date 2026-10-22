# Local installation and admission compatibility

Date: 23 October 2026 (local UTC+3; receipts use UTC). Scope: partial INT-R05.
The ordinary application installed and launched. Native YouTube import,
translation, synthesis and clean Windows acceptance remain unpassed.

## Installed candidate

The production bundle was built from committed Auralis
`b6dc303f4f8794b1f3b586fe829073c82dc8bc2c`, Translate
`44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`. It includes the new source-preview
pagination. It predates the admission-path compatibility change below; a later
candidate must be rebuilt before that fix can be claimed in the installed app.

`task desktop:bundle` passed: frontend production checks, Rust release build,
MSI/NSIS packaging and actual MSI payload verification. Prior executable and
installers were retained under `target/revoice/pre-goal-installers-20261023/`.
No model weights or test-runner payload were bundled. Signing was skipped by
the configured non-production packaging policy; this is not a signed release.

| Artifact                   | SHA-256                                                            |
| -------------------------- | ------------------------------------------------------------------ |
| NSIS installer             | `34b17c418aeb38f4eefdcd04cf8201fa39eeb6e06deca7b7b0547c1171ea3d9f` |
| NSIS installed application | `7f944769255803623d6226318f1d5d96e1b4e8e27e619ceec00111e7a1bf3a22` |
| MSI installer              | `bf95235dafbdad10c1cd4d88023e7d8275a220575ff773a9358f9b9b9a7aa849` |
| MSI application            | `d79c0510deea5fd3a0a2274f27d9b9e8dbacfac31217bdba815aa14c1b2a3e4c` |

The separate application hashes are expected: existing bundle machinery embeds
the captured installer kind into each payload. The source binary is restored
after bundling. `tools/revoice/evidence/LOCAL_INSTALL_V1.json` retains the
8,156-ms installer receipt, exit 0 and verified installed payload identity.
Installation used the ordinary per-user `Programs/Auralis` directory and the
ordinary `com.auralis.desktop` data root. This host has a developer checkout and
existing TTS assets. It cannot establish clean Windows or offline acquisition.

## Sequential asset preparation

The first copy attempt failed because the inherited Windows PowerShell runtime
could not resolve `Get-FileHash`. No copy occurred; streaming .NET SHA-256
replaced that dependency. Attempt v2 then refused admission while Riot still
owned GPU resources. The owner closed the game and authorized application tests.
Attempt v3 verified and copied four already available, pinned assets under the
real shared `-ModelFree` guardian: Hy-MT2 1.8B Q4_K_M, model license, llama.cpp
license and CPU b10977 runtime archive. Admission and release both succeeded;
elapsed time was 16,348 ms. Original files were unchanged.

The two retained receipts are
`tools/revoice/evidence/TRANSLATE_ASSETS_V2.json` and
`tools/revoice/evidence/TRANSLATE_ASSETS_V3.json`. This is raw cache preparation,
not an installed/selected translator, inference, measured translation quality
or a clean acquisition pass. No model was loaded.

## Admission compatibility

The current official Codex native helper also runs from the user's versioned
`OpenAI/Codex/runtimes/cua_node/<16-hex-id>/.../codex-computer-use.exe` path.
The old guard allowed only its WindowsApps distribution path. The fix accepts
the exact process name and anchored versioned runtime path. Unknown locations,
model names and invalid runtime IDs remain refused. All other admission and
release rules are unchanged.

| Command                                                                                                                                                                                                                                        | Observed outcome                                                    |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------- |
| `task tts:test:admission:computer-use-path`                                                                                                                                                                                                    | Six path/name controls passed; no models or lease started           |
| `task tts:test:admission`                                                                                                                                                                                                                      | Shared lease exclusion and release passed                           |
| `task tts:test:admission:metadata`                                                                                                                                                                                                             | Shared model-free lease exclusion and release passed                |
| `task rs:test:model-admission`                                                                                                                                                                                                                 | Five passed, one existing interop test ignored                      |
| `task revoice:translate:seed-assets OUTPUT=target/revoice/normal-profile-translate-assets-v3.json`                                                                                                                                             | Four pinned files verified before and after copy, guardian released |
| `task revoice:install:local INSTALLER_SHA256=34b17c418aeb38f4eefdcd04cf8201fa39eeb6e06deca7b7b0547c1171ea3d9f APPLICATION_SHA256=7f944769255803623d6226318f1d5d96e1b4e8e27e619ceec00111e7a1bf3a22 OUTPUT=target/revoice/local-install-v1.json` | Installed, exit 0, payload verified                                 |

## Native attempt and next boundary

The installed app was launched through its registered app ID. Its single
returned window was titled Auralis. Accessibility showed the normal project
screen and accepted an unsaved project-name value. No project was created:
button input failed with `coordinate input geometry is unavailable`.
Screenshot capture failed with `FrameArrived timed out: timed out waiting on
channel`; after refreshing window selection and activation, the one recovery
attempt failed with `window capture timed out: timed out waiting on channel`.
Keyboard Tab did not produce a confirmed focus change. Stop app input until
the native helper/display state changes. No lock-screen window was observed;
do not claim the desktop was locked.

Startup logs show successful database startup, project listing and recovery.
They also show a separate existing `import_project_pins_cmd` BUSY error during
concurrent legacy-pin import. This is not proof of a hung backend, and remains
an actionable independent defect. The selected YouTube URL and Chinese track
have not yet been submitted. The predeclared native case remains pending.

New voices and qualification reports were not changed. The clean Windows gate
and actual listening gates remain open, as recorded in the full execution plan.
