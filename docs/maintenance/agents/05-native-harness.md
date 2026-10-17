# Auralis native acceptance harness assignment

Execute `AUR-M05` on `refactor/auralis-native-harness`.
Read the [agent contract](../AGENT_CONTRACT.md). Own
`tools/native-e2e/run.mjs`, extracted private harness modules, focused tests and
the minimal root Taskfile entrypoints for those tests. Coordinate command docs
through AUR-M01. Do not change production Rust/frontend behavior, test-only fault
locations, model profiles or scenario acceptance thresholds.

## Sequence

1. Inventory supported environment flags, scenario combinations, fixture server,
   build/launch, wait/kill/restart, SQLite verification and cleanup responsibilities.
   Retain the exact before/after mapping for every supported scenario.
2. Add a Taskfile command before introducing a new fixture-only harness contract
   check. Exercise scenario selection and cleanup failure with injected owned
   process/filesystem adapters; do not invent success from a fake native run.
3. Extract scenario configuration, fixture setup and app lifecycle/verification
   into cohesive modules in separate commits. Preserve `runNativeE2e` entrypoint
   and invocation working directory. Avoid a generic scenario/plugin framework.
4. Run the existing model-free native fixture scenario against isolated temporary
   data. Translation-specific native/model scenarios are separate acceptance
   items when their configured assets and bounded run scope are available.

## Acceptance

All flags select the same scenario and reject the same invalid combinations.
Child exit and descendant cleanup are still verified. Partial logs, failure
evidence, keep-temp behavior and temporary-root checks survive success, timeout,
failure and interrupted publication. Expected SQLite/file counts and hashes,
fault timing and result verification are unchanged. Tests do not weaken the very
assertions they move.

Run the new harness contract task,
`task desktop:e2e:native:preparation:check`,
`task desktop:e2e:native:translation:history:export:check`,
`task desktop:e2e:native`, `task check:docs` and `task check:quality:docs`.
Document any missing native prerequisites; a contract-only pass leaves the native
item open. Submit the common report with scenario coverage, cleanup fault matrix,
exact observed native result and unchanged assertion inventory. Do not claim all
translation/native scenarios passed from one model-free invocation.
