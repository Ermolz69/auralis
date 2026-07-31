import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';

import { checkStorageFallbacks } from './check-storage-fallbacks.mjs';

function fixture() {
  return fs.mkdtempSync(path.join(os.tmpdir(), 'auralis-storage-fallbacks-'));
}

function writeRust(root, relativePath, source) {
  const target = path.join(root, relativePath);
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.writeFileSync(target, source);
}

test('detects unwrap_or_default runtime fallback', () => {
  const root = fixture();
  writeRust(root, 'crates/application/src/runtime.rs', 'let rows = repo.load().unwrap_or_default();\n');

  assert.equal(checkStorageFallbacks({ rootDir: root })[0]?.reason, 'unwrap_or_default storage fallback');
});

test('detects ignored cleanup and commit results without source echo', () => {
  const root = fixture();
  writeRust(
    root,
    'crates/application/src/runtime.rs',
    'let _ = tx.commit().await;\nlet _ = cleanup(secret_token).await;\n',
  );

  assert.deepEqual(checkStorageFallbacks({ rootDir: root }), [
    {
      file: 'crates/application/src/runtime.rs',
      line: 1,
      reason: 'ignored transaction result',
    },
    {
      file: 'crates/application/src/runtime.rs',
      line: 2,
      reason: 'ignored cleanup or persistence result',
    },
  ]);
});

test('allows explicit fallback marker and comments', () => {
  const root = fixture();
  writeRust(
    root,
    'crates/application/src/runtime.rs',
    '// let rows = repo.load().unwrap_or_default();\nlet _ = tx.rollback().await; // allow-fallback\n',
  );

  assert.deepEqual(checkStorageFallbacks({ rootDir: root }), []);
});

test('excludes test fixtures from production fallback scan', () => {
  const root = fixture();
  writeRust(root, 'crates/application/src/usecase/tests.rs', 'let rows = repo.load().unwrap_or_default();\n');

  assert.deepEqual(checkStorageFallbacks({ rootDir: root }), []);
});

test('keeps artifacts_json allowlist narrow', () => {
  const root = fixture();
  writeRust(root, 'crates/application/src/runtime.rs', 'let field = "artifacts_json";\n');

  assert.equal(
    checkStorageFallbacks({ rootDir: root })[0]?.reason,
    'legacy artifacts_json outside migration runtime',
  );
});
