# Durable Job Dispatch

Implementation scope: the shared job runtime and the existing subtitle pipeline,
including desktop composition, project deletion, recovery and shutdown. Speech
engines and Translate algorithms are outside this change.

SQLite owns admission and execution descriptors. Admission atomically commits a
Pending job, the Processing project link and its versioned descriptor, subject to
a bounded queue capacity. Descriptors freeze the operation, version, project and
all subtitle selection parameters. Title and JobKind never reconstruct a request.

The dispatcher reads a bounded oldest-first window. It atomically reserves the
entire CPU/network/disk/process demand, attaches a gated executor and acknowledges
attachment before committing Running under the job mutation lane. SQLite also
checks revision, project status and active_job_id. Only then is the worker released.
Cancellation uses the same mutation lane; no cancelled Pending job can be released.

Fitting tasks may bypass a blocked older task. After a bounded number of successful
bypasses, younger tasks sharing any of its resource categories stop entering until
the older task fits. Disjoint resource categories remain eligible. The fairness
counter belongs to the bounded window and resets on application restart.

Managed workers retain resource ownership through child exit, CPU work and mandatory
cleanup. CPU closures use a fixed thread pool with a bounded backlog. Managed
workers are cooperatively cancelled, never force-aborted to simulate child exit.
Unconfirmed cleanup keeps capacity reserved and remains visible for attention.

Startup preserves validated Pending executions after lifecycle reconciliation.
Missing or invalid descriptors are explicit recovery failures. Running work follows
the existing interruption policy and is never automatically retried.

Shutdown closes admission and dispatch first, joins the dispatcher and cooperatively
stops managed workers within one overall deadline. Pending records remain durable;
unconfirmed work is reported and retains its resource ownership until it stops.

Acceptance requires SQLite-backed application tests for contention, cancellation,
exact descriptor recovery and deletion; deterministic scheduler fairness tests;
owned-process exit checks; and a reproducible fixture-only sequential/parallel
comparison. Compilation alone is not evidence of these behaviors.

## Composition and admission

`JobManager::with_queue` installs the SQLite execution store, resource allocator,
fixed CPU pool and validated `JobQueueConfig`. Desktop storage runs lifecycle
reconciliation before composition starts the dispatcher. The production
`StartMockPipelineUseCase` uses `JobQueuePort`; its executor is built from the same
project repository, artifact store, subtitle adapter and import use case as the
original pipeline. No subtitle discovery, workspace allocation, download, parsing
or child spawn happens at admission.

The admission transaction freezes a `youtube_subtitle_import`, version 1 request:
project identity, the complete selected track (including identity, language,
format and automatic/manual selection), preferred languages, automatic-track
policy and resource demand. JSON rejects unknown fields. The contract bounds the
descriptor to 16 KiB. The owning project source cannot change while its pipeline
is active. Admission returns the accepted Pending revision even if dispatch
commits a newer revision before the command response arrives; existing frontend
revision guards reconcile this order.

Capacity counts durable described Pending jobs, independently of history. A full
queue returns `QueueFull` without a job, descriptor or Processing project mutation.
The project revision and absence of another active pipeline job are checked in the
same transaction. Runtime caches contain active IDs, not execution history. The
dispatcher loads at most one configured window; history remains in SQLite. Job
listing also queries active durable rows so old Pending jobs remain visible.

## Dispatch and fairness

One allocator mutex reserves all four counters plus an active-worker slot in one
operation. Failed reservations retain nothing. Demands may use any subset; a
process demand also needs CPU capacity. Executor contract validation determines
the CPU demand of its children, including any internal CPU threads. Subtitle
import v1 reserves one unit in every category because it runs one subtitle-only
yt-dlp process, asynchronous network/filesystem work and a synchronous parser.
It does not download video or run model/ffmpeg inference.

Ordering uses creation time and job identity. Younger fitting work can bypass a
blocked head inside the window. After eight successful bypasses by default, the
head fences younger work sharing its categories; running work drains until the
whole demand fits. Disjoint work continues. This prevents starvation given finite
execution and confirmed cleanup. The bounded window deliberately limits lookahead;
a fitting task beyond that window waits until the window advances. Restart resets
only fairness counters. Unconfirmed work can block its categories indefinitely;
fairness never overrides ownership to reclaim them.

The mutation lane covers Pending reload, registry insertion, attach acknowledgement,
the conditional durable Running commit and release. The worker initially waits on
activation and release channels and does no useful work. A cancelled/replaced/deleted
row cannot commit Running. Start failure closes the release gate, joins the unused
executor and returns its complete reservation. Damaged descriptors in the window
fail explicitly without preventing subsequent valid work from dispatching.

## Execution and ownership

The CPU pool has exactly the configured CPU number of reusable OS threads, a
bounded synchronous backlog and nonblocking submission. One CPU closure may be
outstanding per job. Panics become typed errors and do not destroy the pool.
The closure retains a resource lease; dropping its async waiter cannot free CPU
capacity. The supervisor drains accepted CPU work before acknowledging completion.

`JobExecutorPort::execute` must join its work and mandatory cleanup. The production
runner drains import instead of racing its future against cancellation. Cancellable
subtitle-port compatibility methods drain old adapters; the production yt-dlp
adapter stops and joins its owned child tree. Windows uses a Job Object and a single
tree wait after parent polling, avoiding a cached-parent-exit race. Unix closes an
app-owned monitor pipe and verifies disappearance of the private process group.
Pipe reads are bounded. A timeout or missing exit proof returns `CleanupFailed`.
The real owned-tree test was run on Windows; Unix tree confirmation still needs
platform execution evidence.

