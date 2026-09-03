import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { test } from 'node:test';
import { createUpstreamDirectory, patchIterator, patchManifest, verifyTrees } from './package.mjs';

const source = 'let p: *mut libc::c_char = std::ptr::null_mut();\n                &p,\n';

test('upstream sources are isolated per invocation outside the Cargo target cache', async () => {
  const directories = await Promise.all([createUpstreamDirectory(), createUpstreamDirectory()]);
  for (const directory of directories) {
    assert.equal(path.dirname(directory), path.resolve(os.tmpdir()));
    assert.ok(path.basename(directory).startsWith('auralis-glib-source-'));
  }
  try {
    assert.notEqual(directories[0], directories[1]);
  } finally {
    for (const directory of directories) await rm(directory, { recursive: true });
  }
});

test('the patch requires the exact single upstream out-pointer operation', () => {
  assert.match(patchIterator(source), /let mut p:/);
  assert.match(patchIterator(source), /&mut p,/);
  assert.throws(() => patchIterator(source + source));
  assert.throws(() => patchIterator(patchIterator(source)));
});

test('third-party lint compatibility is restricted to two style checks', () => {
  const manifest = patchManifest('[package]\nname = "glib"\n');
  assert.match(manifest, /unused_parens = "allow"/);
  assert.match(manifest, /mismatched_lifetime_syntaxes = "allow"/);
  assert.doesNotMatch(manifest, /warnings =|unsafe|clippy|expect_used|unwrap_used/);
  assert.throws(() => patchManifest(manifest));
});

test('the package gate rejects reverted fixes, other edits and additional files', async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), 'auralis-glib-integrity-'));
  assert.equal(path.dirname(directory), path.resolve(os.tmpdir()));
  const original = path.join(directory, 'original');
  const actual = path.join(directory, 'actual');
  try {
    for (const base of [original, actual]) {
      await mkdir(path.join(base, 'src'), { recursive: true });
      await writeFile(path.join(base, 'LICENSE'), 'upstream license');
      await writeFile(path.join(base, 'src/variant_iter.rs'), source);
    }
    await assert.rejects(verifyTrees(original, actual), /Unexpected modification/);
    await writeFile(path.join(actual, 'src/variant_iter.rs'), patchIterator(source));
    assert.equal(await verifyTrees(original, actual), 2);
    await writeFile(path.join(actual, 'LICENSE'), 'changed');
    await assert.rejects(verifyTrees(original, actual), /LICENSE/);
    await writeFile(path.join(actual, 'LICENSE'), 'upstream license');
    await writeFile(path.join(actual, 'extra.rs'), 'unexpected');
    await assert.rejects(verifyTrees(original, actual), /file set differs/);
  } finally {
    await rm(directory, { recursive: true });
  }
});
