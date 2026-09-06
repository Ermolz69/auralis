# Storybook Conventions

## Purpose and scope

Storybook is the interactive catalog for the Auralis design system and product
UI states. It serves four purposes:

- document reusable component contracts and intended usage;
- review foundations, themes, motion, and responsive behavior;
- exercise product states without requiring a running Tauri application;
- run component interaction and accessibility checks in Chromium.

The catalog is not a separate prototype. Stories render production components,
use production semantic tokens, and preserve the current product capabilities.

## Configuration map

| File                                                        | Responsibility                                                                                      |
| ----------------------------------------------------------- | --------------------------------------------------------------------------------------------------- |
| `apps/desktop/.storybook/main.ts`                           | Story and MDX discovery, addons, feature flags, and Storybook-only build policy                     |
| `apps/desktop/.storybook/manager.ts`                        | Manager layout and branded catalog theme                                                            |
| `apps/desktop/.storybook/auralisTheme.ts`                   | Auralis colors and typography for manager and Docs UI                                               |
| `apps/desktop/.storybook/preview.tsx`                       | Production CSS, application-theme toolbar, viewports, Docs defaults, sorting, and global decorators |
| `apps/desktop/.storybook/tauriStoryAdapter.ts`              | Central typed mock IPC boundary for stories that render Tauri-backed UI                             |
| `apps/desktop/src/shared/ui/design-tokens/Introduction.mdx` | Landing documentation, principles, architecture, examples, and definition of done                   |

The Storybook manager theme is intentionally independent from the application
theme selected in the preview toolbar. Changing either one must not write to the
production theme preference.

## Catalog hierarchy

Every `meta.title` starts with one of two roots:

```text
Design System
├── Introduction
├── Foundations
│   ├── Tokens
│   ├── Motion
│   └── Theme Gallery
└── Components
    └── [Component]

Product
├── Pages
│   └── [Page]
├── Features
│   └── [Feature]
└── Widgets
    └── [Widget]
```

Use these title patterns:

- `Design System/Components/Button`
- `Product/Features/Import Local Media/States`
- `Product/Widgets/Job Queue Panel/States`
- `Product/Pages/Project/Workspace`

Do not reintroduce legacy roots such as `Shared UI`, `Features`, `Widgets`, or
`Pages`. The contract checker rejects them and duplicate catalog paths.

## File placement

- A component story lives next to the component it documents.
- A page, widget, or feature state catalog lives in the owning slice.
- A foundation reference lives under `shared/ui/design-tokens` when it only
  depends on the shared UI slice.
- A cross-slice showcase belongs under `app/stories`, where composing shared
  modules does not violate FSD direction. The theme gallery is the current
  example.
- MDX is reserved for narrative documentation and curated examples; behavioral
  examples remain CSF story files.

Typical component layout:

```text
shared/ui/button/
├── Button.tsx
├── Button.test.tsx
├── Button.stories.tsx
└── index.ts
```

## Required metadata

Every story file must declare typed CSF metadata, a unique stable title,
Autodocs, a useful component description, and at least one named story export.

```tsx
import type { Meta, StoryObj } from '@storybook/react-vite';
import { Button } from './Button';

const meta = {
  title: 'Design System/Components/Button',
  component: Button,
  parameters: {
    layout: 'centered',
    docs: {
      description: {
        component:
          'Primary action primitive with semantic variants, loading feedback, and disabled behavior.',
      },
    },
  },
  tags: ['autodocs'],
  argTypes: {
    variant: {
      control: 'select',
      options: ['primary', 'secondary', 'ghost', 'danger'],
    },
  },
} satisfies Meta<typeof Button>;

export default meta;
type Story = StoryObj<typeof meta>;
```

Descriptions explain purpose, boundaries, or important behavior. Avoid empty
phrases such as “Button component” or descriptions that only repeat the title.

Expose controls only for meaningful public props. Use finite option lists for
variants and sizes, and do not add story-only props to a production component.

## Story design

Prefer `args` for one component instance and `render` for a comparison,
composition, or provider-backed scenario.

```tsx
export const Primary: Story = {
  args: {
    children: 'Start import',
    variant: 'primary',
  },
};

export const AllVariants: Story = {
  render: () => (
    <div className="flex gap-3">
      <Button variant="primary">Primary</Button>
      <Button variant="secondary">Secondary</Button>
      <Button variant="ghost">Ghost</Button>
      <Button variant="danger">Danger</Button>
    </div>
  ),
};
```

Use realistic, privacy-safe content. Long filenames, URLs, titles, progress
messages, and metadata should resemble the production domain without containing
personal paths, access tokens, or private resources.

Do not use timers to fake a product result when a stable state fixture is enough.
Stories should be deterministic and independent of execution order.

## State coverage

For each public surface, document the applicable rows of this matrix:

| Area            | Expected examples                                                      |
| --------------- | ---------------------------------------------------------------------- |
| Visual variants | Primary, secondary, ghost, danger, semantic tones, or sizes            |
| Interaction     | Default, hover or focus-visible review, active, disabled, loading      |
| Data lifecycle  | Empty, loading, ready, progress, success, warning, error, retry        |
| Content stress  | Long text, long filename, missing metadata, dense content              |
| Layout          | 1440, 1280, 1024, and 800 pixel desktop widths where relevant          |
| Accessibility   | Keyboard order, focus restoration, labels, relationships, live regions |
| Themes          | All dark and light palettes for shared or high-risk visual surfaces    |

