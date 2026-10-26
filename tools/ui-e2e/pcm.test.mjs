import assert from 'node:assert/strict';
import path from 'node:path';
import { test } from 'node:test';
import { pinnedPcmHelper } from './pcm.mjs';

test('installed PCM wrapper resolves one pinned helper including wheel src prefix', () => {
  const profile = path.resolve('fixture-profile'),
    packages = path.join(profile, 'tts/packages/pinned');
  const entry = { path: 'src\\tts\\audio\\joiner.py', sha256: 'a'.repeat(64) };
  assert.deepEqual(pinnedPcmHelper(profile, packages, { package_files: [entry] }), {
    helper: path.join(packages, 'src/tts/audio/joiner.py'),
    sha256: entry.sha256,
  });
  assert.throws(() => pinnedPcmHelper(profile, packages, { package_files: [] }), /exactly one/);
  assert.throws(
    () =>
      pinnedPcmHelper(profile, packages, {
        package_files: [entry, { ...entry, path: 'tts/audio/joiner.py' }],
      }),
    /exactly one/,
  );
  assert.throws(
    () =>
      pinnedPcmHelper(profile, packages, {
        package_files: [{ ...entry, path: '../../../../tts/audio/joiner.py' }],
      }),
    /owned/,
  );
});
