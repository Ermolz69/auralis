import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const root = fs.realpathSync(path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..'));
const hash = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
const specPath = path.resolve(root, process.argv[2]);
const specBytes = fs.readFileSync(specPath);
const spec = JSON.parse(specBytes);
if (
  !/^[A-Za-z0-9_-]{1,64}$/.test(spec.source_group) ||
  !/^[a-f0-9]{64}$/.test(spec.source_sha256) ||
  !['srt', 'vtt'].includes(spec.format)
)
  throw new Error('Invalid frozen material identity');
const output = path.resolve(root, process.argv[3] ?? '');
if (!output.startsWith(path.join(root, 'target/revoice') + path.sep) || fs.existsSync(output))
  throw new Error('Use a new receipt under target/revoice');
const source = path.resolve(root, spec.source_path);
const metadata = fs.lstatSync(source);
if (
  !metadata.isFile() ||
  metadata.isSymbolicLink() ||
  metadata.size > 8 * 1024 * 1024 ||
  metadata.size !== spec.source_bytes
)
  throw new Error('Source is not the bounded admitted regular file');
const bytes = fs.readFileSync(source);
if (hash(bytes) !== spec.source_sha256) throw new Error('Admitted source hash changed');
let destinationRoot = root;
for (const component of ['.cache', 'revoice-materials', spec.source_group]) {
  destinationRoot = path.join(destinationRoot, component);
  if (!fs.existsSync(destinationRoot)) fs.mkdirSync(destinationRoot);
  if (
    !fs.lstatSync(destinationRoot).isDirectory() ||
    fs.lstatSync(destinationRoot).isSymbolicLink()
  )
    throw new Error('Material directory must not be a link');
  if (
    !fs
      .realpathSync(destinationRoot)
      .toLowerCase()
      .startsWith((root + path.sep).toLowerCase())
  )
    throw new Error('Material directory escaped the workspace');
}
const copy = path.join(destinationRoot, `${spec.source_sha256}.${spec.format}`);
if (!fs.existsSync(copy)) fs.writeFileSync(copy, bytes, { flag: 'wx' });
if (hash(fs.readFileSync(copy)) !== spec.source_sha256)
  throw new Error('Retained copy hash differs; existing data was not replaced');
const receipt = {
  version: 1,
  at: new Date().toISOString(),
  spec: specPath,
  spec_sha256: hash(specBytes),
  source_group: spec.source_group,
  source,
  source_sha256: spec.source_sha256,
  copy,
  copy_sha256: hash(fs.readFileSync(copy)),
  bytes: bytes.length,
  limitation:
    'Private immutable material copy outside Cargo target; not native import, parser, audio, inference or quality acceptance. Originals unchanged.',
};
fs.writeFileSync(output, JSON.stringify(receipt, null, 2) + '\n', { flag: 'wx' });
process.stdout.write(JSON.stringify(receipt) + '\n');
