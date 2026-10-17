# Auralis translation executor assignment

Execute `AUR-M03` on `refactor/auralis-translation-executor`.
Read the [agent contract](../AGENT_CONTRACT.md). Own
`crates/adapters-translate/src/run_executor.rs`, its extracted private modules,
minimal crate declarations and focused adapter tests. Preserve the Translate
gitlink and all application/ports/storage contracts. Do not share code by moving
host-specific logic into the independent library.

## Sequence

1. Map `execute_run`: frozen link/intent IDs, language/profile/policy, bounded
   source/plan, Translate source/run checks, validated replay, start guard,
   server preflight, attempt execution/stop and immutable result commit.
2. Reproduce current integration tests and add missing public-behavior coverage
   for the boundaries being moved, using owned SQLite files and mock HTTP.
3. Extract private frozen-input/source verification and attempt lifecycle owners.
   Move progress forwarding to its own owner only if it improves navigation.
   Keep the public async executor facade and `spawn_blocking` boundary intact.
4. Preserve validation order and exact errors. Keep the `Validated` return path
   free of new inference, the `Running` busy path and stale guard cancellation.
   Do not add a general workflow/strategy framework or change SQL/schema policy.

## Acceptance

The public `TranslationExecutionPort` implementation is unchanged. Each base
case returns the same result/error and writes the same run/attempt/checkpoint
states: source/profile mismatch, invalid policy, already validated/running,
preflight failure, paused admission, mid-request pause, execution failure and
result commit failure. Only committed progress is forwarded. Original bytes and
existing results remain unchanged; no partial result becomes publishable.

Run `task rs:test:translate`, `task rs:test:application`,
`task rs:test:translation-events`, `task rs:fmt`, `task rs:clippy`,
`task q:file-size`, `task q:ipc-contract` and `task check:docs`.
Use existing focused test arguments during each small step. No real model/native
claim is renewed by mock checks. Submit the common report with ordered lifecycle
and error matrices; coordinate documentation corrections through AUR-M02.
