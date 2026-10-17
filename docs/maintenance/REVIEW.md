# Maintainability findings

## Inspected state and scope

Auralis was clean on `main` at
`321be7f7c8914ea3bea9b9d43256072266680015`. Its Translate gitlink is
`44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`; the independent sibling checkout
is at `5749fb72b43569c313410fb1422892272e852432`. Treat them as different
candidates and reinspect all bases at implementation time.

The observed pin-to-sibling delta contains 20 commits and 58 changed files
(5,296 insertions, 148 deletions). A wholesale pin update would include earlier
work beyond this maintenance request. AUR-M06 must review that delta or use a
maintenance-only line based on the embedded pin; it must not advance it casually.

The review surveys both repositories and inspects selected implementation owners,
contracts and existing checks. It does not claim an exhaustive functional,
concurrency, security or platform audit. No product code changes are part of this
planning packet. Other project repositories are outside the edit scope.

## Existing strengths

The Rust workspace already has domain, ports, application, jobs, diagnostic and
concrete adapter boundaries. Composition is in `src-tauri`. Application outbox
modules are already split, and the project/outbox fencing contract is detailed.
Frontend FSD imports, typed IPC, file-size limits and exact-duplicate checks are
enforced. These are reasons to retain the architecture and improve bounded
responsibilities instead of rebuilding it around new patterns.

## Documentation findings

| Priority | Source evidence                                                                                                                                                                                     | Assignment                                                                                                                                                             |
| -------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| High     | `architecture/003-rust-workspace.md` omits `adapters-caster` and `adapters-tts`, both present in Cargo members. Desktop bootstrap configures Caster operations and a TTS catalog.                   | AUR-M01: distinguish implemented adapters/catalogs from incomplete synthesis, publication and native acceptance.                                                       |
| High     | `architecture/001-overview.md` and the integration architecture describe an earlier integration starting point, while later execution records and current bootstrap expose more operations.         | AUR-M01: make current capabilities discoverable and label dated starting-point observations. Do not promote a partial TTS integration to a production speech pipeline. |
| High     | `docs/translation/README.md` is 706 lines. Early paragraphs still call event-driven progress, historical selection and same-process races pending; later sections record named acceptance for them. | AUR-M02: separate the current integration guide from dated evidence and preserve each still-open boundary.                                                             |
| Medium   | `docs/README.md` omits `architecture/008-job-dispatch.md` even though the durable queue is implemented and has a detailed contract.                                                                 | AUR-M01: link the actual queue and resource ownership read path.                                                                                                       |
| Medium   | There is no root `AGENTS.md` in this checkout. Important task, ownership, environment and commit rules are distributed across taskfiles and several guides.                                         | AUR-M01: add a short factual agent entrypoint based on existing conventions and this user request; link to guides instead of copying them.                             |
| Medium   | The integration overview still presents initial prompts beside active execution records without a concise capability map.                                                                           | AUR-M01/AUR-M02: distinguish planned component goals, current code and actual acceptance evidence.                                                                     |

All active English documentation should be inventoried, not blindly rewritten.
The current link/lint checks pass, but do not evaluate the truth of status claims.
The documentation job must reconcile current statements and preserve dated records.

## Code and tooling findings

| Assignment | Source evidence                                                                                                                          | Bounded improvement                                                                                          | Preserve                                                                                                      |
| ---------- | ---------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------- |
| AUR-M03    | `crates/adapters-translate/src/run_executor.rs`, 388 lines; `execute_run` owns most of it                                                | Separate frozen-input/source/database checks, attempt execution and progress forwarding into private owners. | Validation precedence, preflight/admission order, pause, checkpoint/result durability and PortError mapping.  |
| AUR-M04    | `apps/desktop/src/features/project-list/ui/ProjectList.tsx`, 271 lines; explicitly ratcheted by the file-size policy                     | Move data/actions and focus restoration to private hooks; keep component composition local to the feature.   | Generation/revision guards, deletion race handling, NOT_FOUND semantics, focus, notifications and navigation. |
| AUR-M05    | `tools/native-e2e/run.mjs`, 1,256 lines, with scenario flags, fixture HTTP server, build/launch/kill and SQLite verification in one file | Separate scenario configuration, fixture setup and app lifecycle/verification into named modules.            | Fault boundaries, owned process cleanup, sandbox cleanup, expected counts and all supported scenario flags.   |

Raw file length is a navigation signal, not a correctness metric. In particular,
`src-tauri/src/dto/error.rs` includes a large test block; treating its entire raw
length as oversized production code would be misleading. The repository's size
policy excludes inline test blocks. No blanket migration of all Auralis tests or
rewrite of already-split jobs/outbox logic is justified by this survey.

`JobStoreSynchronizer` and larger adapter operations deserve later targeted review
if change pressure or a concrete defect appears. Their size alone is insufficient
reason to replace stable synchronization/process logic now.

## Baseline verification

Checks before adding this packet:

| Command                                                | Observed outcome                                                                    |
| ------------------------------------------------------ | ----------------------------------------------------------------------------------- |
| `task check:docs`                                      | Passed; five checker tests, valid 47-file documentation contract and Markdown lint. |
| `task check:quality:docs`                              | Passed; Markdown formatting.                                                        |
| `task q:file-size`                                     | Passed; eight policy tests and production source scan.                              |
| `task q:duplicate-code`                                | Passed; two detector tests and frontend production scan.                            |
| `task rs:test:translate`                               | Passed; 15 tests, two asset-dependent checks ignored.                               |
| `task fe:test:components -- src/features/project-list` | Passed; four files, 47 component tests.                                             |

The first restricted `task check:docs` invocation failed because the Node test
runner could not spawn its child (`EPERM`). The same task passed with local
process access. This is an environment observation, not a repository defect.
Full workspace/browser/native/model/clean-install acceptance was not part of this
survey and must not be inferred from these focused checks.
