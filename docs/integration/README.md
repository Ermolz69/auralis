# Auralis module integration plan

The owner's active 23 October 2026 full story-revoicing Goal is tracked in
[the execution record](revoice-execution.md), with a finite acceptance matrix
and an [extended TXT/ASR contract](revoice-input-contract.md). That explicit
scope supersedes the older prompts' TXT/ASR exclusions for this Goal only.

Updated: 10 October 2026. The requested delivery connects Translate, TTS and
Caster to Auralis through reusable libraries and explicit adapters. CONTRACT
supplies shared data validation. Marker is a later stage; the existing Caster
WPF application stays unchanged.

These are architecture and execution prompts. Creating them does not activate
a Goal, start model processes, change module code or qualify a release.

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
The active [TTS execution record](tts-execution.md) tracks implementation
evidence and open acceptance gates.
The [ordinary UI automation](ui-automation.md) commands retain separate fixture,
native smoke and real component outcomes with isolated profiles.
The [reviewed video policy](revoice-video-export.md) extends managed assembly to
separate MP4 output without modifying the original video or voice catalog.
Component documentation continues to live in CONTRACT/Docs for the PUZA modules;
these files specify their integration with Auralis rather than replacing those
component manuals.

Run task docs:integration from the Auralis root to verify links, Markdown and
formatting for this document set. This command does not run component code or
load models.
