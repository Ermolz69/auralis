# Production Storage Contract

Desktop production composition always uses SQLite as the durable source of truth.

- `AURALIS_STORAGE` unset: use SQLite.
- `AURALIS_STORAGE=sqlite`: use SQLite.
- Any other value: fail startup with an explicit configuration error.

SQLite owns the persistent project, job, artifact, and outbox state. The outbox worker is part of the production storage graph and is started whenever production storage is initialized.

The database is initialized directly from the final schema. Alternate storage formats are not
supported. An existing database whose tables do not match the final schema is rejected at startup.

The `adapters-storage::memory` module is retained only as test support. It is a test double for application and adapter tests, not a production backend, and it does not claim SQLite-equivalent transaction, outbox, or crash durability semantics.
