# Ordinary profile upgrade and genuine caption screen

Date: 23 October 2026. Scope: partial INT-R02/05. No native import, inference,
translation quality, listening, long-story or clean Windows gate has passed.

## Installed host candidate

`task desktop:bundle` succeeded for committed application code
`70471a458cad95edcb42f54ee7df4fc982ff4c3a` and pinned Translate
`44e7d4f3c6ce2443279f5d0d3acd07cbb931476e`. The only concurrent working-tree
changes were engineering scripts/tests/docs outside the production payload.
MSI extraction again verified the actual packaged media tools/licenses and
absence of test payload/model weights. Code signing remains skipped by the
configured non-production policy.

| Artifact                   | SHA-256                                                            |
| -------------------------- | ------------------------------------------------------------------ |
| NSIS installer             | `485d8783a0b3432ade03fda90eaa9029a3e6c79bcbed8477eedadaf1862e18f0` |
| NSIS installed application | `4c80cb6c6e8a2cd31be886ba87c9d781d41ed94723187e5de1c617ec9b4f7f3a` |
| MSI installer              | `b7242be80dad8a2a0926ebc8904fd584abc41cdfe90d06d518f2706d1bba60a6` |
| MSI application            | `4e37e12bdc97489eda15296bfe1c9ef85a7eec076dc1bbd351a908045048f94f` |

`tools/revoice/upgrade-long-host-v1.json` froze one upgrade attempt before
installation. The bounded task verifies the prior installed payload, refuses
nonempty/active profiles, stops only the verified owned idle application tree,
retains SQLite/configuration/previous executable backups and verifies the new
installed hash. It never launches an app or model itself. The previous packages
also remain in `target/revoice/pre-long-host-installers-20261023/` with a receipt.

`task revoice:profile:inspect` found 0 projects / 0 jobs and schema 17. A separate
`task revoice:profile:backup OUTPUT=target/revoice/profile-backup-smoke-v1`
validated SQLite's read-only online backup including WAL and `quick_check`.
`task revoice:upgrade:local` then completed in 9,548 ms, installer exit 0 and
exact NSIS payload hash. `tools/revoice/evidence/LONG_HOST_UPGRADE_V1.json`
retains the parameters, owned process IDs and result. The raw backups remain
under `target/revoice/local-upgrade-long-host-v1.json.backup/`.

The registered application launched normally. Its new window ID was 63047408.
The ordinary profile migrated to schema 18, still 0 projects / 0 jobs. TTS
configuration hash stayed
`e80c4a3d0f8c6091547b1fcb14f61411b6e2d6402c79e4abcb3130fd73fb91a9`.
Startup logs show `import_project_pins_cmd` succeeded and accessibility shows
the normal empty project page without the earlier pins error. These prove only
the stated startup/read boundary. Capture of the fresh window still failed
with `window capture timed out: timed out waiting on channel`. No new button
input or URL import was attempted after that unchanged helper failure.

## Exact Chinese source bytes

`tools/revoice/captions-v1.json` froze a one-attempt, 120-second / 8 MiB source
acquisition before execution. `task revoice:youtube:captions` acquired the real
shared `-ModelFree` lease, verified the retained metadata hash and fetched only
the exact manual `zh` VTT representation of HRaa8kb1EhE. No automatic fallback,
video, model or inference was used. HTTP 200, 383 ms: **217,352 bytes**, SHA-256
`daede8cab3c510ce0ab7317f100d465b333b4ab39b6489249df79483696020e5`.
The preliminary timing-line screen found 4,004. The guardian released normally.
Receipts are `tools/revoice/evidence/YOUTUBE_CAPTIONS_V1.json` and
`tools/revoice/evidence/YOUTUBE_CAPTIONS_ADMISSION_V1.json`. The raw original is
retained unchanged in the receipt's ignored target path, not redistributed.

`task rs:test:source-inspect` then used the real production FormatInspector with
the frozen bytes/counts in `tools/revoice/source-inspect-v1.json`. It **failed**
in 21 ms: `WebVTT error MissingHeaderSeparator at line 2`. This public provider
representation contains `Kind: captions` and `Language: zh` header metadata;
the pinned strict plain-WebVTT grammar intentionally requires the separator
after `WEBVTT`. It is not accepted source coverage. The failed result is
`tools/revoice/evidence/YOUTUBE_STRICT_VTT_V1.json` and must remain failed.

An earlier compile-only attempt imported unavailable `sha2`; it was corrected
to the existing pinned `SourceHash` API before the first parser execution.
The first inline read-only profile query also failed PowerShell quoting before
SQL executed; it was replaced by the repeatable file-based snapshot task.
Neither failure loaded a model or changed the source/profile.

## Next supported boundary

Metadata also lists an SRT representation of the same manual Chinese track.
The adapter currently exposes format-specific choices but both downloader paths
force VTT, and the subtitle workspace filters to VTT. Preserve the strict VTT
failure and all raw bytes. Implement explicit supported SRT selection/retrieval,
test command arguments/output extension and UI identity, then freeze a new
SRT acquisition/inspection case before running it. Do not strip header bytes,
relax the parser implicitly or relabel automatic captions. A later installed
candidate must include this correction before a full YouTube-import claim.

The active profile is a warm local developer machine, not clean Windows.
New voices/manifests/reports remain unchanged and no real synthesis ran.
