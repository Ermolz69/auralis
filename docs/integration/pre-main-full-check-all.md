# Consolidated pre-main full non-native check

Date: 2026-10-28. Tested source:
`4310b3664b2b4b7ee992a328a1a7800e69c10540`.
Translate: `5d98cb3ca341c25bd446f7fc9d98d4b62ef551f5`.

The literal root command `task check:all` completed with **exit 0 on Windows**.
The source remained unchanged and clean throughout the run. Mutable Cargo,
pnpm, temporary and result directories were private. Effective limits were
`VITEST_MAX_WORKERS=2`, `CARGO_BUILD_JOBS=2` and `RUST_TEST_THREADS=2`.

## Executed results

| Part of the composed gate                              | Result                                                                 |
| ------------------------------------------------------ | ---------------------------------------------------------------------- |
| Frontend type checking and lint                        | Passed; 18 existing lint warnings retained                             |
| Unit/component/integration/Chromium Storybook coverage | 159 files, 906 tests passed; zero failed or skipped                    |
| Production frontend build and budgets                  | Passed                                                                 |
| Production browser E2E                                 | 35 scenarios passed                                                    |
| Production browser smoke                               | One test passed                                                        |
| Frontend architecture and quality                      | Passed                                                                 |
| Rust formatting and all-target Clippy                  | Passed, warnings denied                                                |
| Rust workspace tests                                   | 758 top-level passed, zero failed, 23 existing ignored                 |
| Documentation, formatting and global gates             | Passed                                                                 |
| npm audit                                              | No known vulnerabilities                                               |
| Rust audit                                             | 570 dependencies, 1381 advisories; zero vulnerability/unsound findings |
| Rust advisory/source policy                            | Passed; six existing maintenance notices remain visible                |

Two nested child-fixture successes produce a raw Rust harness sum of 760;
they are not counted twice in the 758 top-level result. The existing opt-in
ignored tests were not enabled, hidden or reclassified. No models or doctors ran.

## Coverage

| Metric     | Covered / total | Percent | Unchanged threshold |
| ---------- | --------------: | ------: | ------------------: |
| Statements |     5733 / 6280 |  91.28% |                 90% |
| Branches   |     4322 / 5150 |  83.92% |                 80% |
| Functions  |     1598 / 1775 |  90.02% |                 90% |
| Lines      |     5063 / 5391 |  93.91% |                 92% |

The fresh JSON summary covers all 263 production modules. Its SHA-256 is
`59db56ceb84eb35977db5566c266ca85da26bdb553eeeb27f98702d462799026`.
Fresh LCOV SHA-256:
`c014641ac279f1e44f7bae5d54089277e861ead4ae334b5598efc1338af54a72`.
The complete [earlier inventory](pre-main-candidate-coverage.json) remains
explicitly attributed to `5a5ca53`; it is not relabelled as this run.

The retained complete Task transcript SHA-256 is
`2341ea694cd171238e15280e142dd2daa47f4d8aca6efa76ebdfb1c0e9e71733`.

## Platform skip and unresolved native boundary

`check:all` composes `check:pr` and `rs:glib:regression`. The latter declares
`platforms: [linux]` in [the Rust Taskfile](../../taskfiles/rust.yml) and was
not executed by this Windows invocation. That is one platform-gated task,
separate from the 23 ignored Rust tests. This run does not establish the
optimized Linux GLib regression or other unexecuted platform lanes.

The [native failure record](pre-main-candidate-verification.md) remains unchanged:
PM-017 failed and PM-018 is blocked. This full non-native run did not retry
native admission, remove its retained session lock, attribute the unseen parent,
or change any ownership guard. It does not establish native cleanup or technical
readiness. Main and production/harness sources were not changed by this run.
