import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export function expectedReleaseTag(rootDir) {
  const manifest = JSON.parse(fs.readFileSync(path.join(rootDir, 'package.json'), 'utf8'));
  if (typeof manifest.version !== 'string' || manifest.version.length === 0) {
    throw new Error('Root package.json must declare a release version');
  }
  return `app-v${manifest.version}`;
}

export function validateReleaseTag(rootDir, tag) {
  const expected = expectedReleaseTag(rootDir);
  if (tag !== expected) {
    throw new Error(`Release tag must be ${expected}; received ${tag || '<empty>'}`);
  }
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  try {
    validateReleaseTag(rootDir, process.argv[2]);
    process.stdout.write(`Release tag matches ${expectedReleaseTag(rootDir)}.\n`);
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
