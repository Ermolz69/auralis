# Auralis module integration plan

Updated: 10 October 2026. The requested delivery connects Translate, TTS and
Caster to Auralis through reusable libraries and explicit adapters. CONTRACT
supplies shared data validation. Marker is a later stage; the existing Caster
WPF application stays unchanged.

These are architecture and execution prompts. Creating them does not activate
a Goal, start model processes, change module code or qualify a release.

## Current host boundary

| Component | Composed in this Auralis checkout                                                                                                                           | Remaining gate                                                                                                                                                                       |
| --------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Translate | Experimental subtitle import, checked-model jobs, saved progress, review and separate result publication; see [Translate status](../translation/README.md). | Source-grounded language quality, clean installation, and remaining concurrency evidence.                                                                                            |
| Caster    | Installed-worker voice/reference catalog, compatibility inspection and explicit manual cast revisions; see the [Caster record](caster-execution.md).        | The dated record reports synthetic native and package checks; genuine reference qualification and frozen CONTRACT acceptance remain open. No automatic casting or speech generation. |
| TTS       | Installed-worker voice/profile catalog and selector; model-free execution transport and persistence pieces exist; see the [TTS record](tts-execution.md).   | Desktop synthesis, complete publication, speech-plan model/listening checks, and clean installation.                                                                                 |
| Assembly  | [Design and goal](goals/04-assembly.md) only.                                                                                                               | Accepted component candidates, real playable speech, timing, media export and clean-install verification.                                                                            |

The [Rust adapter map](../architecture/003-rust-workspace.md) identifies the
current composition boundary. The [durable queue](../architecture/008-job-dispatch.md)
currently supports subtitle import; [outbox concurrency](../architecture/009-outbox-concurrency.md)
governs managed publication. Current code and the named execution records are
the evidence for capability claims. Design arrows and goal prompts do not
demonstrate native, model or release acceptance.

## Architecture and execution order

Read the [integration architecture](architecture.md) and
[boundary contract](boundary-contract.md). Each module can be developed and
verified without running the other modules. Production composition still needs
compatible, pinned contracts and the final assembly gate.

| Goal                                                 | Purpose                                                                       | Production prerequisite                               |
| ---------------------------------------------------- | ----------------------------------------------------------------------------- | ----------------------------------------------------- |
| [CONTRACT and host boundaries](goals/00-contract.md) | Freeze shared schemas, compatibility and process ownership                    | Existing CONTRACT and Auralis storage contracts       |
| [Translate readiness](goals/01-translate.md)         | Finish the existing library integration and selected translation release      | Current Translate backlog and real quality evidence   |
| [TTS integration](goals/02-tts.md)                   | Deliver real, cancellable synthesis inside Auralis                            | Frozen shared speech and worker contracts             |
| [Caster integration](goals/03-caster.md)             | Deliver voice library and manual casting operations inside Auralis            | Frozen voice and casting contracts                    |
| [Complete assembly](goals/04-assembly.md)            | Verify translation, voice assignment, speech timing and media export together | Accepted candidates from the required component goals |

Start with CONTRACT. Translate can progress as a separate branch of work. TTS
accepts explicit reviewed text and voice manifests, so it does not wait for
Translate or Caster implementation. Caster accepts explicit roles and prepared
references, so it does not wait for translation or synthesis. Complete assembly
runs after their required acceptance records exist. All model work is sequential.

Each goal file contains a complete objective to submit in a later execution
request and refers to the [shared execution rules](goals/common.md). Submit one
selected objective; do not launch all goals merely by opening this directory.

Documentation and prompts are in English to follow repository conventions.
The [Caster execution record](caster-execution.md) and
[TTS execution record](tts-execution.md) track implementation evidence and open
acceptance gates.
Component documentation continues to live in CONTRACT/Docs for the PUZA modules;
these files specify their integration with Auralis rather than replacing those
component manuals.

Run task docs:integration from the Auralis root to verify links, Markdown and
formatting for this document set. This command does not run component code or
load models.