Failed import joins workspace/staging cleanup. Successful queued import first
commits transcript, staged bytes and artifact finalization outbox atomically, then
joins removal of its temporary workspace. Finalization remains the existing durable
outbox operation; the queue does not publish a partially finalized artifact.

Normal success, failure and cancellation remove the registry entry/cache ID and
release a single shared lease. A process-owning executor panic, or unconfirmed
cleanup, instead preserves the registry/resource guard and records nonterminal
Cancelling with `CLEANUP_UNCONFIRMED` and `cleanup_unconfirmed` progress. Capacity
is quarantined for the current runtime. Repeated cancel cannot pretend that child
exit was proved. Restart applies interruption reconciliation and existing filesystem
recovery; it does not resume the abandoned operation. Subscriber panics cannot undo
a committed state or interrupt runtime ownership.

## Cancellation, deletion and shutdown

Pending cancellation writes Cancelling under the dispatch mutation lane and then
Cancelled plus its lifecycle outbox. Running cancellation first persists Cancelling,
signals the token, waits for sticky post-cleanup acknowledgement and writes the
terminal/outbox transaction. Initial write failure sends no signal. Terminal write
failure leaves a safely retryable Cancelling row. Managed cancellation has a configured
cleanup deadline; expiry records cleanup attention and keeps the worker/lease alive.
If that worker later confirms cleanup, repeated cancellation commits Cancelled and
clears the stale cleanup-attention error and progress marker.

Project deletion removes durable queue entries with the existing deletion transaction
before cancelling registered work. Conditional Running and artifact writes still
check ownership/status/revision, preventing a late launch or publication. Cleanup
does not abort a managed future that might own an external process.

Shutdown closes admission and stops/joins the dispatcher, then concurrently persists
and signals active managed cancellations within one overall deadline. It closes CPU
submission after stop requests. Pending descriptors survive unchanged. Unconfirmed
workers remain owned and are reported; they may finish cleanup after the deadline.
No dispatcher remains running. Shutdown cannot forcibly interrupt a synchronous
closure; reporting unconfirmed work is intentional. Legacy attachment callers keep
their previous cooperative/forced-abort behavior and are not treated as durable
queued executions.

## Recovery and supported executors

Startup keeps a Pending request only when a valid supported descriptor matches its
Processing project and active job link. Missing descriptors use
`EXECUTION_DESCRIPTOR_MISSING`; malformed/unsupported descriptors use
`EXECUTION_DESCRIPTOR_INVALID`. Invalid resource demands or executor profiles fail
when dispatch validates the reconciled window. Running/Cancelling use the existing
`APP_RESTART` failure policy. No irreversible operation is repeated automatically.
Failed recovery persistence blocks startup before dispatch.

The current desktop queued execution contract supports subtitle import v1. Existing
legacy scheduler/registry users remain compatible, including Translate's separate
checkpoint-aware worker protocol. They do not acquire the new queue budgets merely
by attaching a legacy task. Adding another queued operation requires a versioned
descriptor, cheap validation and an executor with joined cleanup; neither JobKind
nor a title authorizes reconstruction. This change does not implement ASR, TTS,
general workflow graphs, pause, model tuning or a resource-settings interface.

## Validated defaults

Configuration is supplied in Rust composition; desktop uses these defaults:

| Setting                               | Default                                          | Accepted bounds                   |
| ------------------------------------- | ------------------------------------------------ | --------------------------------- |
| CPU units / fixed CPU threads         | available logical CPUs minus one, clamped to 1–4 | 1–256                             |
| Network / disk / child process units  | 2 each                                           | 1–256 each                        |
| Pending capacity                      | 1024                                             | 1–100000                          |
| Dispatch window                       | 64                                               | 1–Pending capacity                |
| Active worker slots                   | 64                                               | 1–256                             |
| Successful bypasses before fencing    | 8                                                | 1–10000                           |
| CPU backlog                           | 64                                               | 1–1024                            |
| Attach acknowledgement deadline       | 5 s                                              | greater than zero, at most 300 s  |
| Durable poll / storage retry interval | 500 ms                                           | greater than zero, at most 300 s  |
| Managed cleanup deadline              | 7 s                                              | greater than zero, at most 300 s  |
| Overall job shutdown deadline         | 10 s, capped by caller's deadline                | greater than zero, at most 300 s  |
| Parsed subtitle byte limit            | 10 MiB                                           | 1 byte–100 MiB                    |
| Subtitle process execution deadline   | 60 s                                             | greater than zero, at most 1800 s |
| Subtitle process stop deadline        | 7 s                                              | greater than zero, at most 300 s  |
| Captured bytes per output pipe        | 4 MiB                                            | 1 byte–16 MiB                     |
| Parent exit poll interval             | 25 ms                                            | greater than zero, at most 1 s    |

Process limits use validated `ProcessLimits` on the subtitle adapter. Queue and
process timeouts are independent; a cleanup deadline can return attention while
the process adapter continues its owned stop protocol. No new technical budgets
or semaphores are exposed in the UI. Pending means waiting, Running means released
execution, and Cancelling includes explicitly unconfirmed cleanup.

Resource units are scheduler reservations, not OS CPU affinity or a hard percentage
cap. The process unit owns an external invocation and its descendant tree. A new
threaded executor must declare its CPU footprint in its demand; the process count
alone never authorizes unrestricted CPU work.

Evidence and the scoped validation record are in
[the fixture comparison](../evaluation/job-queue-fixtures.md).
