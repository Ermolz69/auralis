# Project Documentation

## Sections

- [Architecture overview](./architecture/001-overview.md)
- [Technology stack](./architecture/000-stack.md)
- [Frontend FSD boundaries](./architecture/002-frontend-fsd.md)
- [Rust workspace](./architecture/003-rust-workspace.md)
- [Design system](./architecture/004-design-system.md)
- [Storybook conventions](./architecture/005-storybook-conventions.md)
- [Runtime data layout](./architecture/006-runtime-data.md)
- [Production storage contract](./storage/production-storage.md)
- [Bundled media tools](./media-tools.md)
- [CI and quality gates](./ci/001-quality-gates.md)
- [Pinned runner images](./ci/002-runner-images.md)
- [Taskfile commands](./taskfile/001-commands.md)
- [Release workflow](./release/001-release-workflow.md)
- [Production signing](./release/002-signing.md)

The [Figma interface integration document](./figma-interface-integration.md) is a
design analysis and implementation plan. Historical completion reports live in
`reviews/`; they record the state at the date in their filename and are not
maintained as current-reference documentation.

## Storybook

Storybook is the interactive reference for foundations, reusable components, and
complete product states. Start with the [design-system contract](./architecture/004-design-system.md)
and [Storybook conventions](./architecture/005-storybook-conventions.md).

- **Interactive development**: `task fe:storybook:dev`
- **Complete focused verification**: `task check:storybook`
- **Metadata-only verification**: `task q:storybook-contract`
- **Static verification**: `task fe:storybook`
- **Published Pages build**: `task fe:storybook-pages`
- **Published catalog**: [Auralis Storybook](https://ermolz69.github.io/auralis/docs/storybook/)
- **Deployment flow**: `.github/workflows/storybook-pages.yml` deploys after
  changes reach `main` or when started manually.
- **Generated files**: Static output is not committed. GitHub Actions builds and
  uploads it as a Pages artifact.

The focused check validates the catalog contract, runs every story and
accessibility check in Chromium, and builds the production-static site. The full
frontend and frontend-quality suites already include the browser and metadata
portions respectively, so pull-request CI does not duplicate the Pages build.
