# Production Storage Contract

Desktop production composition always uses SQLite as the durable source of truth.

Schema version 13 combines durable `job_executions`, speech run tables, and
immutable translation review events. Version-11 review-only databases and
version-12 speech/queue databases upgrade without losing their existing rows.
`job_executions` is keyed to a job with cascading deletion.
The queue commits the project link, Pending row and frozen execution descriptor
in one `BEGIN IMMEDIATE` transaction. Its capacity check participates in that
transaction. Dispatch checks the job revision and owning project's active link;
terminal writes still commit the lifecycle outbox atomically. See
[durable dispatch](../architecture/008-job-dispatch.md). Older builds cannot open
version-13 databases.

- `AURALIS_STORAGE` unset: use SQLite.
- `AURALIS_STORAGE=sqlite`: use SQLite.
- Any other value: fail startup with an explicit configuration error.

SQLite owns the persistent project, job, artifact, and outbox state. The outbox worker is part of the production storage graph and is started whenever production storage is initialized.

New databases use schema version 14. Valid version-1 databases gain `projects.revision` with an
initial value of 1, and version-1/version-2 databases gain `projects.avatar_data_url`. Version 3
gains the `youtube_imports` journal. Version 4 gains `ui_settings`, `project_pins`, and
`ui_migrations`. Version 6 gains `project_translations` and `translation_publications` for
stable links to the separate Translate SQLite database. Version 7 adds frozen
`translation_run_intents` for crash-safe run creation. Each migration is atomic and preserves existing project
data. Unsupported schemas are rejected at startup without conversion. Older application builds
cannot open version-13 databases; keep a database backup before downgrading.

An exclusive OS lease on `.auralis-owner.lock` is acquired before opening SQLite or running
recovery and retained for the runtime lifetime. Another process using that root is rejected;
independent roots are allowed. The filename survives exit; ownership is the OS lock, not the
existence of the file. SQLite connections explicitly use WAL, FULL synchronous mode, foreign
keys, and a five-second busy timeout. Filesystem and hardware guarantees still apply.

Project revisions advance once per committed aggregate mutation, including pipeline, transcript,
source-import, and recovery writes. Rename and metadata-only source import use field-scoped
updates guarded by the expected revision. An outdated revision returns `Conflict` without changing
the project; a missing project returns `NotFound`. Callers must reload before retrying a conflict.
Full project writes inside a unit of work also compare revisions, so a stale pipeline or import
snapshot cannot overwrite a successful rename. Revision changes roll back with the transaction.

## Project presentation preferences

The selected project is one discriminated `closed`/`open` state. An open selection contains the
complete project; its ID is derived, never independently assigned. A confirmed `NOT_FOUND` from
project synchronization atomically closes the selection and synchronously invalidates operation
tokens. The project route returns to home, workspace controls become unavailable, and the job
provider masks the old scope immediately while disposing its listeners and pending requests.
Late responses cannot reopen a closed project or close a different selection. Transient backend
errors retain the current selection.

Backend delete and rename results are independent of browser preference persistence. Confirmed
deletion always clears the active project context. `projectUpdated` and `projectRemoved` notify
UI consumers directly, without dummy localStorage writes. Older in-flight fetches cannot undo
these notifications. Preference cleanup failures produce a separate warning, never a false
backend-command failure.

Theme and pin preferences are authoritative in SQLite and use separate preference revisions.
Theme initialization imports only a recognized legacy value when no native row exists; a missing
value never triggers a default write. Existing native values win over legacy values. Pin migration
inserts only missing project preferences and commits its migration marker in the same transaction.
Legacy fields are removed only after backend acknowledgement; pin migration retains avatar fields.
Malformed or inaccessible legacy storage is preserved. Failed writes remain session-only and are
reported to the user, with explicit retry; they are not durable across application restarts.

Avatars are stored in SQLite's project row, not the shared preferences JSON. These presentation
writes do not advance the aggregate revision or timestamp, and aggregate saves cannot overwrite
them. Deleting a project atomically removes its avatar; late avatar writes return `NotFound`.
The backend accepts base64 PNG, JPEG, GIF, and WebP data URLs with matching file signatures and
at most 1 MiB of decoded data; SQLite also limits encoded length. SVG is not accepted.

Legacy avatars migrate lazily when their project row is displayed. Conditional initialization
cannot replace a newer avatar or an explicit removal. SQL NULL means never initialized; an
empty string is an explicit removal tombstone. The browser copy is removed only after confirmed
backend persistence. Failed migration retains the old copy and shows a warning; failed legacy
cleanup leaves the SQLite value authoritative and reports a separate warning.

## Atomic YouTube import

