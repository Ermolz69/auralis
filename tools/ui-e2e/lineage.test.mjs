import assert from 'node:assert/strict';
import { test } from 'node:test';
import { OwnedTree } from './lineage.mjs';
const process = (pid, parent, name, seconds = 0) => ({
  pid,
  parent,
  name,
  created: new Date(Date.now() - 60000 + seconds * 1000).toISOString(),
});
test('cleanup retains non-model WebView descendants after their parent exits', () => {
  const root = process(10, 1, 'auralis.exe');
  const webview = process(20, 10, 'msedgewebview2.exe', 1);
  const renderer = process(30, 20, 'renderer.exe', 2);
  const tree = new OwnedTree(10);
  assert.equal(tree.observe([root, webview, renderer]).length, 3);
  assert.equal(tree.observe([renderer]).length, 1);
  assert.equal(tree.observe([]).length, 0);
  assert.equal(tree.observe([process(30, 1, 'unrelated.exe', 5)]).length, 0);
});
test('missing process identity fails closed instead of claiming cleanup', () => {
  assert.throws(() => new OwnedTree(10).observe([]), /not observed/);
});

test('a late child or opaque orphan across a sampling gap prevents confirmed cleanup', () => {
  const root = process(10, 1, 'auralis.exe'),
    worker = process(20, 10, 'worker.exe', 1);
  const tree = new OwnedTree(10);
  tree.observe([root, worker]);
  const late = { ...process(30, 20, 'late.exe'), created: new Date(Date.now() + 10).toISOString() };
  tree.observe([root, late]);
  assert.throws(() => tree.assertKnown(), /observation gap/);
  const orphanTree = new OwnedTree(10);
  orphanTree.observe([root]);
  orphanTree.observe([root, process(40, 999, 'orphan.exe', 2)]);
  assert.throws(() => orphanTree.assertKnown(), /observation gap/);
});
