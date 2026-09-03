# Review Completion Audit

This checklist rechecks the findings against `e032778` while integrating the
previously unmerged UI branch with the corrections already in main. A merge must
preserve the newer concurrency, avatar-storage, and atomic-selection behavior.

## Data integrity and UI consistency

| Original finding                                     | Resolution and verification                                                                                                                                                                                                                                             |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Rename overwrites concurrent pipeline changes        | Partial repository updates use a revision predicate. The deterministic `rename_conflicts_with_pipeline_then_retry_only_changes_title` test preserves status, source, transcript, and job state.                                                                         |
| Source import and final writes use stale snapshots   | Source import uses a revision-guarded update; pipeline, transcript, and YouTube UoW writes also enforce revision/state conditions. SQLite tests cover conflicts even with identical timestamps.                                                                         |
| YouTube cleanup invents ownership keys               | The allocation's typed key reaches the staged write and `DeleteWorkspaceAllocation` outbox payload. Old serialized payloads remain readable without reconstructing ownership from filenames.                                                                            |
| Missing filename extension becomes `.bin`            | Real artifact-store tests cover source-extension fallback with `original` hints for owned and external files.                                                                                                                                                           |
| YouTube import is not failure-atomic                 | Download/staging precedes persistent mutation. One SQLite transaction commits project, source, artifact, and both outbox writes; revision/status are rechecked. Injected source/artifact/outbox failures and rename/delete races preserve prior state and permit retry. |
| Open/delete folder race                              | Open and delete share lifecycle locks. Tests cover both interleavings, project isolation, and lock release on errors. The adapter may create a missing folder only after the locked existence check.                                                                    |
| localStorage failure reverses successful UI commands | Backend result handling is separate from best-effort preferences. Delete clears selection before preferences cleanup; rename publishes a dedicated change event. Quota/security exceptions have regression coverage.                                                    |
| Avatar data overloads localStorage                   | SQLite-backed avatar APIs and conditional legacy migration remain in use. The older branch's image validation/normalization feeds the backend, not preferences.                                                                                                         |
| Selection has an ID without a project                | A discriminated selection derives both values. NOT_FOUND closes it and invalidates operations. Tests verify late replies cannot reopen it and navigation returns home, including from settings.                                                                         |

The YouTube implementation deliberately chooses the transactional alternative:
there is no persistent project to compensate before a new import commits. Retry
restarts the import from unchanged state. Persistent downloading states and
byte-range download resumption are not implemented or claimed by these tests.

## CI, security, and performance

| Original finding                      | Resolution and verification                                                                                                                                                          |
| ------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Release environment differs from CI   | Shared composite bootstrap and reusable full-check workflow install Chromium and Linux Tauri/GTK prerequisites. Wiring has regression tests.                                         |
| README omits Playwright               | README documents local Chromium installation and Linux system dependencies.                                                                                                          |
| Frontend/docs installers require Rust | `install:frontend`, `install:rust`, and `install:all` are separate; CI jobs select only their required components.                                                                   |
| Browserslist HIGH advisories          | The existing 4.28.7 override and lockfile remain unchanged; npm audit is rerun.                                                                                                      |
| Rust dependency auditing absent       | **Pending security PR #64**: pinned cargo-audit/cargo-deny and CI wiring exist, but unresolved rsa/glib findings require remediation or an explicit reviewed exception before merge. |
| Oversized frontend bundle             | Named icon imports, vendor splitting, and lazy routes reduce initial JS. Build reports and enforced raw/gzip budgets include transitive initial dependencies and all lazy chunks.    |
| CSP disabled                          | Explicit production/development policies are covered by configuration tests and a Chromium test of the actual production output.                                                     |

See [bundle and CSP checks](../ci/011-desktop-bundle-and-csp.md) for measurements,
limits, and the boundary between browser smoke coverage and native desktop testing.

## Secondary technical debt

| Finding                                  | Resolution                                                                                                                                                                                                                 |
| ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Queue disappears on project switch       | App-lifetime global store calls `listJobs`; project selectors isolate local views while header/drawer retain global activity. Tests cover foreign/unattached jobs, buffered events, revision gaps, and StrictMode cleanup. |
| Pipeline ignores job kind                | Job kind is carried through domain, scheduler, DTO, events, and frontend contracts. Pipeline selectors map supported kinds to steps and order attempts deterministically.                                                  |
| Keyboard resize lacks maximum            | Pointer and keyboard resize use the same upper/lower bounds after component extraction.                                                                                                                                    |
| Empty titles accepted by domain          | `ProjectTitle` validates creation, rename, and snapshot restoration; `Project::new` returns a result.                                                                                                                      |
| Production blanket panic allowances      | Removed from production modules. Remaining unwrap/expect allowances are scoped to test modules and test support. Workspace Clippy remains warning-denying.                                                                 |
| JobContext triggers Fast Refresh warning | Context lives in its own `context.ts` and remains part of the entity public API.                                                                                                                                           |
| Boundaries selector syntax deprecated    | The existing v6 `boundaries/dependencies` rules use object selectors; positive and negative rule fixtures are rerun.                                                                                                       |

## Validation and limitations

Use `task check` for the full integrated suite and `task fe:storybook` for the
additional Storybook build. Production Chromium smoke tests run through
`task fe:smoke` with an explicitly mocked IPC boundary; backend behavior is
covered independently by the real SQLite and Rust tests. No real YouTube download,
native OS-folder launch, or signed desktop release is implied by these checks.

The security PR must not be described as finished merely because the other
branches merge. Its blockers are [RUSTSEC-2023-0071](https://rustsec.org/advisories/RUSTSEC-2023-0071)
and [RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429).
