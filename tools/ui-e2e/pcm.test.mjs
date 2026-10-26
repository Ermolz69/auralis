import assert from 'node:assert/strict';
import path from 'node:path';
import { test } from 'node:test';
import { pinnedPcmHelper, numericalLibraryFiles } from './pcm.mjs';

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

test('numerical dependency pins include soundfile typing and cffi parser without engine packages', () => {
  const names = [
    'numpy/core.py',
    'scipy/signal.py',
    'soundfile.py',
    '_soundfile.py',
    '_soundfile_data/libsndfile.dll',
    'cffi/api.py',
    '_cffi_backend.pyd',
    'typing_extensions.py',
    'pycparser/c_parser.py',
    'torch/__init__.py',
    'transformers/modeling.py',
  ];
  const files = names.map((name) => ({
    path: `tts/runtime/pinned/venv/Lib/site-packages/${name}`,
  }));
  assert.deepEqual(numericalLibraryFiles(files), files.slice(0, -2));
});
