# Bounded long-source host admission

Date: 23 October 2026. Scope: partial INT-R07. Parent code is Auralis
`8c30eeb` plus the scoped long-source changes; Translate remains pinned at
`44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`. This is model-free engineering
evidence. LONG-01/02 and the full story/listening gates remain pending.

## Behavior and bounds

The two source checks and storage intent validation no longer reject cue 513.
The [admission policy](../revoice-input-contract.md#speech-admission-policy)
defines configurable source/utterance/chunk quotas, fixed byte ceilings and
invalid-configuration behavior. Defaults are 4,096 utterances, 8,192 chunks and
1 MiB source text; hard counts are 8,192. Plans and result JSON remain capped at
16 MiB. Composition checks expanded chunk counts before registering a run or
starting inference. Lower quotas preserve stored work but block revalidation
until restored. The settings file is optional and read at startup; no settings
screen is claimed.

SQLite schema 18 rebuilds the count-constrained parent/checkpoint tables in one
transaction on a dedicated detached connection. This avoids cascaded deletion
when replacing the parent. The migration validates every foreign key before
commit; a failure rolls back DDL, rows and the version. The connection closes
without returning disabled enforcement to the pool. Existing pooled-connection
foreign-key/WAL controls pass. Old binaries reject the new schema, so an app
downgrade requires a separately managed compatible database snapshot.

Assignment validation/lookup now uses ID sets/maps. History enumerates at most
100 IDs and loads one plan at a time rather than retaining up to 100 full plans.
Source snapshots remain whole, bounded metadata. Saved/composed plans and the
second manifest read have bounded actual reads; the latter rechecks its hash
after worker verification. Result JSON reads also stop if a file grows beyond
its limit. Existing hash, review, qualification, checkpoint sequence and
publication guards remain intact.

## Observed checks

| Command                      | Result                                                                                                                 |
| ---------------------------- | ---------------------------------------------------------------------------------------------------------------------- |
| `task rs:test:speech-limits` | 2 configuration tests, 3 migration/checkpoint tests, 2 legacy combined-schema tests and 9 speech workflow tests passed |
| `task rs:test:storage`       | 192 passed, including all migration, FK, outbox, publication and pin controls                                          |
| `task rs:test:tts`           | 30 passed, 3 installed-runtime tests ignored; no model loaded                                                          |
| `task rs:clippy:speech`      | Passed with warnings denied                                                                                            |
| `task rs:test:desktop`       | 63 unit and 2 integration tests passed, including startup/storage composition                                          |
| `task docs:integration`      | 5 checker tests passed, 64 documents valid, lint/format passed before this acceptance record                           |
| `task revoice:plan:check`    | 3 checker tests passed; 17 tasks / 24 mandatory cases remain active                                                    |

The 600-cue workflow imports an actual temporary SRT through the application
use case, preserves every ordered cue ID/text/time, reviews it, prepares an
immutable plan through a fixture engine and persists all 600 voice/emotion
overrides. Original bytes are unchanged. Lowering the quota to 599 rejects new
admission without removing the accepted run. An invalid configuration also
refuses admission. Catalog qualifications and audio in this test are fixtures,
not the installed voices or real inference.

The upgrade test preserves legacy plans, attempts, reviews, publication records
and checkpoint rows over two reopens. A real checkpoint 4,097 is accepted after
an ordered fixture prefix of 4,096, persists over reopen and still fences progress.
The hard count 8,193 fails both repository and database controls. A corrupt-FK
negative control leaves the original version-17 tables/data unchanged after a
refused migration.

Failures were retained during development. The first new storage test used a
dynamic SQL string rejected by SQLx 0.9's audited-string contract; it now uses
literal SQL queries. Another test incorrectly applied `?` to pool close, which
returns unit; it was corrected. The first broad storage run found additional
old expected-version assertions (17); all affected upgrade expectations now
assert 18, and the unsupported-future-version negative control uses 19.
These were test corrections, not disabled acceptance checks. Full passing logs
remain under `target/revoice/long-storage-check-v2.log` and
`target/revoice/long-tts-check-v1.log`.

## Unpassed boundary

The owner confirmed the desktop is active and the installed window visible.
Resetting/rebinding the official native helper still returned
`FrameArrived timed out: timed out waiting on channel`. Stop retrying unchanged
window input. No native import was confirmed and no lock-screen diagnosis was
made. The ordinary installed payload is still `b6dc303`, not this host slice.

The Python raw audio joiner still collects/concatenates decoded audio in memory.
Streaming assembly, natural long-source bytes/coverage, measured resource use,
recovery and real playback are therefore open. This host change cannot pass
INT-R07 or LONG-01/02 by itself. Voices, references, reports and source originals
were not changed. TXT, ASR, full translation quality, listening and clean Windows
remain mandatory and unpassed.
