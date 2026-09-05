import assert from 'node:assert/strict';
import test from 'node:test';

import {
  collectWorkspaceDependencyErrors,
  parseCentralizedDependencies,
  parseWorkspaceMembers,
} from './check-workspace-dependencies.mjs';

const rootManifest = `
[workspace]
members = ["crates/one", "crates/two"]

[workspace.dependencies]
serde = "1.0.228"
tokio = "1.52.4"
`;

test('reads workspace members and centralized dependency names', () => {
  assert.deepEqual(parseWorkspaceMembers(rootManifest), ['crates/one', 'crates/two']);
  assert.deepEqual([...parseCentralizedDependencies(rootManifest)], ['serde', 'tokio']);
});

test('accepts workspace inheritance with crate-specific features', () => {
  const errors = collectWorkspaceDependencyErrors({
    rootManifest,
    memberManifests: new Map([
      [
        'crates/one/Cargo.toml',
        `[dependencies]\nserde = { workspace = true, features = ["derive"] }`,
      ],
      ['crates/two/Cargo.toml', `[dev-dependencies]\ntokio.workspace = true`],
    ]),
  });

  assert.deepEqual(errors, []);
});

test('rejects a local version for a centralized dependency', () => {
  const errors = collectWorkspaceDependencyErrors({
    rootManifest,
    memberManifests: new Map([
      ['crates/one/Cargo.toml', `[dependencies]\ntokio = "1.36"`],
    ]),
  });

  assert.deepEqual(errors, [
    'crates/one/Cargo.toml: tokio must use workspace = true instead of a local version',
  ]);
});

test('rejects workspace inheritance when the root declaration is missing', () => {
  const errors = collectWorkspaceDependencyErrors({
    rootManifest: `[workspace.dependencies]\nserde = "1"`,
    memberManifests: new Map([
      ['crates/one/Cargo.toml', `[dependencies]\ntokio.workspace = true`],
    ]),
  });

  assert.deepEqual(errors, [
    'crates/one/Cargo.toml: tokio inherits from the workspace but is missing from [workspace.dependencies]',
  ]);
});

test('rejects repeated external versions but ignores local path dependencies', () => {
  const errors = collectWorkspaceDependencyErrors({
    rootManifest: `[workspace.dependencies]\nserde = "1"`,
    memberManifests: new Map([
      [
        'crates/one/Cargo.toml',
        `[dependencies]\ntracing = "0.1"\ndomain = { version = "0.1", path = "../domain" }`,
      ],
      [
        'crates/two/Cargo.toml',
        `[dependencies]\ntracing = {\n  version = "0.1.44"\n}\ndomain = { path = "../domain" }`,
      ],
    ]),
  });

  assert.deepEqual(errors, [
    'tracing is versioned independently in multiple manifests: crates/one/Cargo.toml, crates/two/Cargo.toml',
  ]);
});