Comparison stories such as `AllVariants` make visual review fast. Individual
stories remain useful when a state needs controls, interaction assertions, or a
stable visual snapshot.

## Interaction tests

Add a `play` function when behavior, keyboard navigation, focus, or accessible
relationships are part of the scenario.

```tsx
import { expect, userEvent, within } from 'storybook/test';

export const KeyboardNavigation: Story = {
  render: () => <ProjectTabs />,
  play: async ({ canvasElement }) => {
    const canvas = within(canvasElement);
    const transcript = canvas.getByRole('tab', { name: 'Transcript' });

    transcript.focus();
    await userEvent.keyboard('{ArrowRight}');
    await expect(canvas.getByRole('tab', { name: 'Media' })).toHaveFocus();
  },
};
```

Query by role, accessible name, label, or visible text. Avoid selectors tied to
CSS implementation. Assertions should prove user-visible behavior rather than
the internal state of a component.

The global accessibility configuration uses `test: 'error'`. An accessibility
violation therefore fails the browser story test instead of producing an
advisory-only result.

## Tauri-backed stories

`tauriStoryAdapter.ts` installs one centralized `mockIPC` implementation before
stories render. It uses the frontend `CommandMap` result and argument types so
fixtures remain aligned with the IPC contract.

When a story needs another command:

1. Add an explicit command branch to the shared adapter.
2. Type its input and output through `CommandMap`.
3. Return deterministic, privacy-safe data.
4. Keep state local to the adapter or story and reset it when isolation requires
   it.
5. Do not perform network, filesystem, or real Tauri side effects.

The adapter is a rendering boundary, not proof that native integration works.
Real React → IPC → Rust → SQLite → filesystem coverage belongs to
`task desktop:e2e:native`.

## Themes and responsive review

The **Auralis theme** toolbar control is generated from `COLOR_THEMES`; do not
maintain a second list in Storybook. Review shared components and important
product surfaces in at least one dark and one light theme. Use the Theme Gallery
for side-by-side palette review.

The configured desktop viewports are:

- Desktop: 1440 × 900;
- Desktop: 1280 × 800;
- Tablet: 1024 × 768;
- Compact: 800 × 700.

These are review sizes, not a promise that the packaged desktop window supports
arbitrarily small dimensions. Page stories may add scenario-specific viewport
parameters when their layout contract requires exact sizes.

## Local workflow

Install dependencies and Chromium once on a new machine:

```bash
task install:frontend
task fe:setup:playwright
```

Use the focused commands from the repository root:

```bash
# Interactive catalog on http://localhost:6006
task fe:storybook:dev

# Fast metadata and hierarchy contract
task q:storybook-contract

# Browser stories and accessibility only
pnpm --filter desktop test:storybook

# Production-static catalog only
task fe:storybook

# Complete Storybook gate
task check:storybook
```

`task check:storybook` runs the metadata contract and its fixtures, all stories
in Chromium, accessibility checks, and the static build. Static output under
`apps/desktop/storybook-static` is generated and must not be committed.

`task check:frontend` already runs Storybook browser tests as part of the full
frontend test suite. `task check:quality:frontend` runs the faster Storybook
metadata contract. Pull-request CI therefore covers behavior and catalog
structure without repeating the static Pages build.

## Publication

`.github/workflows/storybook-pages.yml` builds the catalog after changes reach
`main` and can also run manually. It uploads `.pages` as a GitHub Pages artifact;
generated Storybook output is never committed.

Published catalog:
[Auralis Storybook](https://ermolz69.github.io/auralis/docs/storybook/)

## Review checklist

- The story title uses the canonical hierarchy and is unique.
- Autodocs and a purposeful component description are present.
- At least one explicit story demonstrates the normal state.
- Applicable variants, lifecycle states, stress content, and viewports are
  represented.
- Interactive behavior has role-based `play` assertions.
- Fixtures are deterministic and contain no private user data.
- Production semantic tokens and components are used.
- A dark and light theme have been reviewed where visual contrast matters.
- No FSD exception was introduced for documentation code.
- `task check:storybook` passes before review.

## Common failures

| Failure                                                  | Resolution                                                                   |
| -------------------------------------------------------- | ---------------------------------------------------------------------------- |
| `meta.parameters.docs.description.component is required` | Add a purpose and behavior description to the meta parameters                |
| `title must start with Design System/ or Product/`       | Move the story into the canonical catalog hierarchy                          |
| Duplicate title                                          | Give each component or state catalog a unique stable path                    |
| Chromium executable missing                              | Run `task fe:setup:playwright`                                               |
| FSD boundary error in a showcase                         | Move cross-slice composition to `app/stories` or import through a public API |
| Story works alone but fails in the suite                 | Remove order-dependent state and reset mutable fixtures                      |
| Tauri command returns `null`                             | Add a typed deterministic branch to the shared story adapter                 |
| Raw color check fails                                    | Replace the value with a semantic token or complete the theme token workflow |

See [Auralis Design System](./004-design-system.md) for tokens, reuse rules,
motion, accessibility, and the component definition of done.
