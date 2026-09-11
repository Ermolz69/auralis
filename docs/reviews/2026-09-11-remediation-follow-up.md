# Data safety remediation follow-up

This patch addresses R01–R06 from the review of `71cf1ad7ac5c743a307181f6fe1d3bd0e9141dc3`.
It preserves SQLite authority, ready-only public artifact queries, transactional outbox writes,
external-file ownership and explicit retries. Observability remains a module in the composition root.

## Corrections

- **R01:** `ArtifactFinalizationLookup` reads project ownership and pending/ready metadata in one
  SQLite read transaction. A deleted project, missing artifact, invalid metadata and genuinely unknown
  size are distinct outcomes. The worker verifies state and final key before passing the stored size
  to the filesystem. The integration test uses the real SQLite repository/UoW, filesystem and worker:
  truncated staging, wrong existing final, missing artifact, wrong key, valid staging/final, unknown
  size, replay and project deletion. Invalid bytes remain unready, with retryable intent retained.
- **R02:** external import streams into a newly created, owned writable staging file instead of
  inheriting the original's permissions. The output handle is flushed and synchronized; the usual
  finalization synchronization remains. The regression preserves source bytes and permissions on
  Windows and Unix, including mode `0444`.
- **R03:** pin persistence owns saving/error state as well as desired/confirmed values and ordered
  writes. Rows and the sidebar subscribe to that owner. Failed intent and Retry survive remount,
  including failures arriving while unmounted. Repeated toggles are queued rather than ignored.
  Native snapshots are accepted before legacy migration, and shared load failure is visible and
  retryable without writing defaults. Deletion invalidates queued writes and late acknowledgements.
- **R04:** every command emits one safe completion event with operation, result and elapsed time
  inside its request span. Claimed outbox attempts emit durable message ID, attempt, result, elapsed
  time and a stable error category, including failed acknowledgement/retry persistence. Tests capture
  JSON and compact output at INFO for real SQLite rename success/conflict and failed finalization.
  No title, source path, request body or raw error enters these events.
- **R05:** production global error/rejection listeners record bounded allowlisted metadata and cancel
  default browser reporting for cancelable events. Debug builds keep default reporting. A production
  Chromium scenario creates an actual thrown Error and rejected Promise with a synthetic marker and
  checks both console and page-error channels. Listener disposal and rate limiting remain covered.
- **R06:** the tracing guard owns file and console workers and the stop/join lifecycle of the health
  sampler, including console-only mode. Shutdown stops the sampler before sinks and shares one
  absolute deadline. Per-resource reports distinguish flushed, timed out, failed and not owned.
  Completion is acknowledged by the actual underlying writer, not merely by WorkerGuard drop
  returning after its own internal timeout. A zero-progress write is recorded as an I/O failure and
  cannot report Flushed. The health sampler includes underlying console writer failures, and shutdown
  diagnostics distinguish failure from timeout. A child-process test covers normal console tail,
  queue overflow, blocked console shutdown and sampler termination.

The recovery panel additionally refreshes while continuously visible. Its retry acknowledgement is
worded as a historical request, not proof that finalization has already completed.

## Validation

The final verification results and code commit are reported with the handoff. The relevant tasks are
`task check:rust:pr`, `task check:frontend`, `task desktop:e2e:native`, `task check:docs`, and
`task check:quality:global`. The Unix read-only regression uses
`task rs:exec:wsl -- test --locked -p adapters-storage readable_readonly` in Ubuntu WSL.
Focused tests during implementation do not substitute for gates on the final code tree.

## Limits

File length does not detect same-length corruption. Windows directory fsync, power-loss testing,
backup/restore, performance measurements and a separate observability crate remain outside this
patch. The browser privacy scenario is Chromium evidence, not certification of every WebView engine
or developer-tools/debugger channel. Native acceptance covers media/import/cancellation, not all
preference multi-window/restart cases. Logs remain bounded and lossy; a timeout never proves a
blocked task or writer stopped, and no flush is promised after forced termination.
