import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { findUnique } from './install-launch-smoke.mjs';

test('finds exactly one nested installer artifact', (context) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-release-smoke-test-'));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const nested = path.join(root, 'bundle', 'msi');
  fs.mkdirSync(nested, { recursive: true });
  const installer = path.join(nested, 'Auralis.msi');
  fs.writeFileSync(installer, 'fixture');

  assert.equal(findUnique(root, (file) => file.endsWith('.msi'), 'MSI'), installer);
});

test('rejects missing and ambiguous installer artifacts', (context) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-release-smoke-test-'));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  assert.throws(() => findUnique(root, (file) => file.endsWith('.msi'), 'MSI'), /found 0/);
  fs.writeFileSync(path.join(root, 'one.msi'), 'fixture');
  fs.writeFileSync(path.join(root, 'two.msi'), 'fixture');
  assert.throws(() => findUnique(root, (file) => file.endsWith('.msi'), 'MSI'), /found 2/);
});
