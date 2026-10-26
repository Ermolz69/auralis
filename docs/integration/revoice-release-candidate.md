# Subtitle revoicing technical candidate

Status: **blocked**. The technical route has not passed for either engine.
This record coordinates one candidate for YouTube Chinese subtitles, Russian
translation, exact-version review, speech, WAV, MP4 and ordinary Auralis playback.
The [release matrix](revoice-execution.md#mandatory-acceptance-cases) remains
unchanged; its 24 required cases are not accepted by this narrower exercise.
The `revoice-release-candidate.json` coordination register records dependencies,
file ownership, defects and results.

## Ownership

| Role                    | Files                                                                                              |
| ----------------------- | -------------------------------------------------------------------------------------------------- |
| Backend Developer       | Rust application, ports, adapters, jobs and desktop native composition; required component fixes   |
| UI Developer            | `apps/desktop`, `tools/ui-e2e`, `taskfiles/ui.yml`, `docs/integration/ui-automation.md`            |
| Integration Coordinator | This document and register; integration branch, commit review, frozen cases and execution evidence |

Shared contracts and files crossing those boundaries require agreement before
editing. Integrate reviewed commits one at a time on
`feat/revoice-release-candidate`. The baseline is
`defc0b0b13b2352fc5c32d8c3ae8d54e942adf64`, with Translate pinned to
`44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`.
An executable identity is established only after final integration, affected
model-free checks and `task ui:build`; older candidate hashes do not identify it.

## Prospective source and policy

The source is [Chinese manhua HRaa8kb1EhE](https://www.youtube.com/watch?v=HRaa8kb1EhE).
Its retained manual `zh` SRT has 4,004 cues and 236,229 bytes, SHA-256
`868dc24073183cfd9e5f41ded3eb0cca34b6faad53e008a8b50ddb6f79c49735`.
The recorded strict parse ends at 4,435,266 ms; independently verify acquired
video duration rather than treating the last subtitle end as video duration.
Source metadata previously reported 4,435 seconds.

The diagnostic and full-source cases must have distinct unused IDs. Define the
diagnostic extent prospectively. The full case must retain all 4,004 source cues
once, in order, through translation, saved approval, speech plan, measured speech
layout and ready publication. Compare each stage to the frozen source as well
as to adjacent stages. A shortened diagnostic never passes the full case.

Use the [video policy](revoice-video-export.md): replace original audio with
Russian speech, pad shorter speech with silence and reject speech exceeding the
source duration. Preserve source media and raw synthesis. Never silently trim
speech, change speed or remove cues to make the case pass.

## Pre-execution blockers

The independent runner audit found missing source-relative coverage checks,
single-engine restart playback, incomplete process-tree release verification,
incomplete build inputs, absent resource measurements, incomplete hard deadlines
and an all-silent WAV acceptance gap. Backend review also identified incomplete
matching of saved review to canonical speech-plan utterances. Owners must resolve
these defects and provide regression evidence before final case freeze.

Freeze executable, all build inputs and code commits, runtime and wheel hashes,
models, profiles/settings, voices and unchanged reference hashes. Freeze stage
time limits, RAM/VRAM/disk limits, source identity, full coverage and stop rules.
Resource availability is a measured prerequisite. It is not execution evidence.

Each case is consumed once. Retain every attempt, command exit code, stage
duration, screenshot/trace, measured resource samples, output hash and cleanup
decision. No automatic whole-case retry is permitted. Rebuilds or changed inputs
need a new identity and affected mandatory checks. A blocked, failed or unrun
required stage cannot produce a successful case result.

`task revoice:release:check` validates the register and status controls.
`task revoice:release:status` reports the technical result and returns nonzero
while required stages are blocked, failed or unrun. This command does not accept
the broader release matrix.

## Verification checkpoint

On the baseline, `task ui:check:runner` passed all 13 checks (exit 0, 4.87 s).
The preceding sandbox attempt failed to spawn test subprocesses (Task exit 201);
it did not execute the assertions. Both outcomes remain distinct.
This checkpoint is not evidence for the changed integration candidate.

VoxCPM2, ESpeech, actual translation, real WAV/MP4 and native playback are
**not_run**. The overall result remains **blocked**.
Automated exact-version approval proves a workflow boundary only. Translation
quality, actual listening, voice qualification, TXT/ASR, clean Windows and the
broader release matrix require their own factual assessments.
