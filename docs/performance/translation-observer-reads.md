# Translation observer IPC measurement

## Protocol frozen before the baseline

Run `task perf:translation-observer:measure REPORT=target/translation-observer/baseline.json` before changing observer behavior. Run the same task with `REPORT=target/translation-observer/after.json` on the final implementation. The component harness mounts the production React run control or publication notice against a deterministic committed-state fixture. Each mocked IPC method takes 20 ms. It records every command method separately and measures virtual elapsed time from fixture commit until the new committed progress or publication state appears in the DOM. Virtual time avoids scheduler noise; these are controlled UI measurements, not native Tauri, SQLite, model, or total translation timings.

Each scenario runs three times over a 10,000 ms observation window. Scenarios are an empty project, one active run, four active runs, twenty rapid matching events while a read is outstanding, a committed checkpoint with its event lost, and a pending publication becoming ready. Run and publication identities and fixture timing remain identical before and after. The report must preserve every repetition and summarize absolute command counts and median DOM latency. The native checked-model event task may supply separate end-to-end evidence when its installed assets are available.

Before the final probe, protocol v2 adds three state-change scenarios at virtual time 1,000 ms: a translation starts in an initially empty project, one run commits a block, and four runs commit a block together. The original six v1 scenarios and their preimplementation baseline remain retained. The v2 baseline was run with `task perf:translation-observer:measure REPORT=target/translation-observer/baseline-v2.json` against untouched `main` (`7c64f367`) in an isolated checkout using the same Taskfile measurement task and fixture as the final branch. The final branch used `task perf:translation-observer:measure REPORT=target/translation-observer/after-v2.json`. The three added scenarios make ordinary committed-progress latency explicit; the v2 raw JSON supersedes the v1 counts for the final comparison.

## Code-derived estimate, not a measurement

Before the change, each observer read invokes one `list_project_translations_cmd` and one `get_translation_run_status_cmd` per active link: `1 + N` IPC calls for `N` active runs. A project snapshot should invoke one IPC command per read. `TranslationRunControl` also invokes `list_translation_history_cmd` once per link on every delivered observer snapshot. Pending `TranslationPublicationNotice` invokes `get_translation_publication_cmd` independently every three seconds. The report will count these methods rather than folding them into the observer estimate.

## Decision recorded before implementation

Use one project-scoped snapshot. A `job-event` carries a host job ID but no translation ID; `project-updated` and invalidation carry no run identity. Targeted reads would require a potentially stale host-job mapping and full-read fallbacks. The project snapshot provides one IPC call for empty and populated projects without trusting events as state. Keep the three-second recovery read. Keep the notice's pending/error poll because a project run snapshot cannot cover a specific historical publication's pending, ready, and failed states.

## Baseline measured before observer changes

The frozen v1 task passed before observer changes. Its six scenario results are included in `translation-observer-baseline.json`; the initial v1 counts and latency were identical. Each of the three repetitions produced the same values:

| Scenario                     | List | Status | History | Publication | Total | Commit to DOM |
| ---------------------------- | ---: | -----: | ------: | ----------: | ----: | ------------: |
| Empty                        |    4 |      0 |       0 |           0 |     4 |           n/a |
| One active                   |    4 |      4 |       4 |           0 |    12 |           n/a |
| Four active                  |    4 |     16 |      16 |           0 |    36 |           n/a |
| Twenty-event burst           |    5 |     20 |      20 |           0 |    45 |         40 ms |
| Lost event                   |    4 |      4 |       4 |           0 |    12 |       2080 ms |
| Publication pending to ready |    0 |      0 |       0 |           2 |     2 |       2040 ms |

The burst is fired after fixture commit, while its first refresh remains in flight. No native model or SQLite operation is timed. Final results will use the same task and fixture; do not infer model or translation throughput changes.

## Final controlled UI measurement

The same task passed after the snapshot implementation. `translation-observer-after.json` retains all three repetitions; each scenario again produced identical values across repetitions. The table shows absolute calls by command method, with totals for the measured translation controls only.

| Scenario                     | Before: list / status / snapshot / history / publication | Before total | After: list / status / snapshot / history / publication | After total | DOM latency before → after |
| ---------------------------- | -------------------------------------------------------- | -----------: | ------------------------------------------------------- | ----------: | -------------------------: |
| Empty                        | 4 / 0 / 0 / 0 / 0                                        |            4 | 0 / 0 / 4 / 0 / 0                                       |           4 |                        n/a |
| One active                   | 4 / 4 / 0 / 4 / 0                                        |           12 | 0 / 0 / 4 / 4 / 0                                       |           8 |                        n/a |
| Four active                  | 4 / 16 / 0 / 16 / 0                                      |           36 | 0 / 0 / 4 / 16 / 0                                      |          20 |                        n/a |
| Empty → first run            | 4 / 3 / 0 / 3 / 0                                        |           10 | 0 / 0 / 4 / 3 / 0                                       |           7 |                 40 → 20 ms |
| One run commits a block      | 4 / 4 / 0 / 4 / 0                                        |           12 | 0 / 0 / 4 / 4 / 0                                       |           8 |                 40 → 20 ms |
| Four runs commit a block     | 4 / 16 / 0 / 16 / 0                                      |           36 | 0 / 0 / 4 / 16 / 0                                      |          20 |                 40 → 20 ms |
| Twenty-event burst           | 5 / 20 / 0 / 20 / 0                                      |           45 | 0 / 0 / 5 / 16 / 0                                      |          21 |                 40 → 40 ms |
| Lost event                   | 4 / 4 / 0 / 4 / 0                                        |           12 | 0 / 0 / 4 / 4 / 0                                       |           8 |             2080 → 2040 ms |
| Publication pending to ready | 0 / 0 / 0 / 0 / 2                                        |            2 | 0 / 0 / 0 / 0 / 3                                       |           3 |               2040 → 20 ms |

The empty project cannot reduce below one project read per recovery pass. The publication notice makes one extra read to close its subscription-registration gap; readiness is visible on its project event rather than waiting for its timer. A lost event still relies on the three-second recovery cadence. History-head reads are unchanged and remain the largest measured IPC category with four translations. The fixture uses virtual time and mocked methods with 20 ms response latency; it does not measure native IPC, backend SQL duration, outbox scheduling, model speed, or end-to-end translation time.
