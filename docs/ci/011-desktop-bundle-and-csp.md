# Desktop Bundle and CSP Checks

`task fe:build` creates the production build, writes `apps/desktop/dist/bundle-report.json`,
and runs `task fe:bundle:check`. The report lists chunks, static/dynamic imports, and
rendered module sizes. The checker prints the largest modules for investigation.
Module sizes precede final chunk compression; budgets use the actual output files.

## Budgets

| Metric                                                      | Limit   |
| ----------------------------------------------------------- | ------- |
| Initial JavaScript, including all transitive static imports | 350 KiB |
| All JavaScript, including lazy chunks                       | 500 KiB |
| Largest JavaScript chunk                                    | 250 KiB |
| Initial JavaScript gzip                                     | 110 KiB |
| All JavaScript gzip                                         | 160 KiB |
| Stylesheets                                                 | 100 KiB |

The initial measurement after integration was 263.27 KiB JS / 82.88 KiB gzip;
all JS was 308.56 KiB / 99.68 KiB gzip. Compare future results with the current
report rather than raising limits automatically. `task q:desktop-policies` covers
shared/cyclic imports, lazy chunks, invalid manifests, budget failures, and UTF-8 sizes.

The icon registry uses named Lucide imports rather than the full dynamic catalog.
Vite 8's `build.rolldownOptions.output.codeSplitting` creates a vendor chunk.
Project, source/subtitle workspaces, settings, and the queue are lazy-loaded.

## Content security policy

Production CSP is defined in `src-tauri/tauri.conf.json`. Scripts are self-only;
objects, frames, base URL overrides, and form navigation are blocked. Image/media
sources explicitly allow local Tauri assets and blob URLs; images additionally
allow stored data URLs and YouTube thumbnails from `i.ytimg.com`. Connect sources
only allow the app origin and Tauri IPC. No wildcard remote network access or
`unsafe-eval` is allowed. Inline styles remain allowed because layout components
use React style attributes. Tauri's script hash/nonce injection is not disabled.

Development adds only `ws://localhost:5173` for HMR. The Vite port is fixed to avoid
silently falling back to an endpoint not covered by the development policy.

`task fe:smoke` serves the actual production output with the configured CSP header
and checks navigation, lazy chunks, and global job continuity in Chromium. It also
verifies rejection of inline scripts and a disallowed external connection. This
uses an explicit mock of the IPC boundary; it is not a native WebView/IPC test.
It runs after the build in `task check:frontend`, including CI and release checks.

References: [Tauri CSP](https://v2.tauri.app/security/csp/),
[Vite 8 build configuration](https://v8.vite.dev/guide/build).