YouTube validation, metadata lookup, download, and staging happen before any persistent project
mutation and outside the project lifecycle lock. A new project has no row in `projects` at this
point; an existing project must remain `Draft` without an active job. After metadata lookup,
the desired snapshot and owned workspace key are persisted in a separate import journal.

After staging, the use case acquires the shared lifecycle lock and revalidates the project.
The journal commit atomically removes the matching checkpoint, inserts the new project or
conditionally updates the existing one, inserts the pending video artifact, and writes both
finalization and allocation-cleanup outbox messages. The transaction checks the journal revision
and payload plus the original project revision, timestamp, `Draft` status, and absence of an
active job. Source, metadata, and `ReadyForProcessing` become durable together. Existing projects
advance their revision once; new projects start at revision 1.

Any failure before commit preserves the existing project exactly and leaves no new project,
artifact row, or outbox message. Transaction rollback replaces compensating project deletion:
there is no separately committed project to remove. Failed journaled imports retain their
allocation and staging checkpoint for an explicit retry. Allocation failures before journal
insertion still use immediate owned-resource cleanup.

The home screen lists unfinished imports with explicit Resume and Discard actions; startup
does not automatically use the network. The journal stores `Downloading`, `Staged`, or `Failed`,
not new project statuses. A per-project OS file lease prevents concurrent resumes and discard
while a download is running. An explicit retry reloads the current Draft revision and title;
changes during that attempt still conflict. Project deletion removes the journal in its own
transaction, and the running download notices cancellation within its one-second polling interval.

Resume reuses the same allocation and stable `original` filename with yt-dlp's `--continue`
and `--part` options. Partial files and fragment state remain available. Byte-level continuation
depends on the server and yt-dlp format support; changed or expired upstream media can require
downloading again. A valid persisted staging checkpoint skips the downloader completely. A
missing or truncated staged file falls back to the retained download. Staging copies the source
instead of moving it, so a crash before checkpoint persistence does not lose the completed
download. This temporarily requires space for both copies.

Windows downloads run inside a kill-on-close Job Object. Unix downloads use a private process
group and a pipe-lifetime monitor. If the application is killed, yt-dlp and its ffmpeg descendants
are terminated rather than continuing to write into a workspace later reused by Resume.

The janitors exclude pending journal allocations, checkpoint files, and unacknowledged outbox
staging files, including retryable finalization. Unjournaled abandoned staging is still aged out.
Dead finalization messages are not age-pruned while they protect a live pending artifact.
Malformed recovery payloads are retained conservatively. The home-screen file-finalization retry
requeues only the selected project's dead pending-artifact messages, without starting downloads.
Successful finalization and project deletion allow the normal retention/cleanup policies.
Discard atomically removes the journal and enqueues owned-file cleanup without deleting an
existing project. Small lease files under `.import-locks` are retained; their OS locks are released
when the holder exits. After commit, outbox processing resumes without downloading again. A
message interrupted while claimed is reclaimed after the existing five-minute lease expires.

Run `task rs:test:youtube` and `task rs:test:storage` for the recovery regressions. They forcibly
terminate separate processes during partial download, on either side of staging checkpoint
persistence, before/after commit, inside the uncommitted SQLite transaction, and after file
finalization before outbox acknowledgement. They reopen the actual SQLite database and verify
rollback, resume, idempotency, public artifact visibility, retention, and cleanup. External
YouTube availability is deliberately not required by these deterministic tests.

## Workspace ownership

Opening a project folder holds the same per-project lifecycle lock as deletion, from the
repository existence check through the workspace adapter call. The existence check happens
after acquiring the lock, never against a pre-lock snapshot. If deletion wins, opening returns
`ProjectNotFound` without touching the filesystem. If opening wins, deletion waits until the
adapter finishes and then removes the workspace through the existing outbox cleanup.

Empty Draft projects still use lazy workspace creation on their first successful open. This
operation is allowed only under the lifecycle lock for a project that still exists; it cannot
recreate a workspace after application-level project deletion. Folder provisioning is not
moved into project creation in this change.

YouTube downloads pass the allocation's `WorkspaceKey` unchanged into the artifact transaction.
The `DeleteWorkspaceAllocation` outbox payload removes the whole owned temporary allocation after
staging; it never derives ownership from a downloaded filename or an absolute path. Replaying
cleanup for an already removed allocation succeeds. The former `delete_workspace_file` payload
name remains readable for persisted messages, but new messages use `delete_workspace_allocation`.
Old malformed filename-only keys cannot be safely reconstructed and are not automatically repaired.

Artifact staging and external imports prefer the filename hint's extension, then the source
file's extension, and use `bin` only when neither provides a usable extension.

