# Auralis Translate integration status

This is the current guide for experimental translation of an existing subtitle
file into a separate Russian SRT or strict plain-WebVTT file. Auralis pins
Translate commit `44e7d4f3c6ce2443279f5d0d3acd07cbb931476e` in
`modules/auralis-translate`; the independent sibling checkout is a different
candidate. The [dated integration history](history.md) preserves earlier
implementation notes, failures and named acceptance records. Later status here
is based on Auralis code at base `43136f3`, its pinned library, and those records.

The desktop has import, run, comparison, editing, history, explicit selection,
review and export controls. These are current code paths, not a release or
language-quality qualification. Auralis owns projects, host jobs, links, managed
originals and outputs, publication and outbox state in `auralis.sqlite`.
Translate owns detailed runs, committed checkpoints, edits and immutable results
in `auralis-translate.sqlite`. The external source and managed original remain
unchanged. The [storage contract](../storage/production-storage.md) governs
publication across the two databases.

## Managed runtime setup for the current development build

The subtitle workspace offers an explicit **Download and install** action for
the pinned experimental Windows x64 CPU package, then **Select model** and
**Check and repair**. Downloads, installation and model startup occur only when
the relevant user action or run is requested. The package, selected runtime and
GGUF are verified before admission; weights are not part of the Auralis bundle.
The [historical setup record](history.md#managed-runtime-setup-for-the-current-development-build)
contains the manual configuration example and the package/native probe identities.

At startup, `src-tauri/src/bootstrap/mod.rs` uses a valid selected package first.
Only when no package is selected does it load the optional application-data
`translation-runtime.json` development fallback. An invalid selected package
leaves the runtime unavailable for attention; it does not silently fall back to
the manual file. Selection or repair waits for admission and refuses active
translation jobs. A missing runtime does not prevent the rest of Auralis from
starting. The selected package is a local development integration; signed,
unseeded clean-machine delivery, upgrades and removal remain open.

For the manual development path, place `translation-runtime.json` beside
`auralis.sqlite` in Auralis application data. Supply already installed, absolute
paths; this example does not install or download anything:

```json
{
  "executable": "C:/path/to/llama-server.exe",
  "model_file": "C:/path/to/Hy-MT2-1.8B-Q4_K_M.gguf",
  "context_tokens": 2048,
  "gpu_layers": 99,
  "ram_cache_mib": 0,
  "startup_timeout_seconds": 120,
  "readiness_interval_milliseconds": 250
}
```

`gpu_layers: 0` requests CPU use; omitted `ram_cache_mib` defaults to zero.
The runtime checks the frozen profile and installed model again at admission.

## Implemented boundary

1. **Inspect and import.** The project subtitle workspace accepts an existing
   file only in the declared strict SRT or WebVTT subset. The inspector rejects
   unsupported syntax before inference and reports source hash and text slots.
   Import creates a separate managed `OriginalSubtitle` copy through staging and
   outbox finalization; neither external nor managed original is edited.
2. **Prepare and start.** A ready project-owned original is required to register
   a Chinese-to-Russian run with a frozen checked experimental profile. Auralis
   commits the project link and run intent, then idempotently registers the run
   in Translate SQLite. **Start** or **Continue** admits a checked runtime and a
   linked host job. Model preparation can precede the command response.
3. **Pause, resume and saved progress.** Pause is durable during admission,
   preparation or an active request. Acknowledgment waits for owned cleanup;
   unfinished inference output is discarded. A later explicit Continue uses the
   same run and skips only committed checkpoints. Auralis mirrors committed
   block counts into its host job; Translate SQLite remains authoritative.
   No partial result is published.
4. **Publish.** Translate validates and commits an immutable result. Auralis
   reconstructs a separate file from original bytes and verified text, stages
   it, and atomically records a pending artifact, publication and outbox intent.
   The result becomes selected only after the managed artifact is ready. The
   earlier ready result remains available while a new result finalizes.

`needs_review` describes structural validation, not a human decision or speech
handoff. The registered Tauri commands are in `src-tauri/src/lib.rs`; the
desktop source import, model setup, run control and review features live under
`apps/desktop/src/features/translation-*`.

## Committed translation events

The job-event bridge emits after successful host-job writes. The panel uses
project-scoped events, publication notifications and global invalidation to
reread authoritative links and run status. A three-second recovery poll covers
lost events or unavailable listeners; events do not themselves prove a
checkpoint. The [event record](history.md#committed-translation-events)
includes SQLite, observer and one native checked-model UI scenario. It supersedes
the earlier pending event statements in the history; broader diagnostic and
interleaving coverage remains open.

## Ready result history and explicit selection

The desktop lists ready publication/artifact pairs for the immutable source.
Previewing a historical result loads verified comparison pages without changing
the attached project result. The UI shows 25 paired segments per page; the
backend bounds requests to 1–100. **Attach** is separate and rechecks the
original, frozen run, immutable result and managed output digest, then guards
the host link revision and pending publication. A different pending publication
can block selection rather than be stranded.

An ordinary edit creates a new immutable `needs_review` revision from the latest
base. Editing an older preview uses a distinct observed-head and host-link guard,
a durable host intent and an immutable branch. The new branch may become ready
history without changing a newer project choice; its notice distinguishes ready
history from attachment. The [history-selection record](history.md#ready-result-history-and-explicit-selection)
and [historical edit records](history.md#historical-application-save-and-recovery)
include supporting two-database tests and named native checked-model scenarios.

The review panel obtains the chosen ready result and its SHA-256 before showing
an explicit **Approve** or **Withdraw** action. Auralis stores append-only,
digest-bound decisions per immutable result. A new edit does not inherit approval;
selecting another version does not change its decision. A `needs_review` label,
ready publication or exported draft is never implicit approval. The
[review contract](../architecture/008-translation-review-decision.md) requires
a future speech consumer to verify the selected result and latest approval again.

**Export** accepts an explicitly chosen ready result, including historical
versions, and re-verifies its source, run, bytes and managed artifact across both
databases. It writes a separate SRT or strict WebVTT file at the user-selected
absolute destination outside application data. It does not overwrite a different
existing file; an uncertain retry succeeds only if the existing bytes match.
Export does not attach or approve a result. The registered command, application
tests and UI control exist at this base; no AUR-M02 native export run or
clean-machine claim is made here.

## Project deletion

Deleting a project commits a Translate cleanup outbox intent with the Auralis
deletion. Translate cleanup removes its owned records and blocks late
registration; retryable failures remain visible. The external original is not
deleted. The [deletion record](history.md#project-deletion) includes a named
native running-attempt scenario, while other deletion interleavings stay open.

## Startup publication recovery

Startup reconciles frozen run registration and host jobs, then scans active or
selected links for committed Translate results missing a host publication.
Recovery stages only a verified, unrecorded result; existing pending or ready
publications remain with the outbox. It does not rerun inference or overwrite a
newer explicit choice. The [recovery record](history.md#startup-publication-recovery)
separates two-database tests, one native result-gap scenario and open failure
injection boundaries.

## Checks

| Capability                                          | Existing evidence and limit                                                                                                                                                                                                                     |
| --------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| SRT/WebVTT import, checked execution and ready copy | Two-database and named native SRT/WebVTT scenarios in the [history](history.md); synthetic sources do not establish bilingual quality.                                                                                                          |
| Package selection and repair                        | Installed-package, restart and one-cue native CPU records in [setup](history.md#managed-runtime-setup-for-the-current-development-build) and [package checks](history.md#pinned-package-repair-and-delivery-checks); no unseeded clean install. |
| Pause and committed progress                        | Named SRT/WebVTT pause, preparation, initial-hash and worker-preflight checks in the [history](history.md); the event slice has its own native UI check.                                                                                        |
| Concurrent starts                                   | Two named Start boundaries passed in the [start-race record](history.md#native-concurrent-starts-during-preparation); concurrent Pause/deletion/publication and actual Windows cleanup failure remain open.                                     |
| History and older-base edits                        | Named selection/reopen and committed-branch result-gap checks in the [history](history.md); journal-only and staged publication interruption remain separate.                                                                                   |
| Review and export                                   | Code, UI and supporting application tests plus the [review contract](../architecture/008-translation-review-decision.md); AUR-M02 did not run native/model/export acceptance.                                                                   |

For model-free repository checks use `task rs:test:translate` and the focused
application/frontend tasks in the [Taskfile guide](../taskfile/001-commands.md).
Native tasks under `task desktop:e2e:native:translation...` require explicitly
supplied checked assets and isolated data; they were not run for this docs edit.
Mock, compilation, one-cue native and package-content checks do not establish
source-grounded Chinese/Russian quality, full concurrency, clean installation
or a production dubbing release. The pinned Translate
`docs/RELEASE_ACCEPTANCE.md` keeps G1–G9 and audio gates distinct.

### Pinned package repair and delivery checks

The [dated package checks](history.md#pinned-package-repair-and-delivery-checks)
retain corruption/repair, local bundle-content, ignored real-asset tests and
their failures. A clean Windows install and signed/public release remain open.

## Historical fragment redirects

The headings below retain old `README.md#...` fragment destinations. Each
points to its original, dated section in [history.md](history.md); the broad
open matrix is not closed by a single named check.

### Guarded attempt admission

See [the admission guard record](history.md#guarded-attempt-admission).

### Cooperative preparation cancellation

See [the preparation record](history.md#cooperative-preparation-cancellation).

### Accepted host work and preparation status

See [the accepted-work record](history.md#accepted-host-work-and-preparation-status).

### Admission-completion cleanup precedence

See [the cleanup-precedence record](history.md#admission-completion-cleanup-precedence).

### Native accepted-worker preflight pause

See [the worker-preflight record](history.md#native-accepted-worker-preflight-pause).

### Native initial model-hash pause

See [the initial-hash record](history.md#native-initial-model-hash-pause).

### Native concurrent starts during preparation

See [the named start-race record](history.md#native-concurrent-starts-during-preparation).

### Historical edit foundation

See [the edit foundation](history.md#historical-edit-foundation).

### Manual publication ordering

See [the publication ordering record](history.md#manual-publication-ordering).

### Historical application save and recovery

See [the application record](history.md#historical-application-save-and-recovery).

### Historical desktop contracts and controls

See [the desktop contract record](history.md#historical-desktop-contracts-and-controls).

### Native historical branch and reopening

See [the completed-branch record](history.md#native-historical-branch-and-reopening).

### Native historical core commit interruption

See [the core-commit interruption record](history.md#native-historical-core-commit-interruption).
