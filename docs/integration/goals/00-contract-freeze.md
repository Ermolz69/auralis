# INT-C01–INT-C05 contract freeze

Status: frozen for the first native Windows, model-free contract candidate on
10 October 2026. This record fixes the implementation scope before code changes.
The acceptance record must cite a committed candidate and may narrow claims when
evidence is unavailable.

## Baseline and compatibility

- Existing shared-project `project.json` v1, `cast.json` v2, and `voice.json` v2
  retain their current meanings. CONTRACT package `voxscript-contract` 0.1.0 is
  the observed baseline; the new additive package candidate is 0.2.0. Data,
  transport and package versions are independent.
- TTS legacy `source.md` plus cast v1 stays on its existing render path. No
  conversion silently promotes that input to a shared project or speech plan.
- Translate retains its Rust library API and owned llama.cpp process. It does
  not use the Python worker protocol.
- CONTRACT exposes public voice qualification locking and manifest validation
  operations for Caster while keeping current callers working. The new speech
  validator does not import a model or schedule host work.

## First contract set

- `speech-plan` v1 represents only an explicitly reviewed Russian subtitle
  result or explicitly reviewed Russian plain text. The review mode is
  `reviewed_subtitle` or `reviewed_plain_text`, with a nonempty decision ID and
  immutable source hash. Neither mode claims Marker production.
- `voice-snapshot` v1 records a `voice.json` v2 identity, manifest digest,
  selected reference IDs, audio and transcript digests, engine and emotion IDs,
  and qualification status. A usable reference must be `qualified`; physical
  bytes are checked separately before inference.
- `segment-result` v1 and `synthesis-result` v1 bind exact plan, run, attempt,
  utterance, chunk, profile and audio identities. Complete success requires
  exactly one result for every expected utterance and no extras. Segment audio
  hashes and measured frame metadata cannot be inferred from text.
- Speech timing modes are `continuous` and `subtitle_window`. A subtitle
  window retains source start/end milliseconds. Raw synthesis may exceed that
  window; fit and overlap decisions remain host concerns and conflicts are
  explicit. No clipping or acceleration is implied.
- Identifiers use CONTRACT's existing ASCII ID grammar. SHA-256 strings are
  lowercase hex. Canonical JSON is UTF-8 with duplicate keys rejected. Plan
  digests cover the canonical plan with the digest field omitted.

## Worker v1 and resources

- UTF-8 JSON Lines over stdin/stdout, one object per line; stderr carries only
  diagnostics. Protocol version is integer 1. Each request has request,
  project, run and attempt IDs, operation and typed payload. Each event echoes
  those IDs with a strictly increasing sequence and an event kind.
- Capabilities negotiate exact supported protocol and schema versions before
  work. Required operations are `capabilities`, `validate`, `execute`, `cancel`.
  A validator-only installed worker advertises no synthesis capability and
  rejects `execute` without claiming a model result.
- Maximum frame is 65,536 UTF-8 bytes including newline, maximum queued
  requests 32, and diagnostic retention 65,536 bytes per process. A missing
  newline at EOF is an invalid frame. Invalid UTF-8, duplicate JSON keys,
  malformed shape, oversized frames and stale attempt events fail closed.
- Configured defaults: capability deadline 5 s, validation deadline 10 s,
  synthetic execution deadline 30 s, cooperative cancel 2 s, forced process
  exit 5 s. Real synthesis deadlines must be separately configured and tested
  in its own goal. No unbounded waits or implicit retry.
- `run_id + attempt_id + request_id` identifies a request. Replayed identical
  requests return the recorded outcome; a changed payload with the same key is
  a conflict. Terminal completion is unknown until the host verifies the result
  manifest and files. On transport interruption, reconcile before retry. At
  most two retries follow the first attempt for a classified transient failure.
- Auralis's durable job manager remains the lifecycle authority. One
  machine-wide model admission uses the existing
  `Global\\PUZA_TTS_NATIVE_RENDER` mutex and `%LOCALAPPDATA%/PUZA/model-server.lock`
  before a model process can start; an occupied or uninspectable external model
  state blocks admission. Release follows owned-process-tree exit. Lightweight
  validation never acquires model admission.

## Scoped records

| ID      | Scope and acceptance                                                                                           | Status |
| ------- | -------------------------------------------------------------------------------------------------------------- | ------ |
| INT-C01 | Canonical schemas, semantic checks and positive/negative fixtures for plan, voice, segment and complete result | frozen |
| INT-C02 | CLI, installed package and Auralis adapter parity plus retained existing consumer behavior                     | frozen |
| INT-C03 | Executable worker v1 framing, negotiation, duplicate/stale/EOF/malformed/unknown-completion evidence           | frozen |
| INT-C04 | Synthetic process-tree cancellation, admission, fencing and recovery through host lifecycle                    | frozen |
| INT-C05 | Reproducible pinned package/schema candidate, commands, hashes and compatibility matrix                        | frozen |

All probes use synthetic files and processes. TTS, Marker, model doctors,
model servers and real inference are outside this goal.
