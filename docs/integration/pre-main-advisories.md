# PM-001 advisory inventory

This record preserves the retained 2026-10-27 baseline at
`b29719b0dc784745b70275ec93968f5cbda69df3` and the repaired dependency graph.
The [portable JSON inventory](pre-main-advisories.json) contains every advisory
record, resolved version, patched constraint, owning dependency and recorded npm
path, plus the retained Rust reverse dependency trees. Workspace path annotations
were omitted from the tree projection; the original evidence is unchanged.

These are retained scan results, not newly executed final integration audits.
Preparing this inventory ran no dependency installation, audit, build or test.

## Baseline identity

- Windows MSVC `x86_64-pc-windows-msvc`; Node `24.14.0`, pnpm `11.25.0`, Task `3.50.0`.
- Rust `1.95.0` (`59807616e`), Cargo `1.95.0` (`f2d3ce0bd`), cargo-audit `0.22.2`, cargo-deny `0.20.2`.
- Baseline RustSec database: `99670cfc0bc574433034b75876a4b791ccaa2efb`.

| Baseline lock | Recorded SHA-256 | Git blob |
| --- | --- | --- |
| `pnpm-lock.yaml` | `c3d41a10318ee685c0e9c89435367e1d2db8b7851423ff6f4047b8e2490bfa2e` | `53be14cf30ee0323e315c14999ac2efcd16966d7` |
| `Cargo.lock` | `9aaddb2ac0cc78d036342e1649632239005445c96a5fd3cca2ce5a6eebd96738` | `4981e9c8c8a3c6cc9b40c10971303ec981fe2f8c` |

| Retained command | Baseline result | Task / scanner exit |
| --- | --- | --- |
| `task sec:audit:npm` | 38 package/advisory records: 4 critical, 11 high, 17 moderate, 6 low | 201 / 1 |
| `task sec:audit:rust` | 14 vulnerabilities, one denied unsoundness finding, six allowed maintenance notices | 201 / 1 |
| `task sec:deny:rust` | Advisories failed; dependency sources passed | 201 / 1 |

## npm findings and repair

The 38 npm records represent **36 distinct GHSA IDs**, with **801 recorded path
occurrences** and **268 unique dependency chains**. The two Storybook advisories
`GHSA-f2ph-cmg5-wxc8` and `GHSA-px85-f4q5-jh77` each apply to both `storybook`
and `@storybook/addon-vitest`. Finding counts are not unique advisory or path
counts. The full audit includes production and development dependencies; all
affected packages below belong to development/build/documentation tooling.

| Package | Baseline resolved | Fixed resolved | Records | Owning direct dependency or tool |
| --- | --- | --- | ---: | --- |
| `undici` | 7.29.0 | 7.30.1 | 13 | Desktop jsdom/Vitest/Storybook |
| `js-yaml` | 5.2.2 | 5.4.3 | 1 | Root js-yaml and markdownlint-cli2 |
| `brace-expansion` | 5.0.9 | 5.0.12 | 3 | Desktop ESLint/parser/resolver and Storybook |
| `smol-toml` | 1.7.1 | 1.9.1 | 1 | Root markdownlint-cli2 |
| `katex` | 0.16.47 | 0.18.2 | 1 | Root markdownlint-cli2 math extension |
| `source-map-js` | 1.2.1 | 1.2.2 | 1 | Desktop CSS/Vite/Storybook/Vitest tooling |
| `handlebars` | 4.7.9 | 4.7.10 | 3 | Desktop eslint-plugin-boundaries |
| `markdown-it` | 14.2.0 | 15.0.3 | 4 | Root markdownlint-cli2 |
| `vite` | 8.1.3 | 8.2.4 | 5 | Desktop Vite and dependent plugins/test tools |
| `@storybook/addon-vitest` | 10.4.6 | 10.6.2 | 2 | Desktop Storybook and addon-mcp |
| `storybook` | 10.4.6 | 10.6.2 | 2 | Desktop Storybook addons and integration tools |
| `@vitest/mocker` | 4.1.11 | 4.1.12 | 2 | Desktop Vitest and its browser/coverage integrations |

The JSON preserves each advisory's exact audit patched range and upstream
patched branches. This includes the differing Undici, brace-expansion and
markdown-it constraints rather than replacing them with one package-wide range.
Every selected fixed version matches the repaired lock snapshot.

The retained repaired scan at `23a997527cc40eedcad693a24879939721a8f12e` ran
`task sec:audit:npm`: exit 0, zero findings, 646 total dependencies in audit
metadata. Baseline metadata reported 628. Its exact `pnpm-lock.yaml` is unchanged
in the snapshot at `aefa5c00c683696b71d46d2805020c27a54f048c`:
SHA-256 `1ca704a29279ccaa17e7d999af01fc6b74def76bf08eaaf2b1ec8934e13de094`,
Git blob `f5220d5cfe0e7ff60b47f14e659ab6c450e7370c`.

