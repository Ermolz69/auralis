import crypto from 'node:crypto';
import fs from 'node:fs/promises';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const SOURCE_MARKER = Buffer.from('__TAURI_BUNDLE_TYPE_VAR_UNK');

export async function prepareBundledApplication(executable) {
  const bytes = await fs.readFile(executable);
  const markerOffset = bytes.indexOf(SOURCE_MARKER);
  if (markerOffset < 0 || bytes.indexOf(SOURCE_MARKER, markerOffset + 1) >= 0) {
    throw new Error(
      'Expected exactly one unpatched Tauri application marker before bundling; rebuild the executable',
    );
  }
  const record = {
    schema_version: 1,
    application_file: path.basename(executable),
    source_sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
    source_bytes: bytes.length,
    marker_offset: markerOffset,
  };
  const directory = path.join(path.dirname(executable), 'bundle-application-digests');
  await fs.mkdir(directory, { recursive: true });
  await fs.writeFile(path.join(directory, 'source.json'), `${JSON.stringify(record, null, 2)}\n`);
  return record;
}

if (path.resolve(process.argv[1] ?? '') === fileURLToPath(import.meta.url)) {
  if (process.platform === 'win32') {
    const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
    const profile = process.env.TAURI_ENV_DEBUG === 'true' ? 'debug' : 'release';
    const executable = path.join(root, 'target', profile, 'auralis-app.exe');
    await prepareBundledApplication(executable);
    process.stdout.write(`Captured pre-package application identity for ${profile}.\n`);
  }
}
