# Auralis module boundary contract

Updated: 10 October 2026. Status: design requirements to freeze in the CONTRACT
goal. The names below describe proposed v1 interfaces; they are not claims that
these schemas or worker operations already exist.

## Existing schemas and new speech input

Reuse CONTRACT's canonical voice manifest v2, cast v2, validation reports and
reference qualification rules. Record the accepted legacy formats separately;
never silently reinterpret a legacy voice manifest as qualified v2 data.
Unknown versions, unresolved roles, changed reference hashes and unsupported
engine or emotion combinations fail before inference.

The existing shared-project format and TTS legacy render format remain supported
on their existing paths. Introduce a separate speech plan for Auralis's reviewed
subtitles or explicitly reviewed Russian text. Do not invent a marked or
renderable Marker project just to route an unmarked input through its validator.
If a producer conversion is necessary, version it and test identity preservation.

The canonical schema and executable semantic validator belong in
CONTRACT/VOXSCRIPT_CONTRACT. Rust adapters validate wire shape and host ownership;
they reuse the pinned canonical validator for shared semantics. Conformance
fixtures prove both acceptance and rejection behavior. Do not independently
reimplement VoxScript or voice compatibility rules in Rust or TypeScript.

| Data              | Required identity and meaning                                                                                                                     |
| ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------- |
| Speech plan       | Schema version, plan ID, project ID, source hash, text revision, review decision, plan digest, ordered utterances and casting snapshot            |
| Utterance         | Unique ID, source cue or text range, reviewed text, role ID, optional start and end times, effective speech controls and engine profile reference |
| Voice snapshot    | Voice ID, manifest hash, reference IDs, reference text and audio hashes, engine and emotion qualification                                         |
| Synthesis request | Plan digest, run ID, attempt ID, idempotency key, pinned runtime, model and profile identities, declared resource policy                          |
| Segment result    | Utterance ID, internal chunk mapping, content hash, sample rate, channels, frame count, measured duration and effective generation settings       |
| Complete result   | Exact expected utterance set, synthesis manifest digest, verified segment references, warnings, terminal outcome and reproducible lineage         |

Store review provenance without exposing private reviewer details in general logs.
Technical splits cannot change the visible utterance identity, source text or cue
order. Rendering expressions, model instructions and VoxScript-looking text must
remain input data; typed input is preferred over concatenating it into markup.

Timing is a host concern. Continuous text and subtitle-window speech are distinct
modes. Timing fields preserve the source windows even when raw speech is longer.
The host declares overlap handling and maximum permitted fit transformations
before tests. Unsupported overlaps or impossible fit remain explicit conflicts.
No successful speech result alone proves synchronized dubbing.

## Worker transport

Use UTF-8 JSON Lines on stdin and stdout. Reserve stdout for framed protocol
messages and stderr for bounded diagnostic logs. Invoke a verified executable
with an argument vector and explicit working directory, not shell-built commands.
Do not send WAV payloads or full subtitle histories through Tauri IPC.

The minimum operations are capabilities, validate, execute and cancel. Recovery
and result inspection can be separate operations where component checkpoints
require them. Capabilities report supported schema/protocol versions and actual
installed profiles without importing or loading models. Negotiation rejects
incompatible required versions before work begins.

Requests carry protocol version, request ID, project ID, run ID, attempt ID,
operation and typed payload. Responses carry the same identities, event sequence,
event kind and typed payload. Retain structured codes for validation, identity,
resource, runtime, storage, cancellation and unknown-completion failures. Human
messages explain the failure but do not determine retries or state transitions.

Define finite frame sizes, queue limits, operation deadlines, cooperative-stop
deadlines and forced-exit deadlines in validated configuration. Reject malformed,
oversized, unexpected or stale messages. Handle EOF, partial frames and worker
exit independently of a successful terminal response. A success message is not
evidence of complete output until manifest and file validation passes.

A model-owning worker may live for a compatible batch under one resource lease.
It must not keep a model loaded when another engine needs the lease. A lightweight
validator or catalog worker does not reserve model resources unless an operation
explicitly requests synthesis.

## Ownership and state transitions

Auralis owns project jobs and managed artifact publication. Component libraries
own their internal checkpoints and results in their assigned workspaces. Workers
receive narrow input and output roots; shared data schemas contain portable keys
and content hashes. Adapters resolve local paths and reject traversal, symlinks or
an output outside the assigned workspace according to the existing storage policy.

Worker progress is provisional. Auralis emits saved progress only after the
required component persistence and host state update have committed. Duplicate
events and repeated execute requests cannot create a second logical result.
Changed source, plan, cast, model or profile identities cannot resume an older run.

Cancellation fences the active attempt, stops the owned task and waits for process
exit before recording terminal cancellation. A stale attempt cannot commit or
publish after cancellation or replacement. A worker crash preserves accepted
checkpoints and records an interrupted attempt; it never implies successful
completion. Recovery validates hashes and reconciles component and host commits.

Global model admission covers standalone tools, Auralis and external model
servers. Do not merely create one semaphore per module. Coordinate with existing
CONTRACT and Caster guards, process identity checks and host process ownership.
An uncertain or occupied machine admission prevents loading another model.

## Compatibility and error policy

Keep data schema versions, transport versions and package versions distinct.
Pin exact package, wheel, schema, runtime, model and profile hashes in the tested
candidate manifest. A supported version range is established by conformance
tests, not by assuming all matching major versions behave identically.
JSON Schema dialect is declared explicitly through the
[schema keyword](https://json-schema.org/understanding-json-schema/reference/schema).

Validation, identity mismatch, unsupported configuration and invalid model output
are not blindly retried. Transient transport or storage retries use an explicit
finite policy and the same idempotency identity. For initial integration probes,
allow at most two retries after the first attempt for a classified transient
failure; freeze the effective policy before running. Unknown completion is
reconciled before retry. Keep every failed attempt and its diagnostic evidence.

Cache hits require verified content and complete compatible metadata. Invalidation
depends on effective synthesis inputs rather than UI labels. The standalone CLI,
worker and Auralis adapter must agree on these identities and error categories.
