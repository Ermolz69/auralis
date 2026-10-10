# Goal for full TTS integration in Auralis

Updated: 10 October 2026. Submit the objective below for explicit execution.
Preparing this file does not start the Goal. Required integration IDs: INT-T01
through INT-T07. Production acceptance requires the frozen CONTRACT candidate.

## Objective to submit

Deliver PUZA TTS as an independently usable Python library and a complete
model-backed Auralis module for the declared native Windows scope. Auralis must
accept explicitly reviewed Russian text or subtitle utterances, explicit voice
references and an engine profile, produce real verified speech, show saved
progress, cancel safely, resume after interruption and publish a complete playable
result. Preserve the existing standalone CLI and synthesis algorithms as baselines.

Read E:/Anything/Projects/Commercial/auralis/docs/integration/goals/common.md,
the architecture and boundary contract, TTS and CONTRACT AGENTS, and
CONTRACT/Docs/README.md, AUTHORITY.md, tts/CLI.md, CURRENT_STATE.md, ENVIRONMENT.md,
VOICES.md, BUILD.md and contract/TTS_FOLLOW_UP.md. Inspect the actual render,
segmentation, reference, cache and engine code, local dirty state and Auralis's
current mock engine port. Recheck recorded installation and audio evidence; it
does not automatically qualify the new Auralis consumer.

Freeze both existing engines, VoxCPM2 and ESpeech, with one valid pinned native
profile per engine, their actual model revisions and reference requirements.
Define continuous text and subtitle-utterance generation, supported emotions,
technical split policy, cancellation deadlines and resource budgets. Do not claim
every possible profile or platform. If a required engine cannot pass, retain
that failed gate and do not silently remove it to claim this goal complete.

TTS must consume the frozen shared speech plan or provide a verified explicit
conversion to its existing typed render structures. Close the gap between new
shared-project preflight and legacy render. Never route Auralis input through a
fake Marker project, reparse arbitrary text as executable markup, or discard
utterance, cue, role and internal chunk identities during conversion.

Expose a reusable synthesis API and a thin headless worker. CLI and worker call
the same render core. Add lifecycle inputs, checkpoint observers and structured
outcomes where needed while preserving effective model settings. Keep transport
out of engine logic. Add a typed adapters-tts implementation and host use cases;
do not force production control into an insufficient mock port.

Auralis owns jobs, model admission and artifact publication. TTS owns its assigned
run checkpoints and synthesis cache. Use one owned sequential worker for a proven
compatible batch. Verify cache metadata and bytes before reuse, bind keys to
effective generation inputs and preserve existing group reset behavior until a
comparison validates any optimization. Attempt fencing prevents late publication.

Provide native UI controls for profile selection, reference selection, preflight,
start, saved progress, cancellation, resume, errors and playback. These controls
work with explicit voice manifests without installing Caster or running Marker.
Translate is also optional: an explicitly reviewed Russian input is sufficient.
Final video timing and mux belong to the assembly goal; retain timing lineage and
duration conflicts for that consumer rather than claiming raw speech is synchronized.

Deliver pinned runtime and component packages with explicit model acquisition,
verification, recovery, diagnostics and offline operation. Release installation
cannot depend on developer sibling paths, global Python, .venv_gpu or a seeded
checkout cache. Keep reference/model assets and generated audio outside Git.

## Required acceptance

| Gate               | Observable completion                                                                                                                                                                        |
| ------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| INT-T01 core       | Installed library, CLI and worker agree on a frozen speech plan; legacy supported inputs retain their effective synthesis behavior                                                           |
| INT-T02 data       | Invalid schema, reference, profile, model identity, unsupported controls and technical split errors fail before publication; complete cue and chunk lineage is verified                      |
| INT-T03 control    | Sequential admission, saved progress ordering, bounded cancel with full process exit, worker/application death, exact resume and stale attempt rejection pass owned-process and native tests |
| INT-T04 cache      | Unchanged inputs reuse verified audio; changed text/reference/model/profile/seed invalidate affected entries; corruption is rejected and regeneration preserves unaffected checkpoints       |
| INT-T05 real audio | Both pinned engines generate real short and multi-utterance inputs with exact coverage, playable WAVs, measured durations, retained failures and actual listening against declared criteria  |
| INT-T06 desktop    | Real React to Tauri to Rust to worker to managed artifact flow supports synthesis, restart, cancellation, resume and playback without Caster, Translate or Marker services                   |
| INT-T07 delivery   | Clean Windows installation, explicit asset acquisition, interrupted setup recovery and offline real synthesis pass for the same committed candidate                                          |

Predeclare one short case and one representative longer multi-utterance case per
engine, including technical splitting and multiple compatible references. Retain
the unchanged CLI baseline. Declare listening coverage, performance/resource
budgets and stop criteria before rendering. Test admission rejection without
actually launching a second model. No concurrent renders, doctors or loaders.

Add Taskfile wrappers before new probes. Run affected Python tests and docker
compose config after code changes as required by TTS instructions; use a documented
Task wrapper. Docker image and doctor gates apply only when that environment is
available and part of the frozen endpoint. Native Windows evidence is mandatory.
Run affected Auralis Rust, job, frontend and native checks plus documentation gates.

Publish component documentation updates in CONTRACT/Docs/tts and an integration
acceptance record in Auralis. Missing references, real listening or clean-machine
evidence remain open gates. Audit the exact candidate pins and required evidence
before marking full TTS integration complete.
