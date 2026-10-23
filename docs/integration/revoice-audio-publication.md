# Managed speech layout publication, 23 October 2026

This INT-R08/INT-R09 slice connects the verified chunk bridge to the existing
speech workflow. Publish its layout alongside raw WAV, manifest and technical
chunks through the same short SQLite transaction and finalization outbox. The
layout is metadata (`SpeechManifest` artifact kind), identified in a speech
publication as `audio_layout`. There is at most one layout per raw result. All
files must become ready before the publication or job completes. Existing
publications without a layout remain readable and playable; later assembly must
ask for the verified streaming package and a new render when no layout exists.

Only the known streaming TTS policy may produce a layout. Its exact saved plan,
raw manifest, cast, text/chunk coverage and aggregate properties must pass the
shared verifier. Hash the exact serialized layout bytes, write a private
temporary file and persist without replacing any earlier file. Cancellation or
verification failure cannot yield a completed execution or ready layout. Add
cooperative cancellation to verification/hash loops so cancellation also applies
after worker exit. Retain the existing non-cancellable verifier API for callers
that do not supply a token.

The raw WAV, raw manifest and chunks keep their existing identities. A stored
layout's attempt path is provenance, not a path to execute after restart. A
subsequent assembly use case must resolve the ready managed WAV through
`ArtifactIndex`/`ArtifactStore`, check its expected hash and the saved plan/
manifest/layout hashes, then rebind the aggregate path to that managed copy.
This slice does not expose arbitrary paths to the UI or mark a rendered assembly
ready directly. Assembly jobs, selection/review of timing, native controls,
video mux and natural-story acceptance remain required follow-up work.

## Implemented behavior and model-free evidence

The TTS execution adapter now creates the verified layout for the known streaming
policy before returning completed files. A private temporary file is synced and
persisted without clobbering an earlier layout. Existing policies return the
legacy file set. Verification uses a child cancellation token and checks it
between hashed blocks/utterances; dropping its owning scope cancels that child
without cancelling the caller's parent token.

Application publication has a focused module and checks cancellation between
imports and immediately before its transaction. `audio_layout` maps to managed
JSON metadata. SQLite accepts zero or one layout, rejects unknown kinds and
wrong artifact types, and waits for every file before readiness. Frontend IPC
validation accepts both legacy publications and one optional layout while
retaining exact chunk coverage and unique artifact checks. WAV playback still
selects the `wav` entry.

`task rs:test:audio:publication` passed 11 layout/result, 19 assembly, 10 TTS
library, 2 publication-storage and 11 application-workflow controls, plus the
existing transport, speech progress and database migration checks. The new
application cases use real SQLite/managed files/outbox with fixture execution;
they prove hidden pending metadata, cancellation/discard and cached resume. A
storage case reopens SQLite with the complete metadata publication still pending
before promoting it to ready.

`task fe:test:audio:publication` passed 2 IPC and 3 workflow UI checks outside
sandbox. Its first sandboxed Vitest process exited with Windows `0xC0000005`
before reporting tests; that failure is retained separately. The unchanged tests
passed outside sandbox. Root cause was not independently established. An earlier
test formatting error was an extra closing brace and was corrected before tests.

Scoped production/test Clippy, normal desktop composition and architectural
file-size checks passed. Frontend typecheck, lint and production build also passed
through `task fe:typecheck`, `task fe:lint` and `task fe:build`.
`task revoice:publication:evidence` records exact source
hashes and retained logs in
`tools/revoice/evidence/AUDIO_LAYOUT_PUBLICATION_V1.json`; logs also live outside
Cargo target in `.cache/revoice-checks/audio-publication-v1/`.

## Verification fixed before implementation

Use Taskfile commands and model-free controls first. Verify atomic layout-file
creation and hashes, refusal to overwrite an earlier file, cancellation without
accepted output, legacy policy compatibility and corrupt known-policy failure.
Verify optional layout publication, duplicate layout refusal, unknown file-kind
refusal, correct metadata artifact type, no partial visibility while a layout is
pending, recovery after reopening SQLite and cancellation/discard through the
actual application/outbox path. Keep existing three-file legacy controls.

Run TTS/bridge, speech storage/application regressions, scoped Clippy, normal
desktop composition, docs/plan and architectural size gates. Tests with synthetic
PCM and mock execution do not accept a real worker, native, listening, natural
story or clean-install gate. Preserve source voices and installed runtime.
