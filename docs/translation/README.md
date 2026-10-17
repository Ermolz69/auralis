# Auralis Translate integration guide

This page is the current entrypoint for the experimental, file-based subtitle
translation workflow in Auralis. It describes the Auralis checkout pinned to
Translate commit `44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`. The independent
sibling Translate checkout is a separate candidate; its newer HEAD does not
change this Auralis pin.

The [dated integration history](history.md) preserves the former README's
implementation notes, named test outcomes, failures and open gates. Read those
records against their stated candidate and date. The current Auralis
[review/export contract](../architecture/008-translation-review-decision.md)
defines human decisions and verified file export.

Auralis owns projects, host jobs, managed originals/outputs, publications and
outbox actions in `auralis.sqlite`. Translate owns detailed runs, checkpoints,
edits and immutable results in `auralis-translate.sqlite`. The output is a
separate SRT or strict WebVTT file; the external and managed originals remain
unchanged. A ready publication and `needs_review` label are not human approval.

The checked model package is installed and selected explicitly. A manual
`translation-runtime.json` is a development fallback only when no package is
selected. The supported desktop workflow, acceptance scope and old heading
fragments are expanded in the next documentation pass. Until then, consult the
[history](history.md) and registered command map in `src-tauri/src/lib.rs`.

Model-backed translation, clean installation, source-grounded language quality,
the full concurrency matrix and speech handoff require separate evidence. The
assignment [report](../maintenance/README.md) defines the review boundary.
