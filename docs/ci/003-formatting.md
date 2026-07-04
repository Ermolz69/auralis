# Prettier Formatting

## Why is it needed
To maintain an absolutely consistent code style across the repository and prevent long formatting debates during code reviews.

## What does it forbid
It forbids committing code whose style and formatting do not comply with the strict rules defined in `.prettierrc`.

## Where does it run
In the CI pipeline via the `task q:format-check` command.

## How to fix the error
Run the `task q:format-write` command locally before committing. It will automatically format all violating files.

## When can an exception be made
Generated files, lock files (`pnpm-lock.yaml`, `Cargo.lock`), and third-party assets are already ignored automatically in `.prettierignore`. Manual exceptions for regular business logic code are not allowed.

## Who approves the exception
There are no exceptions. Ignore rules must be approved by the Team Lead.
