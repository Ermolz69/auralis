# Portable component build and peer verification

## Sources

Reviewed on 2026-10-28 from integrated source
`bcef6e05ab08a07fd00778ad4a7bbd6160607559`.
The component builder changes are `937d88d3a54a8f9bbeac8712b1c268a0dd78f6b6`
and `f8117c38e31015b37a7f1c7a0f78b17dc8577d63`.

The original `fix/rust-security` source remains at
`41faabb6a80de981079a0a13733ad442911f99c6`. No active runtime source pin was
changed by this tooling work.

## CI and documentation review

The CI change `414b1f0` checks out the pinned Translate submodule before
bootstrap resolves Rust path dependencies in all five consumers. Gitlink and
source-location changes trigger Rust and dependency-policy gates. The controls
reject missing and false submodule settings; existing ordering controls keep
checkout before bootstrap. Local validation passed all 28 controls. This does
not establish a hosted Linux run.

The documentation change `2fc3152` accepts existing non-Markdown evidence files
while rejecting missing or misplaced targets. Assets cannot replace mandatory
Markdown documents. Eight controls pass. Generated `.cache` content is excluded
from Markdown lint; tracked production documentation remains included. The
complete documentation gate now passes in the same worktree where generated
cache documents previously produced 41 findings. No blocking peer-review defect
was found in these two changes within this local verification scope.

## Integrated baseline checks

These checks ran on `bcef6e0` before the component-builder implementation changed.

| Command                       | Exit | Observed result                                                                                |
| ----------------------------- | ---: | ---------------------------------------------------------------------------------------------- |
| `task fe:install`             |    0 | Frozen integrated graph in a dedicated pnpm store                                              |
| `task q:ci-bootstrap`         |    0 | 28 controls passed                                                                             |
| `task check:docs`             |    0 | Eight controls, 74 Markdown documents and 11 mandatory documents validated; lint passed        |
| `task check:quality:docs`     |    0 | Complete Markdown formatting passed                                                            |
| `task q:global`               |    0 | Complete global quality and source-integrity gates passed                                      |
| `task check:quality:security` |    0 | npm audit found no vulnerabilities; complete Rust audit and advisory/source policy passed      |
| `task media:prepare`          |    0 | Pinned media tools and licenses were already cached and verified                               |
| `task media:verify`           |    0 | FFmpeg/ffprobe 8.1.2 and yt-dlp 2026.08.19 hashes, provenance and executable versions verified |

The Rust audit scanned 570 locked dependencies against 1,381 advisories. The
same six transitive unmaintained notices remain allowed under the unchanged
policy documented in [Rust verification](pre-main-rust-security.md).

## Component builder behavior

The builder takes an explicit CPU Python interpreter, uv executable, clean
CONTRACT/TTS repository roots, full source revisions, TTS wheel and SHA-256.
Both packages are exported from the pinned Git revisions. CONTRACT builds with
offline uv, no build isolation, no Python downloads and the fixed source epoch
`1791639837`. Supplied sources remain unchanged.

Wheel admission verifies package/data bytes, complete dependency and entry-point
metadata, portable platform identity and every RECORD entry against the selected
source. Incompatible CONTRACT versions cannot produce a successful receipt.
The receipt binds source revisions/trees and both complete wheel hashes, contains
portable relative paths, and is published exclusively after validation. Existing
output identities cannot be overwritten. Failed attempts have no successful
component receipt.

The matching-hash ignored-payload reproduction exposed a real defect in the
initial checkout-based TTS comparison: a Git-ignored extra module was accepted
as part of the pinned revision. The control failed with Task exit 201: six
subcases passed, the new subcase and its parent failed. The Git-snapshot repair
then passed all eight reported tests with zero failures or skips. Other controls
cover dirty/foreign/missing sources, incorrect hashes, altered payload/metadata/
platform/RECORD, incompatible versions, unsupported build backends, deterministic
rebuilds and no-overwrite behavior.

## Exact model-free build inputs and results

