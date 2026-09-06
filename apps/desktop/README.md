# Auralis Desktop

Auralis is a desktop application built with React, TypeScript, and Vite. This
document is the frontend contributor entrypoint. The canonical rules live in
[Auralis Design System](../../docs/architecture/004-design-system.md) and
[Storybook Conventions](../../docs/architecture/005-storybook-conventions.md).

## Design System & UI Components

Our design system uses semantic tokens, production-backed stories, and strictly
isolated UI components. The frontend follows Feature-Sliced Design (FSD).

### Where are the Shared UI Components?

Reusable generic UI components live in `apps/desktop/src/shared/ui` and are
exported through public `index.ts` entrypoints.

Do not put business logic, data fetching, or Tauri calls in `shared/ui`.

### How to Add a New Component

1. Create a new folder in `src/shared/ui/` (e.g., `src/shared/ui/badge/`).
2. Implement the component (e.g., `Badge.tsx`) using Tailwind classes mapped exclusively to our CSS variables (e.g., `bg-primary`, `text-muted`).
3. Ensure it supports standard accessibility (ARIA labels, keyboard navigation, focus states).
4. Add a `[ComponentName].stories.tsx` file in the same directory with Autodocs,
   a component description, and applicable visual states.
5. Add role-based `play` assertions when interaction, focus, or keyboard behavior
   is part of the contract.
6. Export the component from its local and shared public APIs.
7. Run `task check:quality:frontend` and `task check:storybook`.

### How to Add a Story

Storybook documents foundations, shared components, and complete product states.
When adding or updating public UI, provide explicit examples for applicable
variants, sizes, lifecycle states, long content, keyboard behavior, and responsive
layouts. Helper-only modules may be covered by the components that render them.

Use the repository-level commands:

```bash
task fe:storybook:dev
task q:storybook-contract
task check:storybook
```

The interactive catalog includes the Auralis manager theme, all eight application
themes, standard desktop viewports, Autodocs, source examples, browser interaction
tests, and failing accessibility checks. Cross-slice showcases belong under
`src/app/stories`; component stories remain next to their component.

### Browser Test Setup

Storybook interaction and accessibility checks use Playwright through the existing Vitest browser
project. From the repository root, install frontend dependencies and the browser runtime
on a new machine (and repeat browser setup after upgrading Playwright):

```bash
task install:frontend
task fe:setup:playwright
```

Rust is not required for frontend-only checks. On Linux, use
`task fe:setup:playwright:ci` to also install system libraries with elevated
permissions. Docs-only work needs `task install:frontend`, but no browser runtime.

This setup task is intentionally separate from the regression gates. Run gates through Taskfile:

```bash
task check:frontend
task check:quality:frontend
task check:storybook
task fe:e2e
```

The Settings page includes the signed application updater. Browser development reports it as
unavailable; installed production builds read `latest.json` and their platform bundle from the
latest published GitHub Release. The production signing and publication procedure is documented
in `docs/release/001-release-workflow.md` and `docs/release/002-signing.md`.

## Theming & Colors

The production theme is defined in `src/app/styles/theme.css`. The registry in
`src/shared/theme/config/colorThemes.ts` currently exposes five dark and three
light palettes. `ThemeProvider` persists the application selection; Storybook's
preview toolbar uses the same registry without writing production preferences.

### Why are Raw Hex Colors Forbidden?

Using arbitrary hex colors (e.g., `text-[#ff0000]`) or raw Tailwind color scales (e.g., `text-red-500`) directly in feature or page code breaks visual consistency across the existing dark and light themes.

All colors **must** be routed through our semantic tokens (e.g., `primary`, `surface`, `danger`, `muted`).

### How to Add a Theme Token

If you genuinely need a new semantic color:

1. Open `apps/desktop/src/app/styles/theme.css`.
2. Add the semantic variable to the Tailwind v4 `@theme` block.
3. Add an override to each palette when the default value is not suitable for that theme.
4. Update `DesignTokens.stories.tsx` to document the token visually.

### How to Check for Raw Colors

We enforce color consistency using an automated script that scans the codebase for forbidden hex colors and standard Tailwind utilities.

**To run the color check manually:**

```bash
task check:colors
```

This check is integrated into the frontend quality suite and pull-request CI.
