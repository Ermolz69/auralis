# Node Audit

## Why is it needed
Ensures the security of the application by preventing vulnerable npm packages (with known CVEs) from entering the production build.

## What does it forbid
It forbids the use of dependencies that contain critical vulnerabilities.

## Where does it run
In the main CI pipeline (`ci.yml`) on every pull request and push to the `main`/`develop` branches.

## How to fix the error
Update the vulnerable package (`pnpm update <package-name>`) to a safe version, or run `pnpm audit fix`.

## When can an exception be made
If the vulnerability is in a package used exclusively during development/testing (e.g., an old testing plugin) and the risk of its exploitation in production is exactly zero.

## Who approves the exception
Tech Lead or Security Engineer. Exceptions are configured via `pnpm audit` settings.
