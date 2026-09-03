# Production Storage Contract

Desktop production composition always uses SQLite as the durable source of truth.

- `AURALIS_STORAGE` unset: use SQLite.
- `AURALIS_STORAGE=sqlite`: use SQLite.
- Any other value: fail startup with an explicit configuration error.

SQLite owns the persistent project, job, artifact, and outbox state. The outbox worker is part of the production storage graph and is started whenever production storage is initialized.

New databases use schema version 4. Valid version-1 databases gain `projects.revision` with an
initial value of 1, and version-1/version-2 databases gain `projects.avatar_data_url`. Version 3
gains the `youtube_imports` journal. Each migration is atomic and preserves existing project
data. Unsupported schemas are rejected at startup without conversion. Older application builds
cannot open version-4 databases; keep a database backup before downgrading.

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

Only pin preferences remain in localStorage. Reads validate JSON and entry shapes; inaccessible
storage retains the last readable snapshot. Failed mutations and deletion tombstones stay in
memory for the session and are flushed on the next successful preference write. Unreadable or
malformed storage is not overwritten. This fallback is not durable across application restarts.

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

Atomic YouTube import explicitly returns `Unsupported` on the in-memory adapter because it has
no shared durable artifact/outbox transaction. Its contract tests use the real SQLite adapter.
