import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { expectedReleaseTag, validateReleaseTag } from './validate-release-tag.mjs';

test('derives and accepts the only release tag for the package version', (context) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-release-tag-'));
  context.after(() => fs.rmSync(root, { recursive: true, force: true }));
  fs.writeFileSync(path.join(root, 'package.json'), JSON.stringify({ version: '1.2.3' }));

  assert.equal(expectedReleaseTag(root), 'app-v1.2.3');
  assert.doesNotThrow(() => validateReleaseTag(root, 'app-v1.2.3'));
  assert.throws(() => validateReleaseTag(root, 'app-v1.2.4'), /must be app-v1\.2\.3/);
});
