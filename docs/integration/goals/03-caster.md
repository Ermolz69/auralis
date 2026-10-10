# Goal for Caster library integration in Auralis

Updated: 10 October 2026. Submit the objective below for explicit execution.
Preparing this file does not start the Goal. Required integration IDs: INT-V01
through INT-V05. Production acceptance requires the frozen CONTRACT candidate.

## Objective to submit

Deliver Caster's voice catalog and manual role assignment as an independently
usable library with an Auralis adapter and native UI. Reuse existing voice and
CONTRACT operations, preserve immutable source references and qualification
rules, and keep the existing WPF application completely unchanged. The mandatory
catalog and casting workflow must work without TTS, Translate or Marker running.

Read E:/Anything/Projects/Commercial/auralis/docs/integration/goals/common.md,
the architecture and boundary contract, Caster and CONTRACT AGENTS, and
CONTRACT/Docs/README.md, AUTHORITY.md, caster/CLI.md, VOICE_EDITOR.md, tts/VOICES.md
and contract/PLAN.md. Inspect caster.ps1, the Python library, qualification guards,
public entry points and voice schemas. Inspect WPF only to identify APIs that
must remain compatible; do not edit its source, assets, configuration or build.

Freeze the manual catalog and casting endpoint: list and inspect voices, inspect
prepared references, import or create versioned references through supported
existing operations, validate engine/emotion compatibility, select a voice for
an explicit role and publish an immutable assignment revision. Review roles are
provided by Auralis or explicit input; this goal does not infer speakers from
subtitles and does not implement Marker.

Provide a narrow stable Python facade over the existing reusable operations.
Keep current import paths and behavior compatible. Use CONTRACT's canonical
validation and publication rather than copying its rules. Add a model-free
headless worker, typed adapters-caster and project-scoped Auralis use cases.
Catalog operations must not import TTS, load models or require Marker packages.
Avoid moving WPF dependencies or changing its callers to obtain a new facade.

Add paginated and bounded native catalog views, reference metadata, explicit
engine compatibility and manual assignments. Preserve prior assignments on a
failed or stale write. Snapshot exact manifest/reference hashes in each cast
revision so existing runs retain reproducible voices when the catalog changes.
Never mark a voice qualified from an AI guess, successful generation or missing
errors; retain CONTRACT's human review evidence and pending-journal behavior.

Prepared audio can be inspected and played without synthesis. If the accepted
TTS module is installed later, a preview request routes through Auralis's TTS
port and shared model queue, with no second engine or private Caster scheduler.
The catalog remains usable when TTS is unavailable. Cross-module generated
preview is verified by the assembly goal, not required to fake Caster independence.

The current autocast wrapper invokes text_marker.cli. Preserve its existing
standalone behavior, but do not make it a hidden dependency of this integration.
LLM autocasting and automatic speaker discovery are explicit later scopes;
document this exclusion and retain the role/proposal extension boundary. Do not
claim the manual casting endpoint implements automatic casting.

Package the facade with a pinned lightweight runtime and CONTRACT dependency.
Installed Auralis must not require the sibling Marker venv, hardcoded voice paths
or the WPF editor. Keep component documentation in CONTRACT/Docs/caster.

## Required acceptance

| Gate              | Observable completion                                                                                                                                                   |
| ----------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| INT-V01 library   | Installed facade and standalone entry point perform catalog and manual assignment operations without Tauri, WPF, TTS or Marker dependencies                             |
| INT-V02 integrity | Source audio remains immutable; reference and manifest hashes, unsupported data, qualification state, journals and compatible engine/emotion rules are checked          |
| INT-V03 mutation  | Concurrent revisions, stale proposals, interrupted publication and replay preserve the previous cast and recover one complete revision                                  |
| INT-V04 desktop   | Native catalog, inspection, prepared-audio playback and explicit role assignment pass through typed IPC into a complete managed cast artifact                           |
| INT-V05 delivery  | Pinned packages install outside developer checkouts, accepted data survives restart, WPF tracked files are unchanged and recorded existing entry-point tests still pass |

Use isolated voice-store copies and legally usable model-free WAV fixtures for
engineering tests. Qualification acceptance uses genuine existing human-reviewed
reports; absence of one must not be hidden by test data. Add Taskfile commands
before new probes and run affected component, Rust, frontend, native and docs
checks. No model server, synthesis or WPF launch is needed for mandatory gates.

Audit one committed candidate, pins and acceptance evidence before completion.
Report manual casting readiness separately from deferred automatic casting and
later generated previews. Existing user voice stores are not test workspaces.
