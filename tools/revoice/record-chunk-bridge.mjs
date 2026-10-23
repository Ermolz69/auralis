import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { constants } from 'node:fs';
import { copyFile, mkdir, readFile, readdir, stat, writeFile } from 'node:fs/promises';
import path from 'node:path';

const root = process.cwd();
const receiptPath = path.join(root, 'tools/revoice/evidence/TTS_CHUNK_BRIDGE_V1.json');
const retained = path.join(root, '.cache/revoice-checks/chunk-bridge-v1');
const sha = (bytes) => createHash('sha256').update(bytes).digest('hex');
const checks = [
  [
    'chunk-bridge-model-free-v1.log',
    'failed',
    'Initial 600-item extraction reached its unchanged 20-second deadline.',
  ],
  [
    'chunk-bridge-model-free-v2.log',
    'passed',
    'Removed per-intermediate-file physical sync and expanded controls.',
  ],
  [
    'chunk-bridge-model-free-v3.log',
    'passed',
    'Post-cleanup sources before strengthening source-window controls.',
  ],
  [
    'chunk-bridge-model-free-v4.log',
    'passed',
    'Final source-window coverage and exact silence controls.',
  ],
  [
    'chunk-bridge-regressions-v1.log',
    'passed',
    'Shared WAV/timeline/domain, TTS, storage and application regressions.',
  ],
  ['chunk-bridge-clippy-v2.log', 'passed', 'Scoped production and test Clippy, warnings denied.'],
  [
    'chunk-bridge-desktop-v1.log',
    'passed',
    'Normal desktop composition check; no application launch.',
  ],
  [
    'chunk-bridge-file-size-v1.log',
    'passed',
    'Eight checker controls and production file-size scan.',
  ],
  ['chunk-bridge-docs-v3.log', 'passed', 'Final documentation and links.'],
  [
    'chunk-bridge-plan-v4.log',
    'passed',
    'Final full-goal plan remains active with 24 mandatory cases.',
  ],
];

const sources = [];
async function recordSource(relative) {
  const absolute = path.join(root, relative);
  if ((await stat(absolute)).isDirectory()) {
    for (const name of (await readdir(absolute)).sort()) await recordSource(`${relative}/${name}`);
    return;
  }
  const bytes = await readFile(absolute);
  assert.ok(bytes.length <= 1024 * 1024, `Source exceeds receipt byte limit: ${relative}`);
  sources.push({ file: relative, bytes: bytes.length, sha256: sha(bytes) });
}
for (const relative of [
  'Taskfile.yml',
  'taskfiles/rust.yml',
  'crates/ports/src/lib.rs',
  'crates/ports/src/speech_audio',
  'crates/adapters-tts/src/lib.rs',
  'crates/adapters-tts/src/result_verification.rs',
  'crates/adapters-tts/src/audio_layout.rs',
  'crates/adapters-tts/tests/result_verification.rs',
  'crates/adapters-tts/tests/result_verification',
  'crates/adapters-ffmpeg/src/assembly',
  'docs/integration/revoice-chunk-bridge.md',
  'docs/integration/revoice-plan.json',
  'tools/revoice/record-chunk-bridge.mjs',
])
  await recordSource(relative);

const logs = [];
for (const [name, outcome, note] of checks) {
  assert.ok(
    (await stat(path.join(root, 'target/revoice', name))).size <= 2 * 1024 * 1024,
    `Log exceeds receipt byte limit: ${name}`,
  );
  const bytes = await readFile(path.join(root, 'target/revoice', name));
  const text = bytes.toString('utf8');
  if (outcome === 'failed') assert.match(text, /test result: FAILED/);
  else {
    assert.doesNotMatch(text, /test result: FAILED|error\[E\d+\]|Failed to run task/);
    assert.match(
      text,
      /test result: ok|Finished `dev`|All file sizes|Documentation contract is valid|Revoicing plan valid/,
    );
  }
  logs.push({ file: name, outcome, note, bytes: bytes.length, sha256: sha(bytes) });
}
await mkdir(path.dirname(retained), { recursive: true });
await mkdir(retained, { recursive: false });
for (const log of logs) {
  const target = path.join(retained, log.file);
  await copyFile(path.join(root, 'target/revoice', log.file), target, constants.COPYFILE_EXCL);
  assert.equal(sha(await readFile(target)), log.sha256);
  log.retained_path = path.relative(root, target).replaceAll('\\', '/');
}
const receipt = {
  version: 1,
  case_id: 'TTS-CHUNK-BRIDGE-MODEL-FREE-v1',
  at: new Date().toISOString(),
  base_commit: '99ead94db0dff6b1c23b69a2f98d8c88055036e6',
  raw_policy: 'pcm16_stream_fade10ms_peak089125_kaiser5_v1',
  assembly_policy: 'pcm16_48k_exact_concat_v1',
  status: 'model_free_checks_passed',
  layout_tests: 11,
  assembly_tests: 19,
  models_loaded: false,
  real_ffmpeg_invoked: false,
  installed_runtime_changed: false,
  voice_references_or_qualification_changed: false,
  limitations:
    'Synthetic PCM and controlled fixture children only. Application orchestration, durable assembly jobs/outbox, installed worker integration, native workflow, natural stories, listening, video and clean Windows remain pending.',
  sources,
  logs,
};
const data = `${JSON.stringify(receipt, null, 2)}\n`;
await writeFile(receiptPath, data, { flag: 'wx' });
await writeFile(path.join(retained, 'receipt.json'), data, { flag: 'wx' });
process.stdout.write(
  `${receipt.case_id}: ${sources.length} source hashes and ${logs.length} retained logs\n`,
);
