# Maintainability review and agent assignments

Reviewed on 17 October 2026. The scope is documentation and behavior-preserving
internal organization in Auralis and Auralis Translate. The assignments below
are prompts for later execution; they do not start agents, create branches,
commit changes, run models or qualify a release.

Read the [review](REVIEW.md) and [agent contract](AGENT_CONTRACT.md) first.
The sibling Translate checkout has a separate `docs/maintenance/README.md` with
five assignments. Only its own agents may edit the independent library.

## Assignments

| ID      | Assignment                                                                      | Unique branch                           | Execution dependency                                       |
| ------- | ------------------------------------------------------------------------------- | --------------------------------------- | ---------------------------------------------------------- |
| AUR-M01 | [Current architecture and developer onboarding](agents/01-documentation.md)     | `docs/auralis-current-documentation`    | Initial pass first; final pass after accepted code changes |
| AUR-M02 | [Translation integration documentation](agents/02-translation-documentation.md) | `docs/auralis-translation-guide`        | Independent of AUR-M01 within its owned subtree            |
| AUR-M03 | [Translate executor responsibilities](agents/03-translation-executor.md)        | `refactor/auralis-translation-executor` | Model-free baseline; preserve existing Translate pin       |
| AUR-M04 | [Project list state and rendering](agents/04-project-list.md)                   | `refactor/auralis-project-list`         | Independent frontend slice                                 |
| AUR-M05 | [Native acceptance harness responsibilities](agents/05-native-harness.md)       | `refactor/auralis-native-harness`       | Owns its new check tasks; do not share root Taskfile edits |
| AUR-M06 | [Committed candidate and Translate pin integration](agents/06-integration.md)   | `chore/auralis-maintenance-integration` | After reviewed Auralis/Translate candidates                |

## Coordination

Give every implementation agent an isolated worktree and a recorded base that
contains this packet. AUR-M01 owns the root documentation index and common guides;
AUR-M02 owns translation documentation. They exchange requested index changes
rather than editing the same file. AUR-M05 alone adds its root Taskfile checks.
Each agent owns its acceptance report.

Code assignments AUR-M03, AUR-M04 and AUR-M05 can run independently. Review and
integrate one branch at a time, rerunning affected checks after conflict resolution.
AUR-M01/AUR-M02 then reconcile documentation with accepted code.

AUR-M06 is the only assignment allowed to advance the Translate gitlink. Accept
the standalone Translate candidate first, then verify the embedded candidate
against Auralis's locked dependency graph. Never merge through the nested checkout
or assume that sibling and embedded Translate HEADs match. A documentation-only
Translate change does not require a pin update.

The receiving reviewer checks committed code and the assignment's acceptance
matrix before recording completion. Planning files and proposed branch names are
not evidence of implemented work.
