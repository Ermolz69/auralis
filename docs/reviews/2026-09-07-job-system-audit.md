# Job System Audit — 2026-09-07

## Scope

This review traced job behavior from the frontend command and event store through
Tauri, application orchestration, the runtime registry, the job manager, SQLite
transactions, the lifecycle outbox, and shutdown. It focused on cancellation,
durability, concurrent terminal transitions, status delivery, and retained runtime
resources.

The maintained design contract is documented in
[Job runtime and cancellation](../architecture/007-job-runtime.md).

## Findings and Corrections

| Severity | Finding                                                                                                                                               | Correction                                                                                                                                                                                                |
| -------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| High     | Every successful mock pipeline spawned a cancellation bridge task that could wait forever because neither side was cancelled after normal completion. | Replaced the custom token and bridge task with one shared `tokio-util` cancellation token.                                                                                                                |
| High     | The custom cancellation primitive could miss a notification between its atomic-state check and waiter registration.                                   | Reused the race-safe `tokio-util` token directly and added before-wait and 256-waiter tests.                                                                                                              |
| High     | A transcript transaction could commit after the job cancellation transaction but before the outbox updated the project.                               | The conditional project write now verifies in the same SQLite transaction that the active job is still pending or running.                                                                                |
| Medium   | Terminal jobs remained in the in-memory cache, causing growth proportional to job history.                                                            | The cache now contains pending and running jobs only; terminal snapshots remain in SQLite.                                                                                                                |
| Medium   | Per-job mutation locks were retained after ordinary successful and failed operations.                                                                 | Every mutation path now releases its lock-map entry when no waiter still owns it.                                                                                                                         |
| Medium   | Repeating an idempotent cancellation could attempt another terminal write and collide with the outbox deduplication key.                              | Persistence and event publication now occur only when the domain revision changes.                                                                                                                        |
| Medium   | The API reported `cancelled` immediately after sending a signal, before the runtime had actually stopped.                                             | Cancellation is now two-phase: persist `cancelling`, wait for a sticky runtime-exit acknowledgement, then commit and return terminal `cancelled`.                                                         |
| Medium   | The public lifecycle could not distinguish an accepted cancellation request from confirmed runtime shutdown.                                          | Added `cancelling` across the domain, SQLite active queries and recovery, Tauri events, frontend validation, presentation, and active-job selection.                                                      |
| Medium   | Recovery queried legacy title-case status values while current job serialization writes lowercase values.                                             | Recovery now recognizes current lowercase and legacy title-case active statuses, including `cancelling`.                                                                                                  |
| Medium   | Shutdown could wait beyond its declared deadline while joining an aborted task that was executing synchronous code.                                   | The abort phase is bounded by the remaining overall deadline and reports unconfirmed tasks.                                                                                                               |
| Medium   | A failed frontend event publication could silently lose the only new job revision.                                                                    | The bridge now publishes an invalidation fallback so the frontend reloads SQLite-backed state.                                                                                                            |
| Medium   | Native acceptance could not stop the real pipeline at a deterministic point, so it did not prove the two-phase cancellation contract end to end.      | Added a feature-gated subtitle adapter that pauses at a known await point; the native React scenario now observes `cancelling`, confirms `cancelled`, and verifies SQLite, outbox, and workspace cleanup. |
| Medium   | Global terminal history was coupled to the newest-100 live snapshot.                                                                                  | Added terminal-only keyset pagination through SQLite, Tauri, runtime validation, API, and queue UI, with a stable `(created_at, job_id)` cursor.                                                          |
| Low      | Background reaping of multiple unconfirmed tasks was sequential.                                                                                      | Unconfirmed join handles are now reaped with a concurrent future set.                                                                                                                                     |
| Low      | Collection-size unit tests could miss slow allocator-level retention.                                                                                 | Added a separate ignored release-mode heap soak with live/peak byte tracking, regression slope limits, and a JSON report.                                                                                 |

The existing startup handshake, per-job mutation serialization, optimistic job
revisions, terminal job/outbox transaction, frontend listener-before-snapshot order,
revision-aware reducer, invalidation retry, and owned downloader process behavior
were retained and covered by regression runs.

## Added Regression Coverage

- Cancellation observed before waiting and by 256 concurrent waiters.
- Idempotent repeated cancellation with exactly one terminal transaction and event.
- Concurrent cancel versus complete with exactly one committed terminal state.
- Persistence failure leaving the runtime task running and durable status unchanged.
- Public cancellation remaining `cancelling` until cooperative runtime exit and task
  self-eviction are confirmed.
- Failed terminal persistence leaving a stopped runtime durably `cancelling`, so a
  retry can finish the transition without falsely reporting `cancelled`.
- Recovery loading lowercase pending, running, and cancelling jobs.
- 256 completed jobs leaving no active cache or mutation-lock entries.
- 256 failed mutations leaving no mutation-lock entries.
- Shutdown respecting its deadline for a temporarily unresponsive task.
- Cancelled SQLite jobs rejecting late transcript, artifact, and outbox writes.
- Frontend publication failure producing an invalidation fallback.
- SQLite history pagination reading 205 terminal jobs without duplicates while
  excluding active jobs, plus frontend loading of older pages.
- Native React-to-IPC-to-Rust-to-SQLite cancellation at a deterministic pipeline
  pause, including the intermediate event and post-cancel workspace cleanup.
- A standalone ten-minute allocation soak alternating completed and cancelled jobs;
  a short smoke profile is used to validate the harness itself.
- Local production-browser journeys for history pagination, two-phase cancellation,
  reload recovery with stale-event rejection, and history retry.
- Existing client-side revision gaps, stale snapshots, retries, cancellation UI, and
  terminal status presentation through a focused job test task.

## Validation Evidence

- `task check:jobs`: focused cross-stack job regressions.
- `task fe:e2e`: 34 production-browser journeys, including `E2E-031` through
  `E2E-034` for the additional job scenarios.
- `task desktop:e2e:native`: real native cancellation and persistence journey.
- `task rs:soak:jobs`: ignored release-mode long heap profile.
- `task rs:fmt`: Rust formatting check.
- `task rs:clippy`: Clippy for the full workspace with warnings denied.
- `task rs:test`: full Rust workspace tests.
- `task fe:test`: full frontend and Storybook test suite with coverage thresholds.
- `task docs:all`: documentation contract and Markdown lint.

## Residual Risks

1. The current dubbing runner is still a mock pipeline. Future production model and
   media stages must propagate cancellation and own any child process they start.
2. A runtime that ignores cooperative cancellation can leave its API call and job in
   `cancelling`. This is deliberate: the API no longer claims the runtime stopped
   without confirmation. Production stages must honor cancellation promptly.
3. Tokio cannot interrupt synchronous work already executing inside a task. Shutdown
   reports such work as unconfirmed at the deadline; production stages must avoid
   blocking the async runtime.
