# Auralis Design System

## Purpose

The Auralis design system is the shared contract between product design,
frontend implementation, Storybook examples, and browser tests. It keeps a
desktop application for long-running media operations calm, readable, and
predictable across every supported color theme and workflow state.

The canonical implementation sources are:

- `apps/desktop/src/app/styles/theme.css` for semantic tokens, motion, and theme
  overrides;
- `apps/desktop/src/shared/theme/config/colorThemes.ts` for the selectable theme
  registry;
- `apps/desktop/src/shared/ui` for reusable primitives;
- `apps/desktop/src/shared/ui/design-tokens` for visual foundation stories;
- `apps/desktop/.storybook` for the design-system catalog configuration.

Storybook documents the contract, but production components and tokens remain
the source of truth. Do not create Storybook-only versions of production UI.

## Design principles

1. **Status before decoration.** A long-running operation explains its current
   stage, whether navigation is safe, and what recovery is available.
2. **Semantic roles before palette values.** Components request `surface`,
   `text`, `primary`, or `danger`; the selected theme decides the concrete color.
3. **One visual hierarchy.** A surface has one clearly dominant action and a
   predictable heading, content, and action order.
4. **Short and purposeful motion.** Animation clarifies entry, focus, and state
   changes without delaying interaction.
5. **Accessibility is behavior.** Keyboard order, focus restoration, accessible
   names, live feedback, and reduced motion belong to the component contract.
6. **Local-first states are normal states.** Empty, loading, retry, interrupted,
   and draft-recovery experiences must be intentionally designed.

## Semantic color system

Use Tailwind utilities backed by tokens from `theme.css`. The main token groups
are:

| Role                | Tokens                                                          | Intended use                                               |
| ------------------- | --------------------------------------------------------------- | ---------------------------------------------------------- |
| Canvas              | `canvas`, `bg`                                                  | Window background and primary application regions          |
| Surfaces            | `surface`, `surface-raised`, `surface-hover`, `surface-active`  | Cards, panels, menus, and interaction states               |
| Borders             | `border`, `border-strong`                                       | Separation, field outlines, and stronger boundaries        |
| Content             | `text`, `muted`, `subtle`, `disabled`                           | Primary, supporting, low-emphasis, and unavailable content |
| Primary action      | `primary-action`, hover, pressed, foreground, and soft variants | The dominant action and its states                         |
| Accent              | `accent`, `accent-foreground`, `accent-soft`                    | Selection, information, and secondary emphasis             |
| Status              | `success`, `warning`, `danger` families                         | Completed, cautionary, failed, and destructive states      |
| Focus and selection | `focus`, `focus-ring-color`, `selection-color`                  | Keyboard focus and text selection                          |
| Overlay and shadow  | `overlay`, `shadow-tint`                                        | Modal backdrops and elevation tint                         |

Example:

```tsx
<section className="border border-border bg-surface text-text">
  <p className="text-muted">Supporting description</p>
  <Button>Continue</Button>
</section>
```

Do not use raw hexadecimal colors or Tailwind palette utilities such as
`bg-red-500` in pages, widgets, features, entities, or shared UI. Palette
literals belong only in `theme.css`; the branded Storybook manager theme is a
separate development-tool configuration.

`task q:color-tokens` rejects raw colors and unknown semantic color utilities in
production UI source.

## Supported themes

`COLOR_THEMES` is the registry for theme IDs, labels, descriptions, and light or
dark appearance. The current themes are:

| ID          | Label         | Appearance    |
| ----------- | ------------- | ------------- |
| `auralis`   | Auralis Mint  | Dark, default |
| `abyss`     | Abyss Cyan    | Dark          |
| `indigo`    | Indigo Night  | Dark          |
| `ember`     | Ember Noir    | Dark          |
| `violet`    | Violet Signal | Dark          |
| `frost`     | Auralis Frost | Light         |
| `polar`     | Polar Blue    | Light         |
| `sandstone` | Sandstone     | Light         |

The production `ThemeProvider` persists the user's application theme and applies
it through `data-color-theme` on the document root. Storybook has an independent
toolbar global that applies the same production theme IDs only to the rendered
preview. Switching the Storybook manager theme or the application preview theme
must never mutate production preferences.

When adding a theme:

1. Add its metadata to `COLOR_THEMES`.
2. Add a matching `[data-color-theme='id']` override in `theme.css`.
3. Override every semantic role whose default value is unsuitable.
4. Review it in `Design System / Foundations / Theme Gallery`.
5. Run `task check:colors` and `task check:storybook`.

