# Auralis Desktop

Auralis is a desktop application built with React, TypeScript, and Vite. This document outlines our UI development workflow, design system, and how to maintain visual consistency.

## Design System & UI Components

Our design system relies on a unified set of tokens and strictly isolated UI components. We follow Feature-Sliced Design (FSD) principles.

### Where are the Shared UI Components?

All reusable, generic UI components (Buttons, Inputs, Cards, etc.) are located in:
`apps/desktop/src/shared/ui/`

**Do not** put business logic in `shared/ui`. These components must remain isolated and reusable across the entire app.

### How to Add a New Component

1. Create a new folder in `src/shared/ui/` (e.g., `src/shared/ui/badge/`).
2. Implement the component (e.g., `Badge.tsx`) using Tailwind classes mapped exclusively to our CSS variables (e.g., `bg-primary`, `text-muted`).
3. Ensure it supports standard accessibility (ARIA labels, keyboard navigation, focus states).
4. Add a `[ComponentName].stories.tsx` file in the same directory.
5. Export it from an `index.ts` file.

### How to Add a Story

We use Storybook to develop and visually test our components in isolation.
When you add or update a component, create a `.stories.tsx` file next to it. Provide examples of all its variants, sizes, and states (including `disabled` and `error` states).

**To run Storybook locally:**

```bash
pnpm --filter desktop run storybook
```

## Theming & Colors

We use a strict **CSS Variable-based Theme** defined in `apps/desktop/src/app/styles/theme.css`.

### Why are Raw Hex Colors Forbidden?

Using arbitrary hex colors (e.g., `text-[#ff0000]`) or raw Tailwind color scales (e.g., `text-red-500`) directly in feature or page code breaks visual consistency and prevents us from supporting dynamic themes (like light/dark mode) in the future.

All colors **must** be routed through our semantic tokens (e.g., `primary`, `surface`, `danger`, `muted`).

### How to Add a Theme Token

If you genuinely need a new semantic color:

1. Open `apps/desktop/src/app/styles/theme.css`.
2. Add the CSS variable under `:root` (e.g., `--color-accent-hover: #...`).
3. Open `apps/desktop/src/app/styles/index.css` (or wherever your Tailwind `@theme` config resides) and map the CSS variable to a Tailwind token.
4. Update `DesignTokens.stories.tsx` to document the new token visually.

### How to Check for Raw Colors

We enforce color consistency using an automated script that scans the codebase for forbidden hex colors and standard Tailwind utilities.

**To run the color check manually:**

```bash
task check:colors
# or
pnpm --filter desktop run check:colors
```

This check is also integrated into our CI/CD pipeline and the general `task check` suite. If you use a raw color, the build will fail!
