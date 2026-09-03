import assert from 'node:assert/strict';
import { test } from 'node:test';
import { checkLockfile, checkPackages, checkResolved } from './check-sqlite-dependencies.mjs';

const packages = ['sqlx', 'sqlx-core', 'sqlx-macros', 'sqlx-macros-core', 'sqlx-sqlite'].map(
  (name) => ({
    name,
    version: '0.9.0',
    id: name,
  }),
);
const lock = (entries) =>
  entries
    .map(({ name, version }) => `[[package]]\nname = "${name}"\nversion = "${version}"\n`)
    .join('\n');
const metadata = () => ({
  packages: structuredClone(packages),
  resolve: {
    nodes: [
      {
        id: 'sqlx',
        features: [
          '_rt-tokio',
          '_sqlite',
          'derive',
          'runtime-tokio',
          'sqlite-bundled',
          'sqlx-macros',
          'sqlx-sqlite',
        ],
      },
      { id: 'sqlx-sqlite', features: ['chrono', 'bundled'] },
    ],
  },
});

test('accepts the SQLite-only lockfile with either line ending', () => {
  checkLockfile(lock(packages));
  checkLockfile(lock(packages).replaceAll('\n', '\r\n'));
  checkResolved(metadata());
});

test('rejects forbidden packages even when they are not active dependency nodes', () => {
  for (const name of ['rsa', 'sqlx-mysql', 'sqlx-postgres']) {
    const entry = { name, version: '0.9.0' };
    assert.throws(() => checkLockfile(lock([...packages, entry])), /dependency returned/);
    const graph = metadata();
    graph.packages.push(entry);
    assert.throws(() => checkResolved(graph), /dependency returned/);
  }
});

test('rejects missing, duplicate, stale and mixed SQLx packages', () => {
  assert.throws(() => checkLockfile('version = 4\n'));
  assert.throws(() => checkLockfile('[[package]]\nname = "sqlx"\n'));
  assert.throws(() => checkPackages(packages.slice(1)));
  assert.throws(() => checkPackages([...packages, packages[0]]));
  for (const version of ['0.8.6', '0.9.1']) {
    assert.throws(() => checkPackages([...packages.slice(1), { name: 'sqlx', version }]));
  }
});

test('rejects feature unification that re-enables unnecessary SQLx capabilities', () => {
  for (const feature of [
    'default',
    'mysql-rsa',
    'mysql',
    'postgres',
    'any',
    'macros',
    'chrono',
    'tls-rustls',
  ]) {
    const graph = metadata();
    graph.resolve.nodes[0].features.push(feature);
    assert.throws(() => checkResolved(graph));
  }
  for (const feature of ['load-extension', 'deserialize', 'unbundled', 'preupdate-hook']) {
    const graph = metadata();
    graph.resolve.nodes[1].features.push(feature);
    assert.throws(() => checkResolved(graph));
  }
  for (const feature of ['chrono', 'bundled']) {
    const graph = metadata();
    graph.resolve.nodes[1].features = graph.resolve.nodes[1].features.filter(
      (value) => value !== feature,
    );
    assert.throws(() => checkResolved(graph));
  }
});
