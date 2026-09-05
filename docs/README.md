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
- [Taskfile commands](./taskfile/001-commands.md)
- [Release workflow](./release/001-release-workflow.md)
- [Production signing](./release/002-signing.md)

The [Figma interface integration document](./figma-interface-integration.md) is a
design analysis and implementation plan. Historical completion reports live in
`reviews/`; they record the state at the date in their filename and are not
maintained as current-reference documentation.

## Storybook

We use Storybook to develop and document our shared UI components.

- **Static verification**: Run `task frontend:storybook` to build the current Storybook catalog.
- **Published build**: Run `task frontend:storybook-pages` to create the GitHub Pages layout.
- **Published URL**: Our Storybook is automatically deployed to [https://ermolz69.github.io/auralis/docs/storybook/](https://ermolz69.github.io/auralis/docs/storybook/).
- **Deployment flow**: `.github/workflows/storybook-pages.yml` deploys after changes are merged into `main` or when started manually.
- **Generated files**: Static Storybook output is not committed. GitHub Actions builds and uploads it as a Pages artifact.
