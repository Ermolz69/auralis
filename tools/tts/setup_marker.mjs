import fs from 'node:fs/promises';
import path from 'node:path';
import crypto from 'node:crypto';

const root = path.resolve(process.argv[2] ?? '../MARKER/Normalizer');
const dataRoot = path.resolve(
  process.argv[3] ?? path.join(process.env.APPDATA, 'com.auralis.desktop'),
);
const releaseBytes = await fs.readFile(path.join(root, 'release.json'));
const release = JSON.parse(releaseBytes);
if (release.version !== '1.0.0') throw new Error('Expected frozen Normalizer v1.0.0');
const files = release.files ?? release.fileHashes;
if (!files || typeof files !== 'object') throw new Error('Normalizer release inventory is missing');
for (const [relative, expected] of Object.entries(files)) {
  const file = path.resolve(root, relative);
  if (!file.startsWith(root + path.sep))
    throw new Error('Normalizer inventory escapes its package');
  const actual = crypto
    .createHash('sha256')
    .update(await fs.readFile(file))
    .digest('hex');
  if (actual !== expected)
    throw new Error(`Normalizer file differs from its frozen version: ${relative}`);
}
const config = {
  version: 1,
  node_executable: process.execPath,
  package_root: root,
  release_sha256: crypto.createHash('sha256').update(releaseBytes).digest('hex'),
};
await fs.mkdir(dataRoot, { recursive: true });
const target = path.join(dataRoot, 'puza-marker-runtime.json');
const temporary = target + '.' + crypto.randomUUID() + '.tmp';
await fs.writeFile(temporary, JSON.stringify(config, null, 2) + '\n', { flag: 'wx' });
await fs.rename(temporary, target);
process.stdout.write('Frozen Normalizer v1 configured. No text job or model was started.\n');
