# Subtitle revoicing technical candidate

Status: **failed**. A mandatory integration regression is failing; the real
technical route has not passed for either engine.
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
`feat/revoice-release-integration`. The baseline is
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

The owned runtime snapshot contains 46,487 files. Full byte verification passed
in 56.895 seconds; the relocated manifest SHA-256 is
`b91323fa8a9b6e5a86015badbc96f69ccd8cd878e15dae55c51d4d888dba580c`.
Installed CONTRACT metadata validated four unchanged neutral-reference plans.
`task media:verify` passed for the retained tools. These checks loaded no engines
and establish neither model execution nor native workflow acceptance.

After integration through `1129456587f83d25e4407c4a1de5f68ddffec684`, the
following checks ran on that source:

| Task                      | Result                                 | Scope                                                                           |
| ------------------------- | -------------------------------------- | ------------------------------------------------------------------------------- |
| `rs:test:speech-workflow` | passed, exit 0                         | 15 checks, including the host 4,004-cue fixture                                 |
| `rs:test:contract-bridge` | passed, exit 0                         | Fixture API, 4,005 bindings and reference refusal controls                      |
| `rs:test:tts`             | passed, exit 0                         | 51 checks; four explicit installed/real checks remained ignored                 |
| `ui:check:video`          | passed, exit 0                         | 195 policy, lifecycle, publication and storage checks                           |
| `rs:test:translate`       | failed, Task exit 201 / Cargo exit 101 | Two native-admission fixtures failed; their exact refusal still needs diagnosis |

The sequence stopped on that failure; subsequent contract-adapter and lint
commands were not run by this integration attempt. Earlier owner-side passes do
not replace this result. The absolute 45-minute TTS plan deadline also needs an
explicit bounded long-case configuration before freezing the complete-source case.
No source coverage or duration policy may be reduced to pass these gates.

Owner followups correct request validation before native admission and add a
bounded per-plan TTS budget, including dispatch and checkpoint waits. Independent
review confirmed the worker termination correction in `98b84c7`: termination is
requested without waiting, then one exit confirmation is bounded by five seconds.
Adjacent component and Translate cleanup paths require the same protection.
These changes still require integration checks on the final source.

The model-free DSP oracle passed a distinct second case after an earlier harness
lookup failure; both attempts remain in the register. That oracle did not exercise
the real installed verifier wrapper. Its helper lookup incorrectly omitted the
wheel's `src/` prefix (RC-D28). The wrapper must be corrected and tested before
any expensive speech case is admitted.

The corrected wrapper subsequently resolved the installed helper but its first
installed run failed on an omitted numerical dependency (`typing_extensions`).
That failed case and confirmed cleanup remain separate from the earlier oracle
pass. The dependency closure must be hash-pinned and tested through the wrapper.

Windows descendant confirmation also remains blocked (RC-D29). In the pinned
process wrapper, a cancelled wait can preserve a cached root exit status before
descendants finish, and a completion-port notification alone does not prove an
empty job. Cleanup needs independent all-process confirmation under its existing
deadline; otherwise admission and owned locks must remain retained. This is a
code-review finding, not a reproduced claim that a model process leaked.

The subsequent model-free regression reproduced false cleanup confirmation on
the unchanged Windows wrapper in both component and synthesis paths. The
fixtures independently confirmed their inert descendant processes were gone
before completing the failing assertions. A query-based local correction of
the same dependency version is pending; model execution remains prohibited.

Integrated UI checks on `000beac472a12a91b3d59cff81bf3fc0594bbff5` passed:
runner controls, frontend types and components, both production-browser fixture
routes, frontend regressions, 35 existing browser cases, lint, architecture and
integration-document checks. The fixture retained 32 full-workflow and 18
diagnostic steps. These simulated results do not establish native media or model
acceptance. After the numerical dependency followup, runner controls passed
33 checks on `bcc057318c0972a2cf994dedbb43f5399038b3d5`.

VoxCPM2, ESpeech, actual translation, real WAV/MP4 and native playback are
**not_run**. The overall result remains **failed**.
Automated exact-version approval proves a workflow boundary only. Translation
quality, actual listening, voice qualification, TXT/ASR, clean Windows and the
broader release matrix require their own factual assessments.
