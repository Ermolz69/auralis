# Raw Colors Prohibition

## Why is it needed

To ensure the application design is 100% consistent and easily supports theming (Dark Mode / Light Mode), all colors must be managed centrally through the design system tokens.

## What does it forbid

It forbids the direct use of hardcoded colors in any form: HEX codes (`#FFF`), functions (`rgb(...)`, `hsl(...)`) in Tailwind classes (`bg-[#fff]`), inline styles, or CSS files.

## Where does it run

In the CI pipeline via the `task q:color-tokens` command (runs the `check-raw-colors.mjs` script).

## How to fix the error

Replace the hardcoded color with the correct semantic token from your design system (e.g., `bg-surface`, `text-primary`, `var(--color-border)`).

## When can an exception be made

Exceptions are allowed only for files where these tokens are actually defined (e.g., `theme.css`, `tokens.css`, `tailwind.config.ts`). The script already contains a whitelist for such files.

## Who approves the exception

Lead Designer or Tech Lead.
