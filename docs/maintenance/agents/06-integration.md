# Auralis maintenance integration assignment

Execute `AUR-M06` on `chore/auralis-maintenance-integration` after the selected
local assignments and standalone Translate candidate are reviewed.
Read the [agent contract](../AGENT_CONTRACT.md). Own the final maintenance
acceptance report, necessary conflict resolution, coordinated documentation
reconciliation and the Translate gitlink/Cargo lockfile only if required.
Do not make another architecture refactor or silently upgrade dependencies.

## Sequence

1. Collect each report and full branch diff. Check base/candidate/commit identities,
   ownership, commit sizes, actual task results and before/after behavior. Return
   concrete `changes_requested` items where evidence or invariants are missing.
2. Integrate one accepted branch at a time and rerun affected checks after conflict
   resolution. Do not accept an agent's summary as proof that its code is sound.
3. Compare the old embedded Translate commit with the accepted candidate. Review
   the entire pin delta, including pre-existing sibling commits unrelated to this
   maintenance effort. Do not smuggle that delta through a maintenance pin bump.
   If unrelated changes are not accepted, use a reviewed maintenance-only library
   line based on the embedded pin or leave the pin unchanged with a precise gap.
4. When a library pin update is justified, ensure its commit is available from the
   verified configured source before the host commit references it. Validate the
   host's locked dependency resolution and review any lockfile delta separately.
   Keep the pin update a small separate commit. A docs-only change needs no bump.
5. Have the documentation owners reconcile against the actual accepted tree.
   Verify the committed candidate again after any conflict or implementation edit.

## Acceptance

Every integrated assignment has a reviewer verdict and linked acceptance record.
The gitlink resolves to the reviewed library source and carries no unreviewed
functional change. No production API, UI, SQL/schema, profile or behavior change
is introduced. Documentation agrees with that exact candidate and preserves open
release gates. All other repositories remain untouched.

Run Translate `task check`, `task docs:check`, `task plan:check` and affected
report checks at its accepted source revision. In Auralis run `task check:pr`,
`task rs:test:translate` and `task check:translation-observer`; run the affected
native fixture checks from AUR-M05 when that branch is included. Existing ignored
asset checks and missing browser/security/native prerequisites must be reported,
not counted as passes. Do not install dependencies or start model experiments
merely to hide an environment blocker.

Submit a final report listing both committed candidates, the embedded pin,
per-assignment verdicts, exact final checks, remaining limitations and a rollback
commit sequence. State the narrower completed scope if a required gate remains
open. No release, production language claim or remote site deployment is implied.
