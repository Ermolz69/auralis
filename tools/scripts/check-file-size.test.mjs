import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { checkFileSize } from './check-file-size.mjs';

function fixture() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-file-size-'));
}

function writeLines(root, relativePath, count) {
  const target = path.join(root, relativePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, Array.from({ length: count }, (_, i) => `line${i}`).join('\n'));
}

test('passes files at the configured boundary', () => {
  const root = fixture();
  writeLines(root, 'apps/desktop/src/features/demo/Feature.tsx', 250);

  assert.deepEqual(checkFileSize({ rootDir: root }), []);
});

test('fails files beyond the configured boundary', () => {
  const root = fixture();
  writeLines(root, 'apps/desktop/src/features/demo/Feature.tsx', 251);

  assert.deepEqual(checkFileSize({ rootDir: root }), [
    {
      file: 'apps/desktop/src/features/demo/Feature.tsx',
      lines: 251,
      maxLines: 250,
    },
  ]);
});

test('ratchets existing oversized modules without allowing further growth', () => {
  const root = fixture();
  const file = 'apps/desktop/src/widgets/app-shell/ui/AppShell.tsx';
  writeLines(root, file, 636);
  assert.deepEqual(checkFileSize({ rootDir: root }), []);

  writeLines(root, file, 637);
  assert.deepEqual(checkFileSize({ rootDir: root }), [
    {
      file,
      lines: 637,
      maxLines: 636,
    },
  ]);
});

test('keeps generated and static exclusions narrow', () => {
  const root = fixture();
  writeLines(root, 'apps/desktop/src/features/demo/Feature.generated.ts', 999);
  writeLines(root, 'apps/desktop/src/features/demo/constants.ts', 999);

  assert.deepEqual(checkFileSize({ rootDir: root }), []);
});

test('checks production filenames that contain mock', () => {
  const root = fixture();
  writeLines(root, 'crates/application/src/usecases/pipeline/start_mock.rs', 301);

  assert.deepEqual(checkFileSize({ rootDir: root }), [
    {
      file: 'crates/application/src/usecases/pipeline/start_mock.rs',
      lines: 301,
      maxLines: 300,
    },
  ]);
});

test('counts rust production lines without inline cfg test modules', () => {
  const root = fixture();
  writeLines(root, 'crates/application/src/runtime.rs', 299);
  fs.appendFileSync(
    path.join(root, 'crates/application/src/runtime.rs'),
    '\n#[cfg(test)]\nmod tests {\n'.concat('    fn helper() {}\n'.repeat(50), '}\n'),
  );

  assert.deepEqual(checkFileSize({ rootDir: root }), []);
});
