# Raw Colors Prohibition

## Why is it needed

To keep all current dark and light themes consistent, component colors must be
selected through semantic tokens defined in `theme.css`.

## What does it forbid

The current check scans TypeScript, TSX, CSS, and SCSS under `pages`, `features`,
`widgets`, `entities`, and `shared/ui`. It rejects raw hexadecimal colors and
Tailwind color utilities whose token is not declared in `theme.css`.

## Where does it run

In the CI pipeline via the `task q:color-tokens` command (runs the `check-raw-colors.mjs` script).

## How to fix the error

Replace the hardcoded color with the correct semantic token (for example,
`bg-surface`, `text-primary`, `border-border`, or `var(--color-border)`). Add a
new semantic token only when no existing role fits.

## When can an exception be made

Palette literals are defined in `apps/desktop/src/app/styles/theme.css`. Design
token stories are also excluded so they can render the palette reference. Other
exceptions must remain narrow and explicit in the checker.

## Who approves the exception

Lead Designer or Tech Lead.
