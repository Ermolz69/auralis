# Goal for CONTRACT and Auralis module boundaries

Updated: 10 October 2026. Submit the objective below for explicit execution.
Preparing this file does not start the Goal. Required integration IDs: INT-C01
through INT-C05; freeze their scoped task records before implementation.

## Objective to submit

Complete the shared library and boundary foundation for independently usable
Auralis Translate, TTS and Caster integrations. Reuse the existing
voxscript-contract package and its voice/casting semantics. Deliver canonical
speech-plan and synthesis-result schemas, executable validation, a minimal
versioned worker protocol, pinned packages and host adapter boundaries. Do not
stop at schema drafts: prove an installed model-free producer and consumer path.

Read the shared execution rules in
E:/Anything/Projects/Commercial/auralis/docs/integration/goals/common.md, plus
docs/integration/architecture.md and docs/integration/boundary-contract.md in
Auralis. Resolve those roots if the checkout has moved. Inspect applicable AGENTS,
Auralis application,
ports, jobs, managed storage and Translate process ownership, and
CONTRACT/Docs/README.md, AUTHORITY.md, contract/PLAN.md, MANIFEST_V1.md and
TTS_FOLLOW_UP.md. Inspect dirty state before changing any repository.

Freeze the first native Windows contract set, exact existing voice/cast versions,
new speech-plan version, supported review and timing modes, worker framing,
resource deadlines, idempotency and compatibility policy. Treat the design names
as provisional until that freeze. Preserve existing shared-project and legacy
TTS inputs without silently changing their meaning. Translate keeps its existing
Rust library API; do not route it through the Python worker protocol.

Implement canonical schemas and semantic validation in CONTRACT. Auralis consumes
the pinned schema assets and a lightweight validator through its adapter layer.
Keep shared validators free of model imports and host scheduling. Expose narrow
stable public operations needed by Caster instead of adding new dependencies on
CONTRACT private helpers. Retain compatibility for existing callers.

Create typed Rust speech, voice and execution boundaries at the correct layer.
Define machine-wide admission compatible with existing standalone guards, process
identity and owned process trees. Reuse existing host lifecycle behavior; do not
create a competing scheduler or an admission semaphore for each module. Implement
only concrete common behavior required by consumers, not a general plugin system.

Add Taskfile commands before new conformance, packaging and lifecycle probes.
Build pinned model-free CONTRACT packages and prove the validator operates from
an installed package outside developer checkouts. Use schemas and fixtures that
exercise reviewed subtitle and reviewed plain-text producers without requiring
Marker. Do not claim marker-produced status for those inputs.

## Required acceptance

| Gate                  | Observable completion                                                                                                                                                                   |
| --------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| INT-C01 data          | Canonical versioned speech plan, voice snapshot, segment result and complete result validate valid fixtures and reject wrong versions, identities, references and incomplete coverage   |
| INT-C02 compatibility | CLI, installed Python package and Auralis adapter agree on accepted and rejected fixtures; existing supported CONTRACT and legacy consumers retain their recorded behavior              |
| INT-C03 transport     | Version negotiation, bounded UTF-8 frames, stderr isolation, stale event rejection, EOF, malformed messages, duplicate requests and unknown completion have executable process evidence |
| INT-C04 ownership     | Synthetic owned processes prove cancel and exit ordering, attempt fencing, duplicate admission rejection and recovery without loading any model or altering user data                   |
| INT-C05 delivery      | Package and schema hashes, lockfiles, producer/consumer commands, results and compatibility matrix are recorded for one reproducible candidate                                          |

Run Auralis task check:docs and task check:quality:docs plus affected Rust/job
checks selected from its Taskfile. Run CONTRACT's authoritative model-free
commands; if no Taskfile wrapper exists, add a documented task before the probe.
Do not launch TTS, Marker, model doctors or model servers for this goal.

Produce an acceptance record and exact pins that the other goals can consume.
Every required gate must pass before completion. Leave translation language
quality, real synthesis, listening and final video timing to their named goals;
passing this goal does not qualify those capabilities.
