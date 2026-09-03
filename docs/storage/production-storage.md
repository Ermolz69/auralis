# Production Storage Contract

Desktop production composition always uses SQLite as the durable source of truth.

- `AURALIS_STORAGE` unset: use SQLite.
- `AURALIS_STORAGE=sqlite`: use SQLite.
- Any other value: fail startup with an explicit configuration error.

SQLite owns the persistent project, job, artifact, and outbox state. The outbox worker is part of the production storage graph and is started whenever production storage is initialized.

New databases use schema version 2. Valid version-1 databases are upgraded atomically by adding
`projects.revision` with an initial value of 1; existing project data is preserved. Unsupported
schemas are rejected at startup without conversion.

Project revisions advance once per committed project mutation, including pipeline, transcript,
source-import, and recovery writes. Rename, source import, and YouTube readiness use field-scoped
updates guarded by the expected revision. An outdated revision returns `Conflict` without changing
the project; a missing project returns `NotFound`. Callers must reload before retrying a conflict.
Full project writes inside a unit of work also compare revisions, so a stale pipeline or import
snapshot cannot overwrite a successful rename. Revision changes roll back with the transaction.

YouTube downloads pass the allocation's `WorkspaceKey` unchanged into the artifact transaction.
The `DeleteWorkspaceAllocation` outbox payload removes the whole owned temporary allocation after
staging; it never derives ownership from a downloaded filename or an absolute path. Replaying
cleanup for an already removed allocation succeeds. The former `delete_workspace_file` payload
name remains readable for persisted messages, but new messages use `delete_workspace_allocation`.
Old malformed filename-only keys cannot be safely reconstructed and are not automatically repaired.

Artifact staging and external imports prefer the filename hint's extension, then the source
file's extension, and use `bin` only when neither provides a usable extension.

The `adapters-storage::memory` module is retained only as test support. It is a test double for application and adapter tests, not a production backend, and it does not claim SQLite-equivalent transaction, outbox, or crash durability semantics.
