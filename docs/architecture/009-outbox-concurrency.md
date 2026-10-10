# Project State and Outbox Concurrency

## Final-state owner

The job coordinator alone chooses the terminal outcome of a parent job. `JobManager`
persists its terminal job revision and `HandleTerminalJobState` intent in one SQLite
transaction. The outbox handler is the sole application path that applies that
computed outcome to the project. Restart recovery verifies the current job and
project revisions, then restores a missing terminal outbox intent; it does not
apply a terminal job result directly. An orphaned project with no valid job is
still failed by the separate corruption-repair path.

The terminal project transaction rereads the current project. It applies only
`status`, `active_job_id`, `last_terminal_job_id`, `revision`, and `updated_at`.
Its SQL checks the revision read in that transaction, `Processing`, the active
job identity, and, for worker delivery, the current outbox token and the matching
terminal job row. Completion also waits until no project artifact is
`pending_finalize`. A rename can commit before this transition and remains intact.
An older job yields `IgnoredStale`; an already applied job yields `AlreadyApplied`;
a deleted project yields `ProjectMissing`. Replaying the terminal message never
increments the project revision twice. A replay of an already applied event sends
an idempotent project invalidation so the UI can reload authoritative state.

Source and transcript imports still commit their project revision, artifact row,
and finalization intent atomically. Their conditional SQL checks expected revision,
status, active job, and timestamp. A transcript import can retry a revision
conflict twice: it reloads the current project, checks the same source and job,
and applies only the intended transcript change to the fresh snapshot. A changed
source, cancellation, deletion, or another transcript is not retried.

## Resource and dependency table

| Payload                     | Reads and writes                                                                                                     | Conflict group and predecessor                                                                                                                                                        |
| --------------------------- | -------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `FinalizeStagedArtifact`    | Reads pending artifact and intent; moves staging bytes to the final key; marks artifact ready.                       | Project. Must precede ready-result reconciliation and cleanup of its staging, final file, or project directory.                                                                       |
| `DeleteStorageKey`          | Deletes a staging or final file.                                                                                     | Project from committed origin or validated final-key prefix. An unidentifiable legacy staging key uses the global group. Must wait for earlier finalization.                          |
| `DeleteProjectArtifactDir`  | Removes the entire project artifact subtree.                                                                         | Project. Waits for every earlier operation in that subtree, not just exact-file operations.                                                                                           |
| `DeleteProjectTranslation`  | Deletes the linked Translate projection through its integration port.                                                | Project. Follows earlier project publication and precedes later project cleanup.                                                                                                      |
| `DeleteWorkspaceAllocation` | Removes a whole temporary allocation.                                                                                | Project from committed origin or validated `tmp/<project>/...` key. Unidentifiable legacy keys use the global group. Must wait for staging publication that still uses the workspace. |
| `HandleTerminalJobState`    | Reads terminal job and current project; applies the conditional project transition; emits invalidation after commit. | Project. Cannot overtake earlier finalization, retry, or deletion; old job identities cannot change a newer job.                                                                      |

An outbox intent receives a monotonic `sequence` inside the transaction that
creates it. The scheduler selects only the oldest unresolved operation in each
conflicting group. `created_at` is diagnostic, never an ordering key. The global
group `*` conflicts with every project, so legacy messages without trustworthy
resource provenance do not become parallel by accident. A pending retry,
`processing` predecessor, or `dead` predecessor blocks later conflicting work.
Independent project groups remain eligible even if an earlier group is blocked.
Dead rows needed by dependents are retained by maintenance; an operator can
repair and retry a dead finalization. Project deletion is a documented
compensation: in its SQLite transaction it verifies deletion, resolves
unclaimed project intents as obsolete, and enqueues file and workspace cleanup.
It leaves processing attempts owned until they finish or their lease expires.
Successful predecessors are also retained while later dependent work remains
unresolved, so pruning cannot erase the evidence that made the order safe.
Cleanup intents retain their project provenance even for a discarded import
that never created a project row; only cleanup kinds may be saved in that case.

## Claim ownership and recovery

The production SQLite repository atomically claims at most the configured number
of eligible groups. Each claim uses a fresh UUID token in `locked_by` and a
renewable `locked_at` lease. Success, failure, renewal, and artifact-finalization
metadata writes compare both message ID and token. A reclaimed or cancelled
handler cannot acknowledge the replacement attempt or publish metadata.
Other outbox adapters must implement fenced claims and protected-resource
queries before running this worker; its default methods fail closed.
Heartbeats run while the payload future is active. An expired lease increments
the attempt count and retries under a new token; unrecoverable non-Translate
messages become `dead` after five attempts. Translate project cleanup retains
its existing retry behavior.

Filesystem delivery is at least once. The local finalizer accepts a previously
finalized file only after verifying its expected size, and the metadata
transaction checks project, artifact ID, pending or ready state, final key,
stored intent, and current token. Deletion operations are idempotent. If a file
effect completes before acknowledgement, a retry validates it and finishes the
same intent. Ready-result reconciliation runs after the batch and requires the
artifact's committed `ready` state and the current publication identities.
Events are invalidations after committed state; they are not exactly-once events.

The desktop worker reserves one disk unit from the existing job queue resource
pool before each claim. Claims waiting for a group or a disk slot do not occupy
`processing` rows. `max_active`, `fetch_batch`, lease duration, heartbeat interval,
and shutdown deadline are validated. A value of one for `max_active` preserves
sequential execution. The run loop owns its batch task. Shutdown stops new claims,
waits for active attempts and maintenance within the configured deadline, then
leaves unacknowledged claims for lease recovery. The staging and workspace
janitors exclude keys referenced by unresolved outbox intents and import sessions.

Schema version 14 adds `sequence`, `conflict_group`, and the transactional
`outbox_sequence` allocator. Migration backfills old rows by SQLite insertion
order. A legacy project group is retained only when its payload parses and its
project ID matches the stored aggregate ID; other rows use the global group.
Existing payload JSON and shared outbox variants are
unchanged.

## Verification and measured effect

Run `task rs:test:outbox`, `task rs:bench:outbox`, `task rs:clippy:outbox`,
`task rs:fmt`, and the applicable job, queue, documentation, and quality gates.
The benchmark publishes eight separate 2 MiB local files per trial with the
same two-unit disk budget, using one or two outbox executors and no artificial
delay. On the local Windows host, three paired trials measured only the worker
publication phase after slot refilling was enabled: one executor took 142.9,
131.4, and 143.0 ms; two executors took 87.7, 88.8, and 96.6 ms. The medians
were 142.9 and 88.8 ms, respectively (37.9% lower with two executors).
Each pair ran sequential first, then parallel, in fresh temporary directories;
OS cache and antivirus activity may affect this small local sample.
Observed peak active attempts were one and two. SQLite write transactions
remain serialized. Concurrent read-then-write finalizations initially produced
`SQLITE_BUSY_SNAPSHOT` retries; the short metadata and terminal transactions
now acquire their write reservation before reading. Disk throughput may limit
or reverse the gain on another host.
