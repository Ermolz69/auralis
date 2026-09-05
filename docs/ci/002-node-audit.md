# Node Audit

## Why is it needed

Checks both runtime and development dependencies. Build/test tools can process untrusted
repository inputs, so a dev-only dependency is not automatically exempt from security review.

## What does it forbid

`task sec:audit:npm` runs `pnpm audit` without a production-only filter, severity downgrade,
or advisory ignore list. Reported vulnerabilities fail the gate.

## Where does it run

In the independent `Dependency Security` CI job when dependency inputs or CI setup
change, and in the reusable full release check. `task check:quality:security`
also runs Rust checks; see [Rust dependency security](010-rust-security.md).

## How to fix the error

Update the responsible dependency requirement or add a reviewed override in
`pnpm-workspace.yaml`, then run `task fe:install-update`. Commit the resulting lockfile
and verify `task sec:audit:npm`, frontend tests, and affected builds. Do not manually edit
resolved versions in the lockfile.

The existing `browserslist: 4.28.7` override covers the patched version for
[GHSA-c83g-rgw3-j3cx](https://github.com/advisories/GHSA-c83g-rgw3-j3cx) and
[GHSA-73wf-gq98-2v4g](https://github.com/advisories/GHSA-73wf-gq98-2v4g).
The lockfile resolves only that version; `task q:ci-bootstrap` rejects a regression
to affected Browserslist versions, including the Storybook/Babel development chain.

## When can an exception be made

Only after an explicit risk review identifies affected paths, mitigations, an owner,
and an expiry or removal condition. Development-only use is not sufficient justification.

## Who approves the exception

The project owner or designated security reviewer. Do not add an exception merely to make CI green.
