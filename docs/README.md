# Project Documentation

## Sections

- [Architecture](./architecture/)
- [ADR](./adr/)
- [Conventions](./conventions/)
- [CI](./ci/)
- [Taskfile](./taskfile/)
- [Release](./release/)
- [Security](./security/)

## Storybook

We use Storybook to develop and document our shared UI components.

- **Local Development**: Run `task frontend:storybook` (or `pnpm --filter desktop run storybook`) to start the local Storybook server.
- **Production Build**: Run `pnpm --filter desktop run build-storybook` to verify the static build.
- **Published URL**: Our Storybook is automatically deployed to [https://ermolz69.github.io/auralis/docs/storybook/](https://ermolz69.github.io/auralis/docs/storybook/).
- **Deployment Flow**: Deployment happens entirely through GitHub Actions (see `.github/workflows/storybook-pages.yml`) whenever changes are merged into the `main` branch.
- **Generated Files**: We do not commit the generated static Storybook files to version control. They are built and uploaded as GitHub Pages artifacts on the fly.
