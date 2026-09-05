import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { findDuplicateCode } from './check-duplicate-code.mjs';

function fixture() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-duplicate-code-'));
}

function write(root, relativePath, lines) {
  const target = path.join(root, relativePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, lines.join('\n'));
}

const repeatedBlock = Array.from(
  { length: 12 },
  (_, index) => `const repeated${index} = value${index};`,
);

test('detects large exact production duplicates', () => {
  const root = fixture();
  write(root, 'apps/desktop/src/features/one/a.ts', repeatedBlock);
  write(root, 'apps/desktop/src/features/two/b.ts', repeatedBlock);

  assert.deepEqual(findDuplicateCode({ rootDir: root }), [
    {
      first: { file: 'apps/desktop/src/features/one/a.ts', line: 1 },
      second: { file: 'apps/desktop/src/features/two/b.ts', line: 1 },
      lines: 12,
    },
  ]);
});

test('ignores short similarities and non-production files', () => {
  const root = fixture();
  write(root, 'apps/desktop/src/features/one/a.ts', repeatedBlock.slice(0, 11));
  write(root, 'apps/desktop/src/features/two/b.ts', repeatedBlock.slice(0, 11));
  write(root, 'apps/desktop/src/features/one/a.test.ts', repeatedBlock);
  write(root, 'apps/desktop/src/features/two/b.stories.tsx', repeatedBlock);

  assert.deepEqual(findDuplicateCode({ rootDir: root }), []);
});
