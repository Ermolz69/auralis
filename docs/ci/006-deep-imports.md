# Deep Imports

## Why is it needed
Ensures strict encapsulation of FSD modules. Layers must communicate with each other as black boxes, exposing only what is explicitly exported via the public API (`index.ts`).

## What does it forbid
It forbids direct imports of the internals of another slice (e.g., `import { LoginForm } from "@/features/auth/ui/LoginForm"` is a severe violation).

## Where does it run
In the CI pipeline via the ESLint rule `no-restricted-imports` within the `task q:fsd-boundaries` command.

## How to fix the error
Change the import path to the public API: `import { LoginForm } from "@/features/auth"`. If the required module is not yet exported, add its export to the `index.ts` of the target slice. Within the same slice (for connecting internal files to each other), use relative paths (`../ui/LoginForm`).

## When can an exception be made
Never. The public API is the absolute foundation of encapsulation. Bypassing the public API breaks feature isolation.

## Who approves the exception
There are no exceptions. This is the strictest architectural rule.
