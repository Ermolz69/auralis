import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const canonical = (file) => fs.realpathSync.native(file).replace(/^\\\\\?\\(?=[A-Za-z]:\\)/, '');
const owned = (file) => {
  const resolved = canonical(file);
  const relative = path.relative(canonical(root), resolved);
  if (relative.startsWith('..') || path.isAbsolute(relative))
    throw new Error('Audio evidence is outside the workspace');
  return resolved;
};
const hash = async (file) => {
  const digest = crypto.createHash('sha256');
  for await (const chunk of fs.createReadStream(file)) digest.update(chunk);
  return digest.digest('hex');
};
const specPath = owned(path.resolve(root, process.argv[2] ?? ''));
const spec = JSON.parse(fs.readFileSync(specPath, 'utf8'));
const specHash = await hash(specPath);
if (spec.version !== 1 || spec.cases.length !== 3)
  throw new Error('Unsupported frozen audio matrix');
const parent = path.join(root, '.cache/revoice-audio');
fs.mkdirSync(parent, { recursive: true });
owned(parent);
const destination = path.join(parent, specHash);
if (fs.existsSync(destination)) throw new Error('Retained audio case already exists');
fs.mkdirSync(destination);
owned(destination);
let total = 0;
const files = [];
async function retain(source, relative, expected) {
  const original = owned(source);
  const bytes = fs.statSync(original).size;
  total += bytes;
  if (total > 64 * 1024 * 1024) throw new Error('Audio retention byte budget exceeded');
  if ((await hash(original)) !== expected) throw new Error('Original audio evidence hash differs');
  const target = path.join(destination, relative);
  if (path.relative(destination, target).startsWith('..')) throw new Error('Invalid retained path');
  fs.mkdirSync(path.dirname(target), { recursive: true });
  fs.copyFileSync(original, target, fs.constants.COPYFILE_EXCL);
  if ((await hash(target)) !== expected || (await hash(original)) !== expected)
    throw new Error('Audio evidence changed while retaining');
  files.push({ source: original, retained: target, bytes, sha256: expected });
}
await retain(specPath, 'spec.json', specHash);
for (const fixture of spec.fixtures) {
  await retain(
    path.join(root, spec.fixture_root, fixture.file),
    path.join('fixtures', fixture.file),
    fixture.sha256,
  );
}
for (const item of spec.cases) {
  if (!/^[A-Za-z0-9_-]{1,64}$/.test(item.case_id)) throw new Error('Invalid audio case ID');
  const receiptPath = path.join(root, 'target/revoice', item.case_id, 'receipt.json');
  const receipt = JSON.parse(fs.readFileSync(receiptPath, 'utf8'));
  if (
    receipt.status !== 'completed' ||
    receipt.spec_sha256 !== specHash ||
    receipt.independent_expected_pcm_matched !== true ||
    receipt.admission_release_confirmed !== true ||
    receipt.expected_pcm_sha256 !== item.expected_pcm_sha256 ||
    receipt.frames !== item.expected_frames
  )
    throw new Error('Audio case is not verified and complete');
  await retain(receipt.wav, path.join(item.case_id, 'speech.wav'), receipt.wav_sha256);
  await retain(
    receipt.assembly_manifest,
    path.join(item.case_id, 'assembly.json'),
    receipt.assembly_manifest_sha256,
  );
  await retain(receiptPath, path.join(item.case_id, 'receipt.json'), await hash(receiptPath));
}
const log = path.join(root, 'target/revoice/ffmpeg-assembly-real-v1.log');
await retain(log, 'real-v1.log', await hash(log));
const record = {
  version: 1,
  at: new Date().toISOString(),
  spec_sha256: specHash,
  script_sha256: await hash(fileURLToPath(import.meta.url)),
  total_bytes: total,
  files,
  scope:
    'Exact retained synthetic fixture inputs, verified staged WAVs and manifests. No native, speech/listening, publication or clean Windows acceptance.',
};
fs.writeFileSync(path.join(destination, 'retention.json'), JSON.stringify(record, null, 2) + '\n', {
  flag: 'wx',
});
process.stdout.write(
  JSON.stringify({
    retained: destination,
    files: files.length,
    bytes: total,
    spec_sha256: specHash,
  }) + '\n',
);
