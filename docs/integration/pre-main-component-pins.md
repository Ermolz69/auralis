# Active component source pin verification

Date: 2026-10-28. Auralis baseline: `bcef6e0`.

The active local setup pins now select the published portable TTS and speech
sources. The media lock selects the portable media source while preserving its
verified wheel bytes. CASTER already selected the reviewed source and remains
unchanged. Historical evidence and runtime/profile data are unchanged.

| Role   | Source revision                            | Verified package tree                      | Package version            |
| ------ | ------------------------------------------ | ------------------------------------------ | -------------------------- |
| TTS    | `49662267df9ee467e61a0d4e6a4caf7955f54a7d` | `0eb71b3933d08c38161177ffafd19644e248a0ed` | `puza-tts` 0.3.0           |
| Speech | `13f4383c7ca8b8b7b575eb64fdf9c54856a9204a` | `deab23554bc79faae0f5e402e1ab0f57adec8096` | `voxscript-contract` 0.2.0 |
| Media  | `6ee129cfcf534a7ca865c17ca5fa8195e8a028be` | `74a9d249a6546473e57f0d045ce32a4fc46eeacd` | `voxscript-contract` 0.3.0 |
| CASTER | `2dbab64abeb4d3e7eee7bc819a5578a7008b3c5d` | `c9cb634c4671f363dbc069560ea243c6e079b8fc` | `puza-voice-editor` 0.2.0  |

Trees cover `PUZA_TTS`, `VOXSCRIPT_CONTRACT`, `VOXSCRIPT_CONTRACT`, and `src`,
respectively. The updater also validates each repository's exact HEAD, documented
GitHub origin, clean tracked/untracked status, and package name/version before
writing either target.

| Wheel                                       | Bytes | SHA-256                                                            |
| ------------------------------------------- | ----: | ------------------------------------------------------------------ |
| `puza_tts-0.3.0-py3-none-any.whl`           | 57290 | `1b0fa4ea0e7b8f3e055194edf202bb24ff826e1a54bcf354f5036ebbd9a3cca6` |
| `voxscript_contract-0.2.0-py3-none-any.whl` | 75499 | `c8dcb3ecfbd64f821c56347ee8df039c8bff1d486c8d29f5967ac26a270e5381` |
| `voxscript_contract-0.3.0-py3-none-any.whl` | 78567 | `1d7760d6784fc89a143c7d9fd0c7bf230f6315691a082bfa44823c920d32a20e` |

Each wheel must match its exact length, SHA-256, and metadata identity. This task
consumes the previously built canonical wheels; it does not build/install packages
or load models. TTS requires `scipy==1.18.1` in base metadata, so `scipy` is added
to the setup tool's existing verified native dependency inventory.

## Fresh commands and outcomes

Commands use an existing CPython 3.12.14 executable via `PYTHON`. Component roots
are separate clean checkouts; wheel variables select the exact files above.

```text
task components:pins:check PYTHON=<python>
task --dry components:pins:update PYTHON=<python> TTS_ROOT=<tts> TTS_REVISION=<tts-sha> TTS_WHEEL=<tts-wheel> SPEECH_ROOT=<speech> SPEECH_REVISION=<speech-sha> SPEECH_WHEEL=<speech-wheel> MEDIA_ROOT=<media> MEDIA_REVISION=<media-sha> MEDIA_WHEEL=<media-wheel> CASTER_ROOT=<caster> CASTER_REVISION=<caster-sha>
task components:pins:update <same explicit inputs>
task components:pins:update <same explicit inputs>
```

- Fixture controls: initial exit 0, 12 tests passed in 47.733 seconds; final
  exit 0, 13 tests passed in 73.455 seconds after restricting origins to canonical
  GitHub hosts. These use real worktree-private temporary Git repositories and
  small wheel fixtures. They cover swapped,
  dirty, untracked, foreign and nested sources; abbreviated revisions; tree,
  version, wheel bytes and metadata mismatches; incomplete inputs; unexpected
  locks and local SSH aliases; rollback after a second-target replacement failure; and repeatability
  without content or timestamp changes. The setup module is never imported.
- Three restricted filesystem reruns returned Task exit 201 / Python exit 1
  with 13 fixture setup and 13 cleanup permission errors each. These did not
  reach assertions. The final full fixture run above had access to its own
  temporary directories and passed all 13 controls.
- Task dry run: exit 0.
- First actual execution: Task exit 201 / Python exit 1 because the retained
  wheel was unreadable in that filesystem context. Both targets stayed unchanged.
- Execution with access to the retained wheels: exit 0. Only
  `tools/tts/setup_local.py` and `tools/tts/media-contract.lock.json` changed.
- Identical repeat: exit 0, `updated: []`.
- Two further identical updates using the final canonical-origin helper:
  exit 0 each, `updated: []` each.

The TTS setup pin advances from `22dcbb12a37bc1e54d0d8cf7f52adb9668de36a7`;
the speech setup pin advances from `4ab50d5dffbe611dc7d60d0fe292614f6c47cd8a`.
Media advances from source `0cc6898abb149eeb983c767655e5c88c3f3f854a` and package
tree `b632d82674717533a65f44536307b4bf0313cabc` to the table above. Its package
version and wheel hash are unchanged.

These checks establish source/wheel pin consistency only. They do not establish
the full Auralis gate, runtime installation, model execution, or voice qualification.
