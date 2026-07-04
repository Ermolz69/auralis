# Markdownlint

## Why is it needed
Ensures strict consistency, beautiful presentation, and readability of all project documentation.

## What does it forbid
Violations of heading nesting structures (e.g., jumping from H1 to H3), empty lines in inappropriate places, and trailing spaces. Long lines (MD013) and the use of HTML tags (MD033) are explicitly permitted for flexibility.

## Where does it run
In the CI pipeline via the `task docs:lint` command.

## How to fix the error
Read the error log from `markdownlint-cli2` and fix the formatting (remove extra spaces, align lists). You can verify it locally by running `task docs:lint`.

## When can an exception be made
When importing huge chunks of external documentation, third-party licenses, or generating automated changelogs.

## Who approves the exception
Tech Lead or Team Lead (by adding the file to `.markdownlint-cli2.jsonc` ignores).