The `adapters-storage::memory` module is retained only as test support. It is a test double for application and adapter tests, not a production backend, and it does not claim SQLite-equivalent transaction, outbox, or crash durability semantics.

## Translation project links

Schema version 9 adds `translation_publications.result_revision`, copied from the immutable Translate result. Publication and selection prefer the highest staged revision for the same translation, run, source digest, and expected link revision. An older ready artifact remains unselected while a newer revision is pending or failed; the failed publication needs attention. Version-8 publication rows migrate with revision 1. This ordering is enforced both in the outbox candidate query and the conditional selection transaction.

Startup publication recovery lists links that have an active run or a selected result. For each, the application resolves the owned run, reads its latest Translate result, and checks publication by that exact result ID. This closes the cross-database gap after a manual edit commit, including edits to a previously published result. The application stages the missing copy without duplicating subtitle text in Auralis SQLite.

Schema version 6 adds `project_translations` and `translation_publications`. They contain stable external Translate IDs, source and output artifact IDs, source digest, link revision, publication state, and the small review projection. Translation text, checkpoints, and detailed warnings remain in the separate Translate SQLite database.

`TranslationLinkStore` can read a link by ID, list the links of one project in creation order, and list all links with an active run in update order for startup reconciliation. The active list includes paused or failed Translate runs because Auralis keeps their `active_run_id` until a ready artifact is selected. These reads return only Auralis link metadata; run progress and translated text are fetched from Translate by the integration layer.

For a validated Translate result, `commit_staged_publication` inserts the pending `TranslatedTranscript` artifact, its `translation_publications` row, and the finalization outbox message in one Auralis SQLite transaction. It verifies the active run, source digest, link revision, artifact ID, size, kind, and storage keys. A retry with the same identities is idempotent while the publication is pending or ready; a failed publication or changed input conflicts. A stale revision rolls back every inserted row. The application must call this combined operation after staging verified output bytes. The outbox finalizes the artifact, and the worker conditionally calls `select_ready_result` only after it is ready. The isolated publication-row writer is internal to the adapter and cannot be called through the application port.

`TranslationLinkStore::begin` records an active run against a ready `OriginalSubtitle` artifact in the same project. Repeating the same intent returns the existing link; changing its source or run is a conflict. `start_run` conditionally assigns a later run, retains an earlier selected result and artifact, and advances the link revision. `commit_staged_publication` requires the active run, source digest, and expected link revision to match and records a pending `TranslatedTranscript` artifact with its finalization intent. `select_ready_result` requires that artifact to be ready, then selects the result and artifact together, clears the active run, and advances the link revision in one Auralis transaction. Repeated start and completion calls with the same identity return the stored link. Project deletion cascades the link and publication records.

The production outbox worker now queries ready pending translation publications after each batch, including empty batches. It selects a result only when the artifact is ready and the link still has the same active run, source digest, and revision. This repairs the crash window after artifact finalization, even if its outbox message was already marked done; stale publications are not selected. The application integration test covers normal finalization and this restart gap.

The application now registers runs in Translate SQLite, reconciles active frozen intents, and stages a verified Translate result for publication. It reconstructs the output from the immutable original, checks the stored result digest, and verifies the staged copy's digest before recording publication. Host model jobs, desktop-startup wiring, Tauri/UI, and project editing remain open; the user cannot yet translate through Auralis. Auralis storage cannot query the separate database inside its transaction.
The caller must also stop or explicitly supersede an earlier active Translate run before using `start_run` with its ID as the expected active run. The link store enforces identity and revision, but it cannot determine whether a worker in the separate Translate process is still running.

Schema version 7 adds `translation_run_intents`. `begin_with_run_intent` atomically records the
project link and the frozen run configuration before any write to Translate SQLite. The record
contains the same `translation_id` and `run_id`, exact profile manifest and SHA-256, parser
version, source policy fingerprint, block size, and context bounds. It contains no subtitle
text. Repeating the same IDs and configuration succeeds; conflicting configuration fails.
`run_intent(run_id)` reads the record for reconciliation after a crash between the two database
writes. A failed intent insert rolls back the link. Earlier version-6 links migrate without
invented run-intent records; the integration layer must handle those legacy links explicitly.
New starts use `begin_with_run_intent`; `begin` remains for compatibility with older callers.
`BeginTranslationUseCase` commits this intent before idempotent Translate registration, and
`ReconcileTranslationRunsUseCase` can replay it for every active link after a failed second write.
The reconciliation entry point is not yet wired to desktop startup. The current `start_run`
method does not yet freeze configuration for later runs.

Atomic YouTube import explicitly returns `Unsupported` on the in-memory adapter because it has
no shared durable artifact/outbox transaction. Its contract tests use the real SQLite adapter.
