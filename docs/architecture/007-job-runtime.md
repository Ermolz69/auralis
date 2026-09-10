# Job Runtime and Cancellation

## Responsibilities

The job subsystem separates durable lifecycle state from in-process execution:

- SQLite stores job status, revision, timestamps, errors, and lifecycle outbox events.
- `JobManager` serializes mutations per job and keeps only active jobs in memory.
- `JobRuntimeRegistry` owns cancellation handles and task join handles for running jobs.
- Application runners translate pipeline outcomes into terminal job mutations.
- The Tauri event bridge publishes revisioned snapshots to the frontend; invalidation
  tells the frontend to reload when a specific event cannot be delivered.
- The frontend reducer accepts only newer revisions, so reloads and live events may
  arrive in any order without reverting state.

SQLite is the source of truth. Runtime caches and UI state are reconstructible and
must never be treated as proof that a lifecycle transition was committed.

## Cancellation Contract

Cancellation uses one cloneable `CancellationToken` from the scheduler through the
pipeline. The command follows this order:

1. Lock the job mutation lane.
2. Persist `cancelling` and publish that revision.
3. Signal the registered runtime task.
4. Wait for its sticky completion acknowledgement. The acknowledgement is recorded
   only after the task removes its own runtime-registry entry.
5. Persist terminal `cancelled` and its outbox event in one SQLite transaction, then
   publish the terminal revision and return from the API command.

If the initial persistence fails, the runtime is not signalled and the durable job
remains running. If the terminal persistence fails after runtime exit, the durable
job remains `cancelling`; repeating the command safely retries terminalization. A
repeated cancellation is otherwise idempotent and does not create duplicate
lifecycle events. Project deletion and application shutdown use explicit cleanup
operations that also join the task and report the cleanup outcome.

Pipeline artifact commits conditionally verify the project revision, active job id,
and that the referenced job is still pending or running. This closes the race where
a cancelled job could otherwise commit an artifact after its terminal transaction.
Whichever transaction commits first wins; the loser receives a conflict and cleans
up its staged file.

## Memory and Shutdown Guarantees

- Terminal jobs are removed from the runtime cache; their history remains in SQLite.
- Per-job mutation locks are released after successful and failed operations.
- Runtime registry entries are taken exactly once and removed before cleanup starts.
- Background cleanup of unconfirmed tasks joins them concurrently, so one stuck task
  does not retain unrelated cleanup futures.
- Shutdown spends part of its deadline on cooperative cancellation, aborts remaining
  tasks, and returns when the overall deadline expires. Tasks that are executing
  blocking code may be reported as unconfirmed because Tokio cannot interrupt a
  synchronous operation already in progress.

All pipeline adapters must therefore remain cooperatively asynchronous or place
external work in an owned process that can be terminated.

## Status Delivery

Job revisions are monotonic. The event bridge emits a full job snapshot for each
accepted mutation. If publication fails, it emits an invalidation event so the
frontend reloads authoritative state. The frontend also reloads on startup and uses
revision-aware merging to reject stale snapshots.

Project lifecycle updates use the outbox and can lag the job snapshot briefly. This
is expected eventual consistency; a terminal job row remains authoritative while
the outbox worker reconciles the related project state.

The startup snapshot remains capped at the newest 100 jobs because it is a live-state
synchronization primitive. Terminal history uses a separate SQLite keyset cursor
ordered by `(created_at, job_id)`, so the queue can load every older page without
holding the entire audit history in memory or skipping rows with equal timestamps.

## Regression Suite

Run the focused suite with:

```bash
task check:jobs
```

It covers:

- cancellation before and during a wait, including many simultaneous waiters;
- API cancellation remaining in `cancelling` until runtime exit is acknowledged;
- cooperative task cancellation and forced shutdown deadlines;
- idempotent cancellation and competing cancel/complete transitions;
- persistence failure before runtime cancellation;
- active-cache and mutation-lock boundedness across many completed and failed jobs;
- SQLite rejection of artifact commits after cancellation;
- pipeline cleanup and lifecycle behavior;
- cursor pagination beyond 100 terminal jobs, including equal-timestamp rows;
- event-publication fallback to frontend invalidation.

The native acceptance path is compiled with the `native-e2e` feature. Its subtitle
adapter stops at a deterministic asynchronous boundary, the React runner requests
cancellation, observes `cancelling`, waits for terminal `cancelled`, and the harness
then verifies SQLite, terminal outbox state, and workspace cleanup. The same run
imports a generated local video through the bundled `ffprobe`, serves that video from
an isolated loopback HTTP endpoint, and downloads it through the bundled `yt-dlp`.
The database, media fixture, WebView2 profile, staging files, workspaces, and frontend
test build are removed in the harness `finally` path on success or failure.

The production-browser suite also keeps deterministic local journeys for terminal
history beyond 100 rows, the visible `cancelling` interval before runtime shutdown,
reload recovery with stale-revision rejection, and history retry after a temporary
storage error. Run them together with `task fe:e2e -- --test-name-pattern=jobs` or
individually by their stable `E2E-031` through `E2E-034` identifiers.

Run the standalone allocation profiler with:

```bash
task rs:soak:jobs
```

It runs for at least 10 minutes and 100,000 alternating complete/cancel lifecycles by
default. The ignored release-mode test records live and peak heap bytes, allocation
counts, retained bytes, and the live-byte regression slope in
`target/job-soak/heap-profile.json`. Duration, iteration count, report path, and
thresholds are configurable through the task variables and documented environment
variables in the test.

## Remaining Boundaries

- The current dubbing pipeline is a mock orchestration path. Each production media or
  model adapter needs the same cancellation contract and an owned-process test.
- A runtime that ignores cooperative cancellation keeps the command and durable job
  in `cancelling`; production stages must remain cancellation-aware. Application
  shutdown has a separate deadline and forced-abort path so process exit is bounded.
