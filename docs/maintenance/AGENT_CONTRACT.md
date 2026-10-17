# Maintenance agent contract

## Objective

Improve the assigned documentation or internal structure while preserving
observable behavior. Read the current repository instructions, documentation
index, relevant architecture, taskfiles and assignment before editing. Preserve
all user-provided instructions even if there is no root `AGENTS.md` yet.

Only Auralis and Auralis Translate may change. Do not edit CASTER, CONTRACT, TTS,
MARKER or original MCE-2 directories. Those components may be described from the
Auralis integration boundary without rewriting their manuals. Use fixtures and
owned temporary application data for checks; preserve personal databases/media.

Do not change public Rust/IPC APIs, command names, serialization, error codes or
messages, UI text, keyboard/focus behavior, database schema, retry/timeout defaults,
process/resource ownership, queue fairness, SQL transaction ordering or model
profiles. A probable bug gets a retained reproduction and a separate proposed
fix. Do not slip functional corrections into mechanical refactoring.

Prefer private operations and existing ports. Introduce a pattern only when it
clarifies a real responsibility. Do not add frameworks, a generic job engine,
new public traits for one caller or private-test-only public APIs.

## Branches and ownership

Use the unique branch in your assignment in an isolated managed worktree. Record
base HEAD, current branch, status, remotes and existing diffs. The coordinator
supplies a base containing this packet. Never reset an existing branch, switch
another agent's checkout, stage unrelated changes or edit another agent's paths.

The independent Translate checkout is the library source owner. Do not edit its
nested copy inside Auralis. AUR-M06 alone advances the gitlink after the standalone
candidate is accepted. Documentation-only library changes do not require a pin.
Coordinate shared Taskfile/index changes through their named owner.

## Reviewable commits

Aim for 100–300 non-move changed lines per coherent commit; split before 500
non-move lines or about 1,000 raw added/deleted lines. These are budgets, not
minimums. Keep mechanical moves separate from renaming, formatting and behavior
changes. Record ordinary and rename-aware diff statistics. Explain generated
output churn without manually splitting inconsistent generated artifacts.

Use the computer's primary global Git identity for both author and committer.
Inspect global name/email, local/environment overrides and `git var` identities.
Never invent a bot/noreply account or change global configuration. If overrides
interfere, set the four `GIT_AUTHOR_*`/`GIT_COMMITTER_*` identity variables only
for the commit process and restore them afterward. Resolve an absent/ambiguous
primary identity before committing. Do not rewrite anyone else's authorship.

Write English `type(scope): short summary` commits. Without an issue a body is
required: what changed, why, and preserved behavior. Issue subjects use only
`(#number)`; no issue-closing keywords in commits. Review explicitly staged paths.
After successful checks, commit and push only your branch to the verified
repository remote; record an unavailable remote rather than inventing one.
Do not force-push, merge main or publish a site implicitly. Requested
PRs use the session template and default technical title and must be attached.

## Verification and reporting

1. Record baseline focused checks and a behavior matrix. Distinguish failing code
   from environment/process restrictions and unavailable assets.
2. Extract one responsibility per step. Add only meaningful missing contract
   coverage. Use existing `task ...` commands; add a task before any new check.
3. Run focused and required final gates. Do not weaken limits or skip assertions
   to pass. Keep normal/native/model evidence separate. Run the specifically
   assigned native fixture scenario only against its isolated data root.
4. Commit, verify the candidate, and record what was actually tested. A docs-only
   evidence commit may reference the previously tested source commit; never
   invent a future SHA or claim unrun acceptance.

Save `docs/maintenance/reports/<ID>.md` with repository/branch/base/candidate,
responsibility map, owned paths, before/after behavior matrix, commit hashes and
line counts, exact task commands/exit codes/results, ignored checks, retained
failures, API/schema/config changes, generated diffs, limitations and push/PR
status if performed. Explain what the receiving reviewer must check next.

The reviewer reads the complete diff, compares public behavior, checks negative
paths and reruns focused checks on committed code. Self-report and compilation
alone are insufficient. Record `accepted`, `changes_requested` or
`blocked_by_environment`, with any narrower accepted scope. Missing native/model
or installation evidence stays open and never becomes a production-quality claim.
