# GLib Iterator Safety Backport

## Scope and provenance

The desktop's Linux Tauri/GTK3 stack requires `glib 0.18`. Its `VariantStrIter`
contains the invalid FFI out-pointer reported as
[RUSTSEC-2024-0429](https://rustsec.org/advisories/RUSTSEC-2024-0429).
The fix is released in `glib >=0.20`, which is not a drop-in GTK3 dependency update.

The workspace patch selects `vendor/glib-0.18.5` without changing its version.
The complete official package, including its MIT license and copyright, is retained.
Archive checksum and upstream revisions are recorded in
`tools/glib-backport/provenance.json`.

Only these maintained changes are allowed:

- The two-line source correction from upstream commit
  [`b5a4071e439bef2b5eea76c3aa25e5ae84839e34`](https://github.com/gtk-rs/gtk-rs-core/commit/b5a4071e439bef2b5eea76c3aa25e5ae84839e34):
  make the pointer mutable and pass `&mut p` to the C variadic function.
- Package-local `unused_parens` and `mismatched_lifetime_syntaxes` allowances in
  the normalized Cargo manifest. These preserve compatibility with newer compilers
  after changing from a registry dependency to a path dependency. They affect only
  third-party style diagnostics, not safety lints, project warnings, or RustSec.

No advisory ignore, global lint relaxation, or misleading version bump is added.

## Verification

`task sec:glib:verify` downloads/caches the checksum-pinned official archive and
compares all 121 files. It rejects missing/extra files, a reverted fix, any other
source modification, and a lockfile that selects a registry copy or another version.
Fixture tests exercise failure cases. Global quality checks run this gate; vendor
and verification-tool changes trigger both Rust and quality CI jobs.

On Linux:

- `task rs:glib:regression` runs four tests against the application's resolved
  dependency using the release profile (optimization, LTO, one codegen unit).
  It covers forward/reverse access, skipping, mixed directions, empty arrays,
  exhaustion, and UTF-8 strings.
- `task rs:glib:reproduce` verifies Cargo's resolved source, first requires the
  patched tests to pass, then repeats the same pointer test against the original
  source with identical style-lint compatibility settings. Only a runtime
  SIGSEGV/SIGABRT qualifies as reproduction; compilation/setup failures do not.
- `task check:rust` includes the positive optimized regression on Linux.
  CI also runs the negative control. The control runner requires Node 18+ and tar,
  available on the Ubuntu GitHub runner.
- `task rs:build:release` builds the production native executable after frontend
  assets have been built with `task fe:build`.

For pre-commit Linux verification from Windows, `task rs:setup:wsl` prepares the
existing `Ubuntu-24.04` distribution with native build libraries and Rust 1.95.0.
It installs packages in that distribution, not on the Windows host.
`task rs:glib:wsl` runs both controls. Other checks use
`task rs:exec:wsl -- <Cargo verification arguments>`, for example
`task rs:exec:wsl -- test --locked --workspace` and
`task rs:exec:wsl -- build --locked --release -p auralis-app`.
Linux build output is isolated in `/tmp/auralis-glib-backport-target`.

The pre-commit run on 2026-09-03 passed the complete Windows `task check`
(387 frontend tests, Rust tests, production browser smoke and quality gates),
Linux workspace tests, strict Clippy, formatting and the native release build.
With Rust 1.95.0 and `RUSTFLAGS=-Dwarnings`, all four optimized iterator tests
passed on the patched package; the original pointer test terminated with SIGSEGV.
Native GUI interaction and a signed/distributed installer were not tested.

## Security scanning and maintenance

`task sec:glib:audit` aliases `task sec:rust:audit`, which runs cargo-audit without
exceptions. The separate [SQLite-only SQLx update](013-sqlite-only-dependencies.md)
removes the unrelated `rsa` dependency. A scanner's treatment of local
packages is not proof that this source is fixed: the mandatory source-integrity
gate and executable before/after regression provide that evidence separately.
This backport does not address GTK3 maintenance notices or claim general memory
safety for every GLib API.

The Auralis maintainer owns this pinned copy. Review it on each Tauri/GTK update.
Remove the patch, vendor package, control harness and compatibility allowances
when all application dependency paths use an upstream fixed GLib version; rerun
Linux release tests and the full audit before removing the integrity gate.
Do not copy new upstream releases over the maintained directory: the import task
deliberately refuses to overwrite a nonempty package.
