# AUR-M01 documentation acceptance record

## Candidate and ownership

- Repository: Auralis; branch: `docs/auralis-current-documentation` in isolated
  worktree `.cache/worktrees/auralis-current-documentation`.
- Original checkout: `perf/translation-observer-reads` at
  `321be7f7c8914ea3bea9b9d43256072266680015`, with an untracked
  `docs/maintenance/` packet and no tracked diff. Its state was left intact.
- Branch base containing the packet: `43136f3` (created from `321be7f`);
  tested documentation source candidate: `65f76b8`.
- Remote: `origin`, `git@github.com-personal:Ermolz69/auralis.git`.
  The global author/committer identity was `Ermolz <00ermzahar@gmail.com>`;
  no local or environment override was present, and both `git var` identities
  matched. No global configuration was changed.
- Owned edits: root `AGENTS.md` and `README.md`, `apps/desktop/README.md`,
  `docs/README.md`, architecture `000`, `001`, `003`, `006`, the production
  storage contract, CI quality guide, Taskfile guide, integration index and
  integration architecture, and this report. The user-supplied packet was
  copied and committed unchanged as the recorded base. No code, component
  repository, nested Translate checkout, or gitlink changed.

## Complete English-document inventory at the tested candidate

Classification records the function of each file, including guides left
unchanged. Current contracts can cite dated evidence; a dated record is not
automatically a current release claim.

| Class                                                    | Files                                                                                                                                                                                                                                                                                                                                                                                         |
| -------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Entry and agent rules                                    | `README.md`, `AGENTS.md`, `.agents/AGENTS.md`, `apps/desktop/README.md`, `docs/README.md`                                                                                                                                                                                                                                                                                                     |
| Active architecture                                      | `docs/architecture/000-stack.md`, `001-overview.md`, `002-frontend-fsd.md`, `003-rust-workspace.md`, `004-design-system.md`, `005-storybook-conventions.md`, `006-runtime-data.md`, `007-job-runtime.md`, `008-job-dispatch.md`, `009-outbox-concurrency.md` (all under `docs/architecture/`)                                                                                                 |
| Dated implementation contract                            | `docs/architecture/008-translation-review-decision.md`                                                                                                                                                                                                                                                                                                                                        |
| Storage, task, media and release guides                  | `docs/storage/production-storage.md`, `docs/taskfile/001-commands.md`, `docs/media-tools.md`, `docs/release/001-release-workflow.md`, `docs/release/002-signing.md`                                                                                                                                                                                                                           |
| CI and repository policies                               | `docs/ci/001-quality-gates.md`, `002-node-audit.md`, `002-runner-images.md`, `003-formatting.md`, `004-markdownlint.md`, `005-fsd-boundaries.md`, `006-deep-imports.md`, `007-theme-tokens.md`, `008-file-size-limits.md`, `009-path-filtering.md`, `010-rust-security.md`, `011-desktop-bundle-and-csp.md`, `012-glib-backport.md`, `013-sqlite-only-dependencies.md` (all under `docs/ci/`) |
| Integration design and navigation                        | `docs/integration/README.md`, `architecture.md`, `boundary-contract.md` (the last two are dated design requirements)                                                                                                                                                                                                                                                                          |
| Proposed integration goals                               | `docs/integration/goals/common.md`, `00-contract.md`, `01-translate.md`, `02-tts.md`, `03-caster.md`, `04-assembly.md`                                                                                                                                                                                                                                                                        |
| Dated integration evidence                               | `docs/integration/caster-execution.md`, `tts-execution.md`                                                                                                                                                                                                                                                                                                                                    |
| Mixed current guide and dated evidence, owned by AUR-M02 | `docs/translation/README.md`                                                                                                                                                                                                                                                                                                                                                                  |
| Dated evaluation and performance evidence                | `docs/evaluation/job-queue-fixtures.md`, `docs/performance/translation-observer-reads.md`                                                                                                                                                                                                                                                                                                     |
| Maintenance plan and contract                            | `docs/maintenance/README.md`, `REVIEW.md`, `AGENT_CONTRACT.md`, `agents/01-documentation.md`, `02-translation-documentation.md`, `03-translation-executor.md`, `04-project-list.md`, `05-native-harness.md`, `06-integration.md` (agent files under `docs/maintenance/agents/`)                                                                                                               |
| New execution evidence                                   | `docs/maintenance/reports/AUR-M01.md`                                                                                                                                                                                                                                                                                                                                                         |

No generated Markdown reference was found in this inventory. Storybook static
output and other generated reports are untracked build artifacts or external
publications, not current source guides. The documentation link checker scans
the two READMEs and `docs/`; root `AGENTS.md` and `.agents/AGENTS.md` are covered
by Markdown lint/formatting but not its relative-link inventory.

## Responsibility and claim corrections

