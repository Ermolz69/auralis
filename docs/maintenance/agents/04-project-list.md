# Auralis project list assignment

Execute `AUR-M04` on `refactor/auralis-project-list`.
Read the [agent contract](../AGENT_CONTRACT.md). Own
`apps/desktop/src/features/project-list/` and your report. Do not alter shared
IPC, project/job entity synchronization, navigation contracts or global UI styles.
Avoid editing the file-size ratchet policy; the existing exception can remain
until a later explicit policy cleanup.

## Sequence

1. Read the component, update hook, deletion dialog and tests. Map request
   generations, project revisions, current-project refs, deletion ownership,
   notifications, navigation and focus restoration.
2. Run current component tests. Add meaningful missing cases for stale fetches,
   deletion races, NOT_FOUND, BUSY/CONFLICT refresh, selection cleanup, preference
   cleanup warnings and focus after success/cancel/failure.
3. Move data/actions and focus restoration into cohesive private hooks under
   the same feature. Keep `ProjectList` responsible for composition. Preserve
   React effect timing and generation/ref behavior, not just the final markup.
4. Keep API calls, translated/user-visible strings, toasts and keyboard behavior
   identical. Use existing lower-layer ports; do not add context/store frameworks.

## Acceptance

The component fits its normal feature limit without moving an equally mixed body
to another file. Existing test behavior and accessible UI remain unchanged. Late
responses cannot restore a deleted project or override a newer revision. Current
project deletion clears selection/navigation as before. Focus returns to the same
control, next project or empty-list heading in each existing outcome. No extra IPC
request, repeated toast or event subscription is introduced.

Run `task fe:test:components -- src/features/project-list`,
`task fe:typecheck`, `task fe:lint`, `task q:fsd-boundaries`,
`task q:file-size`, `task q:duplicate-code`, `task fe:e2e` and `task fe:build`.
Use existing browser fixtures; do not launch the personal desktop workspace.
Report browser availability explicitly. Submit the common report with event/
request/focus comparisons, not only rendered snapshots. Keep hook extraction and
component cleanup as separate small commits.
