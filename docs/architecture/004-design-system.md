# Design System Rules

This document outlines the core rules for building the Auralis UI to maintain consistency and prevent design debt.

## 1. Colors and Theme Tokens

Always use Tailwind theme tokens instead of raw colors.

- **Canvas/background**: Use `bg-canvas` for the window canvas and `bg-bg` for primary application areas.
- **Cards/panels**: Use `bg-surface`, `bg-surface-raised`, and the surface interaction tokens.
- **Primary actions**: Use `bg-primary-action` with its hover, pressed, foreground, and soft variants.
- **Status Colors**:
  - **Danger/Error**: Use `bg-danger` (or `text-danger`) for destructive actions, deletions, and error messages.
  - **Success/Warning**: Use `success` and `warning` token families.
- **Typography**: Use `text-text` for primary readable content and `text-muted` for secondary text, descriptions, and placeholders.
- **Borders**: Use `border-border` and `border-border-strong`.

`theme.css` currently defines five dark themes and three light themes. The theme
registry in `shared/theme/config/colorThemes.ts` is the source of selectable theme
identifiers and metadata.

## 2. Sizes and Component States

- **Button Sizes**: Limit buttons to 3 distinct sizes. Do not use arbitrary padding classes (like `px-7 py-2.5`) directly in feature code.
  - `sm`: Small actions within lists or headers.
  - `md`: Standard actions.
  - `lg`: Main call-to-actions on empty states or landing pages.
- **States**:
  - **Hover**: Use fractional opacity (`hover:bg-primary/90`) or subtle background shifts (`hover:bg-bg` over `bg-surface`).
  - **Disabled**: Reduce opacity and disable pointer events (`opacity-50 cursor-not-allowed`).
  - **Loading**: Apply the disabled state visually and display a spinner or loading text.
  - **Error**: Highlight the border with `border-danger` or display a `text-danger` helper message.

## 3. Raw Hex Usage

To preserve all supported themes:

- **Forbidden**: NEVER use raw hex values (`#FFFFFF`, `bg-[#1a1a1a]`) or hardcoded utility colors (`bg-red-500`) in feature code, pages, or components.
- **Allowed**: Palette literals belong only in
  `apps/desktop/src/app/styles/theme.css`, where Tailwind v4 `@theme` variables and
  theme overrides are defined. `index.css` imports Tailwind and the theme and owns
  global base/utility styles.

## 4. Shared Components Architecture

Follow these rules for populating `shared/ui`:

- **When to reuse**: Before creating a new button, input, or card, look in `apps/desktop/src/shared/ui`. If an existing component can solve your problem with minor prop additions, extend and use it.
- **When to create new**: If a UI pattern (e.g., a specific stylized panel, a complex input field, or an alert banner) is duplicated in 2 or more distinct widgets or pages, abstract it into a generic `shared/ui` component. Do not write the raw Tailwind layout twice.
- **Shared behavior**: Cross-domain utilities, including locale formatting and typed browser event channels, belong in `shared/lib`; entities and features must not keep local copies of the same formatter or subscription plumbing.
- **Icons**: Production UI imports icons only through `shared/ui/icon`. Direct `lucide-react` imports outside the central registry are rejected by the frontend architecture gate.
- **Copy-paste gate**: `task q:duplicate-code` rejects exact production blocks of 12 or more meaningful lines. Extract the behavior at the narrowest common FSD layer instead of suppressing the check.
- **Theme validation**: `task q:color-tokens` rejects raw hex colors and unknown semantic color utilities in restricted FSD source directories.