- CONTRACT source: `13f4383c7ca8b8b7b575eb64fdf9c54856a9204a`, package tree
  `deab23554bc79faae0f5e402e1ab0f57adec8096`.
- TTS source: `49662267df9ee467e61a0d4e6a4caf7955f54a7d`, package tree
  `0eb71b3933d08c38161177ffafd19644e248a0ed`.
- Build environment: CPython 3.12.14, setuptools 84.0.0, fixed epoch 1791639837.

| Wheel                                       | Bytes | SHA-256                                                            |
| ------------------------------------------- | ----: | ------------------------------------------------------------------ |
| `voxscript_contract-0.2.0-py3-none-any.whl` | 75499 | `c8dcb3ecfbd64f821c56347ee8df039c8bff1d486c8d29f5967ac26a270e5381` |
| `puza_tts-0.3.0-py3-none-any.whl`           | 57290 | `1b0fa4ea0e7b8f3e055194edf202bb24ff826e1a54bcf354f5036ebbd9a3cca6` |

`task ui:components:build` with these explicit inputs passed in three distinct
retained identities. Both complete wheel hashes matched across the two original
builds and the final both-sources-from-Git replay. The separate CASTER package
verification used its own recorded build-tool version and wheel hash; that
acceptance is not reassigned to these wheels by implication.

`task ui:components:check PYTHON=<build-python> UV_BIN=<uv>` passed all eight
reported tests after the source-binding repair. The existing portable
`task ui:check:runner` passed 33 controls; global and documentation gates passed
after the initial builder implementation.

Full integrated PR checks, the ordinary Windows candidate build and native
metadata verification require the final combined source and component pins.
They are not established by these package-build checks. No models, engine
doctors, project data, reference voices or qualification statuses were changed.

## Static build and complete entry-point follow-up

The follow-up starts from integrated source
`aefa5c00c683696b71d46d2805020c27a54f048c`. The original reviewed component
pyprojects contain no alternate backend paths, custom command classes or
setup.cfg; this work tightens admission without alleging corruption of those
sources.

New real PowerShell/Git/wheel controls reproduced the gaps before the repair:
`task ui:components:check PYTHON=<private-builder> UV_BIN=<uv>` returned 201,
with seven tests passing and five failing, including the parent. An inert
backend-path fixture wrote its execution sentinel before rejection. Honestly
recomputed RECORD files allowed undeclared entry points through both no-script
and declared-script sources. A valid complete case-sensitive console/GUI/plugin
mapping was incorrectly rejected.

The repaired admission rejects backend-path, cmdclass and setup.py/setup.cfg
before uv, and compares every declared entry-point group, name and value with
case preserved and interpolation disabled. The same Task then returned 0:
12 tests passed, zero failed or skipped. All three alternate-build controls
reject without executing their fixture sentinel. A fresh canonical build via
`task ui:components:build` returned 0 and preserved both wheel hashes and lengths
in the table above.

`task ui:components:prepare ID=<new-id> PYTHON=<existing-cpython312>` returned 0.
It creates a private Windows CPython 3.12 venv with pip available and exactly
setuptools 84.0.0, wheel 0.45.1 and packaging 26.3; cache/temp directories belong
to that new identity. CPython 3.12.14 with pip 25.0.1 was observed. Repeating an
existing identity and supplying a traversal identity each returned 201; the
existing successful receipt remained unchanged. No interpreter was downloaded.

An earlier `setup:build` environment lacked public packaging despite having
setuptools and wheel. Its control run returned 201 before meaningful assertions
(one passed, 11 failed); that is a preparation failure, not the defect
reproduction above. A restricted Node child-process attempt also returned 201
before assertions. An initial malformed canonical-build Task invocation returned
200 without building; corrected argument grouping produced the successful build.

The literal PM-010 entry points also ran on the unchanged Rust implementation
in `aefa5c0`: `task rs:test:component-cleanup` returned 0 with six TTS and two
translator tests passing; `task rs:test:job-tree` returned 0 with four tests
passing and owned inert descendant exit identities confirmed. Neither command
ignored tests or launched a server/model.