| Before                                                                      | After and source                                                                                                                                                                                                                                                                                                                             |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Root and stack guides described TTS/translation only as planned.            | Root, stack and overview distinguish experimental Translate jobs, optional Caster operations, the installed TTS catalog, and incomplete synthesis/publication. Sources: `src-tauri/src/lib.rs`, `src-tauri/src/bootstrap/{caster,tts,translation}.rs`, `docs/integration/{caster-execution,tts-execution}.md`, `docs/translation/README.md`. |
| Rust crate map omitted two Cargo members.                                   | `docs/architecture/003-rust-workspace.md` now includes `adapters-caster`, `adapters-tts` and the GLib regression member; compared with root `Cargo.toml` and bootstrap composition.                                                                                                                                                          |
| Overview/index did not route readers to the durable queue.                  | Overview, index, agent entrypoint and storage guide link `docs/architecture/008-job-dispatch.md`, `007-job-runtime.md` and `009-outbox-concurrency.md`; sources include `src-tauri/src/bootstrap/services.rs`, `bootstrap/mod.rs`, and those current contracts.                                                                              |
| Runtime layout implied one database; storage opening emphasized version 13. | Runtime layout shows `auralis-translate.sqlite` and optional component configuration from `src-tauri/src/bootstrap/paths.rs`; storage guide identifies current schema 14, supported by `crates/adapters-storage/src/sqlite/connection/validation.rs`.                                                                                        |
| Integration architecture's 10 October observations read as current state.   | It labels the inspected hashes as a dated starting point; the integration index maps current composed boundaries to execution records. Goal files and unaccepted model/native gates remain plans.                                                                                                                                            |
| Contributor setup and checks were scattered.                                | Root and desktop READMEs plus root `AGENTS.md` give entry paths for frontend/backend ownership, IPC, storage and fixture checks. Command names were matched against `Taskfile.yml` and `taskfiles/{frontend,rust,quality,docs}.yml`; no setup command was executed merely to validate an example.                                            |

## Before and after behavior matrix

| Boundary                           | Before                                                                     | After                                                                                         |
| ---------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------- |
| Rust, frontend and IPC behavior    | Existing code and command contracts                                        | Unchanged; no source or generated-code diff                                                   |
| SQLite schema and managed files    | Existing version 14 database and outbox                                    | Unchanged; documentation now names both databases and queue/outbox ownership                  |
| Optional model and worker runtimes | Existing explicit installation and model gates                             | Unchanged; guide labels Caster/TTS catalogs, experimental Translate, and incomplete synthesis |
| Reader navigation                  | Queue and execution evidence were hard to locate; no root agent entrypoint | Current contracts, dated evidence, setup and focused checks are linked or named               |

## Commits and change size

- `43136f3` packet base: 9 files, 451 added lines; user-provided assignment
  material only, no implementation claim.
- `5483a6f` onboarding: 3 files, 94 added / 12 deleted lines.
- `b079a22` architecture and storage: 5 files, 66 added / 8 deleted lines.
- `f891833` integration navigation: 5 files, 62 added / 10 deleted lines.
- `65f76b8` onboarding follow-up: 1 file, 3 added lines.
- From packet base to tested source: 13 files, 225 added / 30 deleted lines;
  ordinary and `--find-renames` diff statistics agree, with no moves or renames.

## Verification and limits

| Command                   | Observed result                                                                                                                                                                      |
| ------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `task check:docs`         | Exit 0 with this report in the `65f76b8` worktree; five validator tests passed, 57 indexed Markdown files and 11 mandatory files valid, Markdown lint found zero errors in 59 files. |
| `task check:quality:docs` | Exit 0 with this report in the `65f76b8` worktree; all matched Markdown used Prettier style.                                                                                         |
| `task docs:integration`   | Exit 0 with this report in the `65f76b8` worktree; five validator tests, 12 integration/index lint targets and formatting passed.                                                    |

Formatting used `task q:format-write --` with these exact path sets, each with
exit 0 and no unrelated files retained:

```text
README.md AGENTS.md apps/desktop/README.md docs/README.md docs/architecture/000-stack.md docs/architecture/001-overview.md docs/architecture/003-rust-workspace.md docs/architecture/006-runtime-data.md docs/ci/001-quality-gates.md docs/integration/README.md docs/integration/architecture.md docs/storage/production-storage.md docs/taskfile/001-commands.md
docs/integration/README.md docs/architecture/003-rust-workspace.md
docs/integration/README.md
docs/maintenance/reports/AUR-M01.md
```

An initial restricted `task check:docs` failed with Node `spawn EPERM` before
the documentation assertions ran. Repeating it with local child-process access
passed. The first post-edit link run found root `AGENTS.md` outside the link
checker's inventory; README links were changed to code-path mentions, and the
final gate passed. A preliminary `task docs:integration` hit the same `EPERM`
restriction; the authorized rerun passed. A preliminary format check found the
edited integration table; the existing format task corrected it before commit.

No Rust/frontend behavior test, native fixture, model, listening, clean-install
or release gate was run for this documentation-only candidate. No tests were
ignored to create a passing result. API, schema, configuration, generated output
and product behavior changes: none. No external module documentation changed.

The AUR-M01 final reconciliation remains open until AUR-M02 and accepted code
branches supply their final candidates and requested index links. The receiving
reviewer should read the complete diff, compare capability statements with the
named bootstrap/adapter sources and dated execution records, rerun the three
documentation gates on the committed candidate, and record `accepted`,
`changes_requested` or `blocked_by_environment`. The packet and this self-report
do not constitute reviewer acceptance.

Push and PR status: source candidate `65f76b8` was pushed to
`origin/docs/auralis-current-documentation`; this report commit will follow.
No PR was created.
