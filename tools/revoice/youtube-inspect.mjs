import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const hash = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
const specFile = path.resolve(root, process.argv[2] ?? 'tools/revoice/youtube-source-v1.json');
const specBytes = fs.readFileSync(specFile);
const spec = JSON.parse(specBytes);
const parent = path.join(root, 'target/revoice', spec.case_id);
fs.mkdirSync(parent, { recursive: true });
if (fs.readdirSync(parent).some((n) => n.startsWith('attempt-')))
  throw new Error(
    'The frozen one-invocation budget is exhausted; retain the result and declare a new case before another attempt',
  );
const attempt = fs.mkdtempSync(path.join(parent, 'attempt-'));
const manifestBytes = fs.readFileSync(path.join(root, 'tools/media-tools/manifest.json'));
const manifest = JSON.parse(manifestBytes);
const pin = manifest.targets['x86_64-pc-windows-msvc']['yt-dlp'];
const executable = path.join(root, 'src-tauri/binaries', pin.output);
const binaryHash = hash(fs.readFileSync(executable));
if (binaryHash !== pin.sha256) throw new Error('Bundled yt-dlp differs from the pinned binary');
const args = [
  '--ignore-config',
  '--no-cache-dir',
  '--skip-download',
  '--no-playlist',
  '--retries',
  '0',
  '--extractor-retries',
  '0',
  '--socket-timeout',
  String(spec.socket_timeout_seconds),
  '--dump-single-json',
  spec.url,
];
const started = new Date().toISOString();
const clock = performance.now();
const result = spawnSync(executable, args, {
  cwd: attempt,
  encoding: 'utf8',
  timeout: spec.max_wall_seconds * 1000,
  maxBuffer: 32 * 1024 * 1024,
  windowsHide: true,
});
fs.writeFileSync(path.join(attempt, 'stdout.json'), result.stdout ?? '', { flag: 'wx' });
fs.writeFileSync(path.join(attempt, 'stderr.txt'), result.stderr ?? '', { flag: 'wx' });
let metadata;
let validationError;
try {
  if (result.error || result.status !== 0)
    throw new Error(result.error?.message ?? `yt-dlp exit ${result.status}`);
  metadata = JSON.parse(result.stdout);
  if (metadata.id !== spec.video_id) throw new Error('Returned video differs from frozen source');
} catch (error) {
  validationError = error.message;
}
const tracks = (collection) =>
  Object.entries(collection ?? {}).map(([language, formats]) => ({
    language,
    formats: formats.map(({ ext, name }) => ({ ext, name: name ?? null })),
  }));
const manual = tracks(metadata?.subtitles);
const automatic = tracks(metadata?.automatic_captions);
const chinese = (list) => list.filter((t) => /^(zh|cmn)(-|$)/.test(t.language));
const record = {
  version: 1,
  case_id: spec.case_id,
  started_at: started,
  ended_at: new Date().toISOString(),
  elapsed_ms: Math.round(performance.now() - clock),
  spec_sha256: hash(specBytes),
  script_sha256: hash(fs.readFileSync(fileURLToPath(import.meta.url))),
  media_manifest_sha256: hash(manifestBytes),
  binary_sha256: binaryHash,
  args,
  exit_code: result.status,
  signal: result.signal,
  error: validationError ?? null,
  status: validationError
    ? 'failed'
    : chinese(manual).length || chinese(automatic).length
      ? 'chinese_track_listed'
      : 'no_chinese_track',
  video: metadata
    ? {
        id: metadata.id,
        title: metadata.title,
        channel: metadata.channel,
        duration_seconds: metadata.duration,
        language: metadata.language ?? null,
        license: metadata.license ?? null,
        webpage_url: metadata.webpage_url,
      }
    : null,
  manual_chinese_tracks: chinese(manual),
  automatic_chinese_tracks: chinese(automatic),
  stdout_sha256: hash(result.stdout ?? ''),
  stderr_sha256: hash(result.stderr ?? ''),
  limitation:
    'CLI metadata screening only. No media/subtitle bytes, source-language listening, rights admission or native Auralis import has passed.',
};
const recordPath = path.join(attempt, 'receipt.json');
fs.writeFileSync(recordPath, JSON.stringify(record, null, 2) + '\n', { flag: 'wx' });
process.stdout.write(`${JSON.stringify({ receipt: recordPath, ...record })}\n`);
if (record.status !== 'chinese_track_listed') process.exitCode = 1;
