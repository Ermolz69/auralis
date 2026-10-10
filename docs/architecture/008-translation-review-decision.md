# Translation review decisions

Status: implementation contract for `INT-TR03`, 10 October 2026.

Auralis already stores a verified immutable translation publication and allows
previewing or attaching a historical result. The `needs_review` flag reports
Translate's structural outcome; it is not a human approval or a speech handoff.
Auralis owns an append-only review journal for these ready publications. Translate
continues to own result text, edits, revisions and output bytes.

An approval or withdrawal identifies the project, translation, result and exact
SHA-256 output digest. The command verifies the source, frozen run, Translate
result and managed artifact before persisting the decision. It also checks the
observed host link revision and requires a ready publication. The decision has a
reviewer label, note and timestamp; these identify a local user action, not an
independent bilingual quality score. A duplicate identical action is idempotent.
A changed action appends another event; old events are not overwritten.

Review is per immutable result. Editing creates another result ID and digest, so
the new revision has no inherited approval. Selecting another historical result
does not change either result's review history. A speech consumer must verify the
currently selected result and managed output again, then require that result's
latest decision to be approval for the verified digest. A missing, withdrawn,
wrong-project, stale or mismatched decision fails closed. The selected result may
still be viewed, edited and exported as a draft subtitle; its publication state
and `needs_review` label do not silently change.

The review journal lives in Auralis SQLite with project/translation/publication
foreign keys and migrates existing databases with no implicit approvals. The
desktop presents the reviewed result identity and digest, requires an explicit
action and shows the current decision. This local workflow supports a later
speech handoff, while independent Chinese/Russian review and G3–G5 remain
separate release evidence.
