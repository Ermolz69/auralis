import assert from 'node:assert/strict';
import { test } from 'node:test';
import { budgetViolations, measureBundle } from './check-bundle.mjs';

test('initial budget includes transitive shared chunks but excludes lazy routes', () => {
  const manifest = {
    entry: {
      file: 'entry.js',
      isEntry: true,
      imports: ['vendor'],
      dynamicImports: ['lazy'],
      css: ['app.css'],
    },
    vendor: { file: 'vendor.js', imports: ['shared'] },
    shared: { file: 'shared.js', imports: ['vendor'], css: ['app.css'] },
    lazy: { file: 'lazy.js', isDynamicEntry: true, imports: ['vendor'] },
  };
  const assets = {
    'entry.js': 'a'.repeat(100),
    'vendor.js': 'b'.repeat(200),
    'shared.js': 'c'.repeat(30),
    'lazy.js': 'd'.repeat(400),
    'app.css': 'e'.repeat(50),
  };
  const sizes = measureBundle(manifest, (file) => assets[file]);
  assert.equal(sizes.initialJs, 330);
  assert.equal(sizes.totalJs, 730);
  assert.equal(sizes.largestJs, 400);
  assert.equal(sizes.totalCss, 50);
  assert.ok(sizes.initialGzip < sizes.totalGzip);
  assert.equal(budgetViolations(sizes).length, 0);
  assert.equal(budgetViolations(sizes, { totalJs: 729 }).length, 1);
  assert.equal(budgetViolations(sizes, { initialJs: 329 }).length, 1);
  assert.equal(budgetViolations(sizes, { largestJs: 399 }).length, 1);
});

test('missing manifests and broken static dependency graphs fail closed', () => {
  assert.throws(() => measureBundle({}, () => ''), /no entry/);
  assert.throws(
    () => measureBundle({ entry: { file: 'e.js', isEntry: true, imports: ['missing'] } }, () => ''),
    /Missing imported/,
  );
  assert.equal(budgetViolations({ totalJs: NaN }, { totalJs: 10 }).length, 1);
});

test('size metrics measure UTF-8 bytes rather than string length', () => {
  const sizes = measureBundle({ entry: { file: 'e.js', isEntry: true } }, () => 'Привет');
  assert.equal(sizes.totalJs, 12);
});