## Typography, spacing, and elevation

- The sans-serif stack starts with Inter and falls back to Segoe UI Variable,
  Segoe UI, and the system sans-serif font.
- The monospace stack starts with JetBrains Mono and falls back to Cascadia Code
  and Consolas.
- Prefer the existing Tailwind spacing scale. Repeated arbitrary dimensions are
  a signal to extract a layout primitive or add a documented token.
- Use the shared radius tokens from `xs` through `xl`; related nested surfaces
  should not have a larger radius than their parent.
- Use `shadow-sm`, `shadow-md`, and `shadow-lg` to communicate elevation. A border
  should still define the surface in themes where the shadow is subtle.
- Keep secondary text readable. Do not use opacity as a substitute for the
  `muted`, `subtle`, or `disabled` content roles.

## Motion

The shared duration tokens are:

| Token                      | Duration | Use                                                |
| -------------------------- | -------- | -------------------------------------------------- |
| `--motion-duration-fast`   | 140 ms   | Hover, focus, compact popovers, immediate feedback |
| `--motion-duration-base`   | 180 ms   | Content changes and standard control transitions   |
| `--motion-duration-reveal` | 220 ms   | Dialogs, drawers, and revealed surfaces            |

Use opacity and transform for entrance motion. Avoid animating layout-shifting
properties. Only active progress may loop, and motion must never be the only way
to communicate status. Global reduced-motion rules disable non-essential
animations and transitions when the operating system requests it.

The visual reference lives at `Design System / Foundations / Motion` in
Storybook.

## Shared component inventory

The public `shared/ui` barrel currently exposes:

- actions and status: `Button`, `Badge`, `Progress`, `Notice`, and Toast;
- fields: `Input`, `Textarea`, `Select`, and shared form-field messages;
- surfaces and composition: `Card`, `Dialog`, `Tabs`, `PageLayout`, and
  `StateView`;
- visual language: the typed `Icon` adapter backed by the central icon registry.

Before creating a component:

1. Search `shared/ui` and Storybook for an existing primitive.
2. Extend a compatible public component when the new prop represents reusable
   behavior rather than product-specific policy.
3. Keep a one-off composition in its owning feature or widget.
4. Extract a new shared component when the same visual and behavioral pattern is
   used by at least two independent consumers.
5. Put business rules, data fetching, and Tauri calls outside `shared/ui`.

Cross-domain utilities belong in `shared/lib`. Production icons are imported
through `shared/ui/icon`; direct `lucide-react` imports outside the registry are
rejected. Cross-slice imports use public `index.ts` entrypoints.

## Component state contract

Document and test every applicable state:

- default, hover, focus-visible, active, and disabled;
- empty, loading, progress, success, warning, and error;
- long labels, long paths, narrow desktop width, and dense data;
- keyboard navigation and focus restoration;
- accessible names, descriptions, relationships, and live regions;
- all supported themes and reduced-motion behavior.

Not every primitive needs every state. Omitted states should be intentional and
obvious from the public API and Storybook documentation.

## Accessibility requirements

- Prefer native elements before adding ARIA.
- Every control has an accessible name and visible focus indication.
- Fields connect labels, helper messages, and validation errors.
- Dialogs trap focus while open, restore it when closed, and support Escape.
- Tabs expose correct tab, tabpanel, selection, and keyboard relationships.
- Determinate progress exposes a value; indeterminate progress explains the
  active operation without a false percentage.
- Persistent status uses `role="status"`; urgent feedback uses `role="alert"`
  only when an immediate announcement is necessary.
- Color, animation, or icon shape must not be the only status signal.

Storybook accessibility violations fail browser story tests. This does not
replace keyboard and screen-reader reasoning in component tests and review.

## Definition of done

A public visual change is ready when:

- it reuses the narrowest suitable component and respects FSD boundaries;
- all colors use existing semantic roles or introduce a complete theme-aware
  token;
- applicable visual, data, and interaction states are documented;
- the story description explains purpose rather than restating the component
  name;
- keyboard, focus, and accessible relationships are verified;
- `task check:colors`, `task check:quality:frontend`, and
  `task check:storybook` pass;
- product behavior remains production-backed and Storybook-specific adapters do
  not hide unsupported functionality.

See [Storybook conventions](./005-storybook-conventions.md) for catalog structure,
templates, testing, and publication.