## Rust findings and repair

Each vulnerability below has its original owning chain and patched constraints
in the JSON. Quinn's baseline reverse-tree task exited 0 with no active path;
its lock-only entries remain within full-lock audit scope. Other reverse trees
include shared-subtree markers, so a complete distinct-path count is not inferred.

| Advisory | Crate baseline → fixed | Patched constraints |
| --- | --- | --- |
| RUSTSEC-2026-0185 | quinn-proto 0.11.14 → 0.11.19 | `>=0.11.15` |
| RUSTSEC-2026-0338 | quinn-proto 0.11.14 → 0.11.19 | `>=0.11.18` |
| RUSTSEC-2026-0339 | quinn-proto 0.11.14 → 0.11.19 | `>=0.11.17` |
| RUSTSEC-2026-0340 | quinn-proto 0.11.14 → 0.11.19 | `>=0.11.18` |
| RUSTSEC-2026-0342 | quinn-proto 0.11.14 → 0.11.19 | `>=0.11.17` |
| RUSTSEC-2026-0285 | rustls 0.23.43 → 0.23.46 | `>=0.23.45` |
| RUSTSEC-2026-0346 | rustls 0.23.43 → 0.23.46 | `>=0.23.46` |
| RUSTSEC-2026-0348 | rustls 0.23.43 → 0.23.46 | `>=0.23.46` |
| RUSTSEC-2026-0349 | rustls 0.23.43 → 0.23.46 | `>=0.23.46` |
| RUSTSEC-2026-0382 | rustls-webpki 0.103.13 → 0.103.16 | `>=0.103.16,<0.104.0-alpha.1` or `>=0.104.0-alpha.8` |
| RUSTSEC-2026-0383 | webpki-root-certs 1.0.9 → 1.0.10 | `>=1.0.10` |
| RUSTSEC-2026-0367 | zip 4.6.1 → 9.1.2 | `>=9.0.0` |
| RUSTSEC-2026-0368 | zip 4.6.1 → 9.1.2 | `>=9.0.0` |
| RUSTSEC-2026-0369 | zip 4.6.1 → 9.1.2 | `>=9.0.0` |
| RUSTSEC-2026-0386 (denied unsoundness) | tokio 1.53.1 → 1.53.3 | `>=1.51.6,<1.52.0` or `>=1.53.3` |

The retained repaired Rust scan at `c0a8bdc5077a0cc09efcac1d89e923fe885a053a`
ran `task sec:audit:rust`: exit 0, 570 dependencies, no vulnerabilities or denied
unsoundness, six allowed maintenance notices. RustSec database
`40f5be6fb12ed93fc591bf4ab23b7d793b3f2239` contained 1,381 advisories.
See the [Rust verification record](pre-main-rust-security.md) for the associated
policy and regression checks.

That repaired `Cargo.lock` and snapshot `aefa5c0` have the same Git blob
`16cc5d8fdfa1f1e06c9e245b675f845a0c2ab4e0`; their Git diff is empty. The snapshot's
checkout SHA-256 is `bf4954f6144d9beed2ebf2d9f8b4d1028fdf98c455ab2f7501ece6e9082ef94b`.
Normalizing CRLF to LF yields
`d7d9d8f413b8b9a6b19a9d4cac76f60ee7cfdf49d7aa86252679a237e6ce1a62`, the retained
canonical hash. This is a representation difference, with no dependency graph drift.

The existing `unmaintained = "workspace"` policy retains these six transitive
maintenance notices without a new ignore or suppression:

| Advisory | Unchanged crate | Owning chain |
| --- | --- | --- |
| RUSTSEC-2024-0370 | proc-macro-error 1.0.4 | glib-macros → patched GLib/GTK/Tauri |
| RUSTSEC-2025-0081 | unic-char-property 0.9.0 | unic-ucd-ident → urlpattern → tauri-utils |
| RUSTSEC-2025-0075 | unic-char-range 0.9.0 | unic-char-property → unic-ucd-ident → urlpattern |
| RUSTSEC-2025-0080 | unic-common 0.9.0 | unic-ucd-version → unic-ucd-ident → urlpattern |
| RUSTSEC-2025-0100 | unic-ucd-ident 0.9.0 | urlpattern → tauri-utils → Tauri |
| RUSTSEC-2025-0098 | unic-ucd-version 0.9.0 | unic-ucd-ident → urlpattern → tauri-utils |

Fresh integrated security scans remain a separate acceptance step. This inventory
does not establish a final PR gate, clean-machine runtime acceptance or voice
qualification.
