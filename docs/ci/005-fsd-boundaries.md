# FSD Boundaries

## Why is it needed
Ensures that the architecture does not turn into a big ball of mud by strictly controlling the direction of dependencies between architectural layers of the Feature-Sliced Design methodology.

## What does it forbid
It forbids "upward" layer imports. For example, `shared` cannot import `features` or `entities`. The `entities` layer cannot depend on `widgets`, etc.

## Where does it run
In the CI pipeline via the `task q:fsd-boundaries` command, which internally calls ESLint and `eslint-plugin-boundaries`.

## How to fix the error
Fundamentally review the architecture of your solution. If a `shared` component needs a `feature` module, it means either your component is too "smart" and should be elevated to the `feature` level, or the requested logic must be pushed down to the `shared` level.

## When can an exception be made
In extremely rare cases of complex cross-domain composition that does not fit the standard structure, or during gradual refactoring of legacy code.

## Who approves the exception
Architect or Tech Lead. Exceptions are defined via an `eslint-disable` comment with a mandatory explanation.
