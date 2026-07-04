# Design System Rules

This document outlines the core rules for building the Auralis UI to maintain consistency and prevent design debt.

## 1. Colors and Theme Tokens

Always use Tailwind theme tokens instead of raw colors.

- **Background**: Use `bg-bg` for the main application background (pages, full-screen layouts).
- **Cards/Panels**: Use `bg-surface` for distinct UI blocks, sidebars, modals, and cards.
- **Primary Actions**: Use `bg-primary` for main CTA buttons, progress bars, active states, and highlights.
- **Status Colors**:
  - **Danger/Error**: Use `bg-danger` (or `text-danger`) for destructive actions, deletions, and error messages.
  - **Success/Warning**: Use standard semantic colors when introduced (e.g., `text-green-500` if no token exists, or abstract into a token).
- **Typography**: Use `text-text` for primary readable content and `text-muted` for secondary text, descriptions, and placeholders.
- **Borders**: Use `border-muted` for all structural lines and dividers.

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

To ensure our application can support theming (like light/dark mode) seamlessly:

- **Forbidden**: NEVER use raw hex values (`#FFFFFF`, `bg-[#1a1a1a]`) or hardcoded utility colors (`bg-red-500`) in feature code, pages, or components.
- **Allowed**: Raw hex values are ONLY allowed in the global stylesheet (`apps/desktop/src/app/styles/index.css`) to define the CSS variables that power the theme tokens.

## 4. Shared Components Architecture

Follow these rules for populating `shared/ui`:

- **When to reuse**: Before creating a new button, input, or card, look in `apps/desktop/src/shared/ui`. If an existing component can solve your problem with minor prop additions, extend and use it.
- **When to create new**: If a UI pattern (e.g., a specific stylized panel, a complex input field, or an alert banner) is duplicated in 2 or more distinct widgets or pages, abstract it into a generic `shared/ui` component. Do not write the raw Tailwind layout twice.
