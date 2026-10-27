import assert from 'node:assert/strict';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { storage, idPattern, json } from './core.mjs';
import { candidateIdentity } from './session.mjs';

const [id, seed] = process.argv.slice(2);
assert.ok(idPattern.test(id ?? '') && idPattern.test(seed ?? ''));
const candidate = await candidateIdentity();
const runtime_seed = await json(path.join(storage, 'seeds', `${seed}.json`), 32 * 1024 * 1024);
assert.ok(runtime_seed.files.some((file) => file.path === 'project-runtime.json'));
assert.ok(
  runtime_seed.files.every((file) => !/\.(safetensors|gguf|pt|pth|onnx|wav)$/i.test(file.path)),
);
const spec = {
  version: 1,
  mode: 'media-smoke',
  case_id: id,
  max_attempts: 1,
  max_wall_seconds: 1800,
  allow_inference: false,
  candidate_sha256: candidate.receipt.sha256,
  candidate_receipt_sha256: candidate.receipt_sha256,
  runtime_seed,
};
await fsp.mkdir(path.join(storage, 'specs'), { recursive: true });
await fsp.writeFile(path.join(storage, 'specs', `${id}.json`), JSON.stringify(spec, null, 2), {
  flag: 'wx',
});
process.stdout.write(`Frozen model-free media smoke ${id}\n`);
