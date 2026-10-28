# Rust pre-main security verification

## Source and scope

Verified on 2026-10-28 from `fix/rust-security` at
`c0a8bdc5077a0cc09efcac1d89e923fe885a053a`. The implementation builds on
`b29719b0dc784745b70275ec93968f5cbda69df3`; the independently owned Translate
component is pinned to `5d98cb3ca341c25bd446f7fc9d98d4b62ef551f5`.

This record covers the Rust portions of PM-009 through PM-012. Final integrated
security and candidate acceptance remain PM-013 through PM-018.

| Input                  | SHA-256                                                            |
| ---------------------- | ------------------------------------------------------------------ |
| Root `Cargo.lock`      | `d7d9d8f413b8b9a6b19a9d4cac76f60ee7cfdf49d7aa86252679a237e6ce1a62` |
| Translate `Cargo.lock` | `cfbd1fdb032db88a66612d8d3c4a8dd4773d88e3567d905ab269baacca852f67` |

## Environment and baseline

- Windows MSVC, Rust `1.95.0` (`59807616e`), Cargo `1.95.0` (`f2d3ce0bd`).
- Task `3.50.0`, cargo-audit `0.22.2`, cargo-deny `0.20.2`.
- Fresh RustSec database: `40f5be6fb12ed93fc591bf4ab23b7d793b3f2239`,
  1,381 advisories. Audit refreshes used network access.
- Cargo dependency caches, targets and temporary data were dedicated to this
  worktree. Build concurrency was bounded to two jobs.
- The retained `b29719b` baseline recorded 14 vulnerabilities and one denied
  unsoundness finding. It is historical evidence; the fresh completion baseline
  below is the repaired source at `c0a8bdc`.

The source keeps Quinn `0.11.19`, Rustls `0.23.46`, WebPKI `0.103.16`, bundled
WebPKI roots `1.0.10`, Tokio `1.53.3` and ZIP `9.1.2`. The updater explicitly
retains TLS and system-proxy support and the existing v2 installer/update formats.
No dependency, advisory-policy or production implementation change was made
during this completion pass.

## Fresh verification

| Command                                                                   | Exit | Observed result                                                                                                                        |
| ------------------------------------------------------------------------- | ---: | -------------------------------------------------------------------------------------------------------------------------------------- |
| `task sec:audit:rust`                                                     |    0 | 570 dependencies; no vulnerabilities or denied unsoundness findings; six allowed maintenance notices                                   |
| `task sec:audit:rust LOCKFILE=modules/auralis-translate/Cargo.lock`       |    0 | 231 dependencies; no vulnerabilities, unsoundness findings or yanked notices                                                           |
| `task sec:deny:rust`                                                      |    0 | Advisories and dependency sources passed the unchanged policy                                                                          |
| `task rs:pr`                                                              |    0 | SQLite graph, formatting and all-target Clippy passed; 758 top-level tests passed, zero failed, 23 existing ignored                    |
| `task --dir modules/auralis-translate check`                              |    0 | Formatting and all-target Clippy passed; 295 tests passed, zero failed, three existing ignored; all five archive-security cases passed |
| `task sec:glib:verify`                                                    |    0 | Six controls; 121 upstream files; exact safety patch and existing lint compatibility settings                                          |
| `task rs:process-wrap:verify ARCHIVE=<process-wrap-10.0.0.crate>`         |    0 | 94 upstream files; exactly three pinned local edits                                                                                    |
| `task release:smoke:check`                                                |    0 | All 11 release format, integrity and signing-policy controls passed                                                                    |
| `task fe:install`                                                         |    0 | Frozen documentation-tool installation; manifests and lockfiles unchanged                                                              |
| `task q:docs:format-file FILE=docs/integration/pre-main-rust-security.md` |    0 | Scoped report formatted                                                                                                                |
| `task check:docs`                                                         |  201 | Link contract and five controls passed; full lint found 41 violations in untracked generated cache files                               |

The first `task rs:pr` attempt was interrupted before tests while its Cargo
Clippy preparation produced no compiler diagnostics. Task reported exit 201;
the attempt does not establish a source failure or a successful test run. A
successful repeat used the same locked gate with `CARGO_NET_OFFLINE=true` and the already
populated dedicated dependency cache. Online vulnerability scans remain separate.

The workspace count excludes two additional successful fixture-child harness
invocations; summing every raw harness summary gives 760. Existing ignored tests
were not changed or silently counted as passing. The component check used two
build jobs and two test threads; Auralis and component gates ran sequentially.

Two initial documentation-format attempts exited 201 because pnpm's automatic
dependency check selected a mismatched store and refused a noninteractive
modules replacement. Setting the correct private `PNPM_CONFIG_STORE_DIR` and
related cache/state configuration allowed the frozen installation and formatting
to pass without a dependency update. The full documentation lint's generated-cache
findings remain a retained failed attempt; source-document validation in the
integrated candidate must be checked separately.

## Maintained security boundaries

The ZIP installer checks raw permission/type bits as well as the decoded file
view: the upstream `unix_mode()` representation removes privileged bits. Its
regressions cover traversal, Windows alternate streams, directories, duplicate
names, special entries, privileged permissions, escaping symlinks, corrupt or
incomplete archives, interrupted asset copies and publication only after complete
verification. They retain sentinels outside the destination and assert that
rejected inputs leave no published package or staging residue.

Process cleanup continues to preserve bounded cancellation, unknown-exit errors,
owned descendant confirmation and admission retention when release is unproved.
The GLib and Windows job-object backports retain their exact provenance gates.

## Allowed maintenance notices

These are transitive unmaintained packages accepted by the existing
`unmaintained = "workspace"` policy, with no new exception or ignore:

| Advisory          | Package                    | Owning chain                                                           |
| ----------------- | -------------------------- | ---------------------------------------------------------------------- |
| RUSTSEC-2024-0370 | `proc-macro-error 1.0.4`   | `glib-macros` through patched GLib/GTK and Tauri; GLib regression tool |
| RUSTSEC-2025-0081 | `unic-char-property 0.9.0` | `unic-ucd-ident` through `urlpattern` and `tauri-utils`                |
| RUSTSEC-2025-0075 | `unic-char-range 0.9.0`    | `unic-char-property` through the same Tauri chain                      |
| RUSTSEC-2025-0080 | `unic-common 0.9.0`        | `unic-ucd-version` through the same Tauri chain                        |
| RUSTSEC-2025-0100 | `unic-ucd-ident 0.9.0`     | `urlpattern` through `tauri-utils`                                     |
| RUSTSEC-2025-0098 | `unic-ucd-version 0.9.0`   | `unic-ucd-ident` through the same Tauri chain                          |

## Integration boundary

The exact Translate component commit must be fetchable from its documented
remote before a clean-clone candidate is reproducible. Separately passing Rust
and frontend branches do not establish the combined PM-013 security gate or the
PM-014 PR gate. The integrated source and lockfiles require their own final run.

No model inference, model loading/download, model doctor, existing project data,
voice reference or qualification change belongs to this verification. Native
application, real-media/listening and clean-Windows acceptance remain separate.
