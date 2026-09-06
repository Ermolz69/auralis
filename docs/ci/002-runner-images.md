# Pinned GitHub Runner Images

Windows and macOS package jobs use reviewed versioned runner labels rather than
the moving `*-latest` aliases. The current pins are:

- Windows Server 2025: `windows-2025`
- macOS 26 arm64: `macos-26`

The machine architecture remains aligned with the release contract: Windows
publishes `windows-x86_64` updater assets and macOS publishes `darwin-aarch64`
assets.

## Policy Contract

`tools/ci/runner-images.json` is the source of truth for approved Windows and
macOS labels. It records the official image list and the last review date.

`task q:runner-images` runs fixture tests and scans every workflow. It rejects:

- `windows-latest` and `macos-latest`;
- a concrete runner label that is absent from the reviewed policy;
- malformed, duplicate, or unused policy labels;
- invalid workflow YAML.

The global quality gate runs this task, so a pull request cannot silently return
to a floating runner or change an OS version in only one workflow.

## Controlled Upgrade

When GitHub deprecates an image or a toolchain needs a newer one:

1. Review the official [runner image list](https://github.com/actions/runner-images#available-images)
   and the linked image release notes.
2. Update the workflow matrices and `tools/ci/runner-images.json` in the same pull
   request, including `reviewedAt`.
3. Run `task q:runner-images` and `task q:ci-bootstrap` locally.
4. Trigger `Tauri Build` and verify Windows and macOS package, media-tool,
   install, and launch checks. Confirm that their updater target mapping remains
   `windows-x86_64` and `darwin-aarch64`.
5. For a release-pin change, validate signing and notarization in the production
   release workflow before publishing its draft.
6. Merge only after both pinned platforms are green.

GitHub exposes versioned operating-system labels through `runs-on`; it does not
offer a stable label for an individual weekly VM image build. The OS pin removes
major image migrations, while normal security and tool updates within that image
line still arrive from GitHub.
