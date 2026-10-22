import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const hash = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
const specBytes = fs.readFileSync(path.resolve(root, process.argv[2]));
const spec = JSON.parse(specBytes);
if (process.env.AURALIS_REVOICE_ADMITTED_CASE !== spec.case_id)
  throw new Error('Use the shared-admission Taskfile command');
if (
  spec.track.kind !== 'manual' ||
  spec.track.format !== 'vtt' ||
  spec.track.language !== 'zh' ||
  spec.max_invocations !== 1
)
  throw new Error('Unsupported frozen caption case');
const parent = path.join(root, 'target/revoice', spec.case_id);
fs.mkdirSync(parent, { recursive: true });
if (fs.readdirSync(parent).some((file) => file.startsWith('attempt-')))
  throw new Error('Frozen one-attempt budget exhausted');
const attempt = fs.mkdtempSync(path.join(parent, 'attempt-'));
const clock = performance.now();
const receipt = {
  version: 1,
  case_id: spec.case_id,
  started_at: new Date().toISOString(),
  spec_sha256: hash(specBytes),
  script_sha256: hash(fs.readFileSync(fileURLToPath(import.meta.url))),
  source_url: spec.url,
  selected_track: spec.track,
  status: 'pending',
  limitation: spec.limitation,
};
try {
  const metadataEvidence = JSON.parse(fs.readFileSync(path.resolve(root, spec.metadata_evidence)));
  const metadataRoot = path.join(root, 'target/revoice', spec.metadata_case);
  const attempts = fs.readdirSync(metadataRoot).filter((file) => file.startsWith('attempt-'));
  if (attempts.length !== 1) throw new Error('Metadata attempt identity is ambiguous');
  const raw = fs.readFileSync(path.join(metadataRoot, attempts[0], 'stdout.json'));
  if (hash(raw) !== metadataEvidence.stdout_sha256)
    throw new Error('Retained metadata bytes changed');
  const metadata = JSON.parse(raw);
  if (metadata.id !== spec.video_id || metadata.webpage_url !== spec.url)
    throw new Error('Metadata belongs to a different video');
  const track = metadata.subtitles?.[spec.track.language]?.find(
    (track) => track.ext === spec.track.format,
  );
  if (!track || new URL(track.url).protocol !== 'https:')
    throw new Error('Exact manual Chinese VTT track is unavailable');
  receipt.metadata_sha256 = hash(raw);
  const response = await fetch(track.url, {
    signal: AbortSignal.timeout(spec.max_wall_seconds * 1000),
  });
  receipt.http_status = response.status;
  if (!response.ok) throw new Error(`Caption HTTP ${response.status}`);
  const chunks = [];
  let bytes = 0;
  for await (const chunk of response.body) {
    bytes += chunk.length;
    if (bytes > spec.max_subtitle_bytes) throw new Error('Caption byte ceiling exceeded');
    chunks.push(chunk);
  }
  const content = Buffer.concat(chunks);
  if (
    !content.length ||
    !content
      .subarray(0, 64)
      .toString('utf8')
      .replace(/^\uFEFF/, '')
      .startsWith('WEBVTT')
  )
    throw new Error('Caption response is empty or lacks the WEBVTT header');
  const output = path.join(attempt, 'HRaa8kb1EhE.zh.vtt');
  fs.writeFileSync(output, content, { flag: 'wx' });
  receipt.subtitle = {
    path: output,
    bytes,
    sha256: hash(content),
    timing_lines_screen: [
      ...content.toString('utf8').matchAll(/^(?:\d+:)?\d{2}:\d{2}\.\d{3} --> /gm),
    ].length,
  };
  receipt.status = 'caption_bytes_retained';
} catch (error) {
  receipt.status = 'failed';
  receipt.error = error.message;
}
receipt.ended_at = new Date().toISOString();
receipt.elapsed_ms = Math.round(performance.now() - clock);
fs.writeFileSync(path.join(attempt, 'receipt.json'), JSON.stringify(receipt, null, 2) + '\n', {
  flag: 'wx',
});
process.stdout.write(JSON.stringify({ attempt, ...receipt }) + '\n');
if (receipt.status !== 'caption_bytes_retained') process.exitCode = 1;
