# Auralis architecture and onboarding assignment

Execute `AUR-M01` on `docs/auralis-current-documentation`.
Read the [agent contract](../AGENT_CONTRACT.md) and [review](../REVIEW.md).
Own root `README.md`, a concise new root `AGENTS.md`, `apps/desktop/README.md`,
the docs index and active architecture/storage/CI/task/release/integration guides.
`docs/translation/` belongs to AUR-M02. No component repository may change.

## Sequence

1. Inventory all active English guides and classify current contracts, setup,
   plans, dated execution evidence and generated references. Check Cargo members,
   frontend slices, bootstrap registration, Taskfile commands and actual tests.
2. Update the current Rust adapter map for Caster/TTS and the actual desktop
   catalogs. Keep incomplete synthesis/publication/assembly gates explicit. Label
   the integration architecture's starting point and keep its historical facts.
3. Link durable dispatch, resource ownership, cancellation and outbox contracts
   from the index and relevant guides. Document existing mechanisms; do not add
   a competing queue or generic integration architecture.
4. Provide short reader paths for setup, first check, frontend/backend ownership,
   IPC additions, storage changes and fixture/native acceptance. Add factual agent
   instructions that link to those paths and use current Taskfile conventions.
   Preserve supplied user rules; do not invent new approval requirements.
5. Reconcile against accepted code branches at the end and incorporate index
   links requested by AUR-M02. Do not hand-copy changing task/test totals.

## Acceptance

A new developer can identify what runs today, find a suitable local check and
understand the mock/model/native distinction. All Cargo adapters and active
composition boundaries are represented. Every current capability claim has a
source or acceptance reference; plans are not described as implemented. External
module docs, immutable evidence and product behavior are unchanged.

Run `task check:docs`, `task check:quality:docs` and `task docs:integration`.
Verify documented commands against actual definitions. Setup examples must not
silently install software, download models or change user state during validation.
Use separate onboarding, architecture/status and integration-navigation commits.
Submit the common report with the complete document inventory and claim/source
corrections; green links alone do not prove factual consistency.
