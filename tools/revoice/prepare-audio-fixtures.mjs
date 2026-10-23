import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const sha = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
const git = (args) => execFileSync('git', ['-C', root, ...args], { encoding: 'utf8' }).trim();
const production = [
  'crates',
  'Cargo.lock',
  'Cargo.toml',
  'Taskfile.yml',
  'taskfiles/rust.yml',
  'tools/revoice/prepare-audio-fixtures.mjs',
];
if (process.platform !== 'win32') throw new Error('The frozen cases target Windows x64');
if (git(['status', '--porcelain', '--', ...production]))
  throw new Error(
    'Commit the tested implementation and generator before freezing real-media cases',
  );
const output = path.join(root, 'tools/revoice/ffmpeg-assembly-v1.json');
const relativeRoot = "target/revoice/FFMPEG-ASSEMBLY-INPUTS-v1/音频's";
const fixtureRoot = path.join(root, relativeRoot);
if (fs.existsSync(output) || fs.existsSync(fixtureRoot))
  throw new Error('Frozen fixture case already exists');
const executable = 'src-tauri/binaries/ffmpeg-x86_64-pc-windows-msvc.exe';
const executableHash = '4044b3924c977ad31229d504c5d5b8685f9553124fbaff6e9c99048b42830341';
const digest = crypto.createHash('sha256');
for await (const chunk of fs.createReadStream(path.join(root, executable))) digest.update(chunk);
if (digest.digest('hex') !== executableHash) throw new Error('Bundled FFmpeg identity changed');

fs.mkdirSync(fixtureRoot, { recursive: true });
const fixtures = [];
const payloads = [];
for (const channels of [1, 2]) {
  for (const [label, frames] of [
    ['a', 4800],
    ['b', 2400],
  ]) {
    const payload = Buffer.alloc(frames * channels * 2);
    for (let frame = 0; frame < frames; frame += 1) {
      for (let channel = 0; channel < channels; channel += 1) {
        const sample =
          (label === 'a' ? ((frame + channel * 7) % 64) - 32 : 32 - ((frame + channel * 7) % 64)) *
          (label === 'a' ? 256 : 128);
        payload.writeInt16LE(sample, (frame * channels + channel) * 2);
      }
    }
    const header = Buffer.alloc(44);
    header.write('RIFF', 0);
    header.writeUInt32LE(payload.length + 36, 4);
    header.write('WAVEfmt ', 8);
    header.writeUInt32LE(16, 16);
    header.writeUInt16LE(1, 20);
    header.writeUInt16LE(channels, 22);
    header.writeUInt32LE(48000, 24);
    header.writeUInt32LE(48000 * channels * 2, 28);
    header.writeUInt16LE(channels * 2, 32);
    header.writeUInt16LE(16, 34);
    header.write('data', 36);
    header.writeUInt32LE(payload.length, 40);
    const bytes = Buffer.concat([header, payload]);
    const file = `${channels === 1 ? 'mono' : 'stereo'}-${label}.wav`;
    fs.writeFileSync(path.join(fixtureRoot, file), bytes, { flag: 'wx' });
    fixtures.push({ file, sha256: sha(bytes), sample_rate: 48000, channels, frames });
    payloads.push(payload);
  }
}
function freeze(case_id, mode, indices, pauses, windows, duration, expectedFrames) {
  const hash = crypto.createHash('sha256');
  let cursor = 0;
  const channels = fixtures[indices[0]].channels;
  for (let index = 0; index < indices.length; index += 1) {
    const fixture = fixtures[indices[index]];
    const start = mode === 'subtitle_window' ? windows[index].start_ms * 48 : cursor;
    hash.update(Buffer.alloc((start - cursor) * channels * 2));
    hash.update(payloads[indices[index]]);
    cursor = start + fixture.frames;
    if (pauses[index]) {
      hash.update(Buffer.alloc(pauses[index] * 48 * channels * 2));
      cursor += pauses[index] * 48;
    }
  }
  if (duration !== null) {
    hash.update(Buffer.alloc((duration * 48 - cursor) * channels * 2));
    cursor = duration * 48;
  }
  if (cursor !== expectedFrames) throw new Error('Fixture frame oracle differs from frozen count');
  return {
    case_id,
    mode,
    fixture_indices: indices,
    pauses_ms: pauses,
    windows,
    source_duration_ms: duration,
    expected_frames: expectedFrames,
    expected_pcm_sha256: hash.digest('hex'),
    expected_peak: 8192,
    expected_full_scale: 0,
  };
}
const spec = {
  version: 1,
  source_commit: git(['rev-parse', 'HEAD']),
  generator_sha256: sha(fs.readFileSync(fileURLToPath(import.meta.url))),
  node_version: process.version,
  ffmpeg: { file: executable, sha256: executableHash, version: '8.1.2' },
  fixture_root: relativeRoot,
  fixtures,
  budgets: {
    max_invocations_per_case: 1,
    process_seconds: 60,
    total_seconds: 180,
    cleanup_seconds: 30,
    max_generated_bytes: 64 * 1024 * 1024,
  },
  cases: [
    freeze('FFMPEG-AUDIO-CONTINUOUS-v1', 'continuous', [2, 3], [25, 0], [null, null], null, 8400),
    freeze(
      'FFMPEG-AUDIO-WINDOWS-v1',
      'subtitle_window',
      [0, 1],
      [0, 0],
      [
        { start_ms: 100, end_ms: 200 },
        { start_ms: 300, end_ms: 350 },
      ],
      400,
      19200,
    ),
    freeze(
      'FFMPEG-AUDIO-LONG600-v1',
      'continuous',
      Array.from({ length: 600 }, (_, i) => i % 2),
      Array(600).fill(0),
      Array(600).fill(null),
      null,
      2160000,
    ),
  ],
  scope:
    'Synthetic PCM fixtures only; no models, profiles, voices, native UI, speech quality, playback or clean Windows acceptance.',
};
fs.writeFileSync(output, JSON.stringify(spec, null, 2) + '\n', { flag: 'wx' });
process.stdout.write(
  JSON.stringify({
    spec: path.relative(root, output),
    source_commit: spec.source_commit,
    spec_sha256: sha(fs.readFileSync(output)),
    cases: spec.cases.map((item) => item.case_id),
  }) + '\n',
);
