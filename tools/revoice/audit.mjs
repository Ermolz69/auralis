import crypto from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { isDeepStrictEqual } from 'node:util';
import { fileURLToPath } from 'node:url';

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
const workspace = path.dirname(root);
const output = path.resolve(process.argv[2] ?? '');
if (!process.argv[2] || fs.existsSync(output))
  throw new Error('Supply a new audit output path; retained evidence is never replaced');
const started = performance.now();
const sha = (bytes) => crypto.createHash('sha256').update(bytes).digest('hex');
const errors = [];
function fileIdentity(file) {
  try {
    const bytes = fs.readFileSync(file);
    return { path: file, size_bytes: bytes.length, sha256: sha(bytes) };
  } catch (error) {
    if (error.code !== 'ENOENT') errors.push({ path: file, error: error.message });
    return { path: file, status: error.code === 'ENOENT' ? 'absent' : 'unreadable' };
  }
}
function json(file) {
  return JSON.parse(fs.readFileSync(file, 'utf8'));
}
function git(repo, ...args) {
  const result = spawnSync('git', ['-C', repo, ...args], {
    encoding: 'utf8',
    windowsHide: true,
    timeout: 30_000,
    maxBuffer: 8 * 1024 * 1024,
  });
  if (result.error || result.status !== 0) throw new Error(result.error?.message ?? result.stderr);
  return result.stdout;
}
const repositories = [];
for (const name of ['auralis', 'auralis-translate', 'CONTRACT', 'TTS', 'CASTER', 'MARKER']) {
  const repo = path.join(workspace, name);
  const changed = git(repo, 'diff', '--name-only', '-z', 'HEAD').split('\0').filter(Boolean);
  repositories.push({
    name,
    head: git(repo, 'rev-parse', 'HEAD').trim(),
    branch: git(repo, 'rev-parse', '--abbrev-ref', 'HEAD').trim(),
    status: git(repo, 'status', '--porcelain=v1', '--untracked-files=normal').trimEnd(),
    tracked_diff_sha256: sha(git(repo, 'diff', '--binary', 'HEAD')),
    changed_tracked_files: changed.map((file) => fileIdentity(path.join(repo, file))),
  });
}
const pinLine = git(root, 'ls-tree', 'HEAD', 'modules/auralis-translate').trim();
const pinned = pinLine.match(/^160000 commit ([0-9a-f]{40})\s/)[1];
const submoduleHead = git(path.join(root, 'modules/auralis-translate'), 'rev-parse', 'HEAD').trim();
const appData = path.join(process.env.APPDATA, 'com.auralis.desktop');
const configurations = [
  'tts-runtime.json',
  'translation-runtime.json',
  'caster-runtime.json',
  'marker-runtime.json',
].map((name) => fileIdentity(path.join(appData, name)));
const receiptFile = path.join(appData, 'tts/receipt.json');
const receipt = json(receiptFile);
const config = json(path.join(appData, 'tts-runtime.json'));
const wheels = Object.entries(receipt.wheels).map(([name, expected]) => {
  const wheelRoot = path.join(appData, 'tts/wheels');
  const matches = fs
    .readdirSync(wheelRoot, { withFileTypes: true })
    .filter((e) => e.isDirectory())
    .map((e) => path.join(wheelRoot, e.name, name))
    .filter((p) => fs.existsSync(p));
  const files = matches.map(fileIdentity);
  return {
    name,
    expected_sha256: expected,
    files,
    verified: files.some((f) => f.sha256 === expected),
  };
});
const voiceIds = [
  'marina_elevenlabs_ru',
  'alex_analyst_elevenlabs_ru',
  'anastasiya_kim_elevenlabs_ru',
  'artur_denophine_elevenlabs_ru',
  'denis_elevenlabs_ru',
  'nastya_elevenlabs_ru',
  'rina_elevenlabs_ru',
  'stanislav_elevenlabs_ru',
];
const probeRoot = path.join(workspace, 'TTS/PUZA_TTS/.tmp_audio/new-voices-20261021');
const registryFile = path.join(probeRoot, 'cases.json');
const cases = json(registryFile).cases;
const combinations = [];
const probes = [];
for (const item of cases) {
  const wave = fileIdentity(path.join(probeRoot, item.case_id, 'sample.wav'));
  const outcomeFile = path.join(probeRoot, item.case_id, 'outcome.json');
  const outcome = json(outcomeFile);
  probes.push({
    case_id: item.case_id,
    voice_id: item.voice_id,
    reference_id: item.reference_id,
    engine_id: item.engine_id,
    manifest_sha256: item.manifest_sha256,
    reference_audio_sha256: item.reference_audio_sha256,
    wave,
    outcome: fileIdentity(outcomeFile),
    verified:
      outcome.status === 'completed' &&
      isDeepStrictEqual(outcome.input, item) &&
      outcome.output_sha256 === wave.sha256 &&
      outcome.effective_seed === item.seed,
  });
}
const voices = [];
for (const id of voiceIds) {
  const directory = path.join(config.voice_store, id);
  const manifestFile = path.join(directory, 'voice.json');
  const manifest = json(manifestFile);
  const identity = fileIdentity(manifestFile);
  const sourceFile = path.join(workspace, 'CASTER/voices', id, 'voice.json');
  voices.push({
    id,
    manifest: identity,
    source_manifest: fileIdentity(sourceFile),
    references: manifest.references.length,
  });
  for (const reference of manifest.references) {
    const audioPath = path.resolve(directory, reference.audio.path);
    if (!audioPath.startsWith(directory + path.sep))
      throw new Error('Reference audio escapes its voice directory');
    const audio = fileIdentity(audioPath);
    const matching = probes.filter(
      (p) =>
        p.voice_id === id &&
        p.reference_id === reference.reference_id &&
        p.engine_id === reference.engine_id &&
        p.manifest_sha256 === identity.sha256 &&
        p.reference_audio_sha256 === reference.audio.sha256 &&
        p.verified,
    );
    combinations.push({
      id: `${id}/${reference.emotion_id}/${reference.engine_id}`,
      voice_id: id,
      emotion_id: reference.emotion_id,
      engine_id: reference.engine_id,
      reference_id: reference.reference_id,
      manifest_sha256: identity.sha256,
      audio,
      expected_audio_sha256: reference.audio.sha256,
      audio_verified: audio.sha256 === reference.audio.sha256,
      qualification_status: reference.qualification?.status ?? 'missing',
      qualification_report: reference.qualification?.report ?? null,
      retained_probe_ids: matching.map((p) => p.case_id),
      evaluation_status:
        reference.qualification?.status === 'unverified'
          ? matching.length
            ? 'awaiting_listening'
            : 'probe_required'
          : 'requires_report_validation',
    });
  }
}
const report = {
  version: 1,
  kind: 'model_free_read_only_baseline',
  created_at: new Date().toISOString(),
  command: `task revoice:audit OUTPUT=${process.argv[2]}`,
  node: process.version,
  audit_script: fileIdentity(fileURLToPath(import.meta.url)),
  repositories,
  translate: {
    pinned,
    checked_out: submoduleHead,
    matches_pin: pinned === submoduleHead,
    sibling_head: repositories.find((r) => r.name === 'auralis-translate').head,
    submodule_status: git(
      path.join(root, 'modules/auralis-translate'),
      'status',
      '--porcelain=v1',
    ).trim(),
  },
  installed: {
    data_root: appData,
    configurations,
    tts_receipt: fileIdentity(receiptFile),
    component_pins: {
      tts: receipt.tts_commit,
      contract: receipt.contract_commit,
      caster: receipt.caster_commit,
    },
    python_executable: fileIdentity(config.python_executable),
    wheels,
    profiles: fs
      .readdirSync(config.profile_root)
      .filter((n) => n.endsWith('.yaml') || n.endsWith('.json') || n.endsWith('.yml'))
      .map((n) => fileIdentity(path.join(config.profile_root, n))),
    installer_acceptance: 'not_established',
    clean_windows: 'unavailable_owner_confirmed_2026-10-23',
  },
  voices,
  combinations,
  probes,
  registry: fileIdentity(registryFile),
  listening_page: fileIdentity(path.join(probeRoot, 'INDEX.html')),
  summary: {
    combinations: combinations.length,
    qualified: combinations.filter((c) => c.qualification_status === 'qualified').length,
    unverified: combinations.filter((c) => c.qualification_status === 'unverified').length,
    rejected: combinations.filter((c) => c.qualification_status === 'rejected').length,
    verified_probe_combinations: combinations.filter((c) => c.retained_probe_ids.length).length,
    retained_probes: probes.length,
    verified_probes: probes.filter((p) => p.verified).length,
  },
  errors,
  elapsed_ms: Math.round(performance.now() - started),
  limitations: [
    'No model loading, inference or worker execution',
    'Qualification report semantics and listening are not inferred',
    'No clean Windows or installed native workflow acceptance',
    'Repository dirty hashes do not identify a committed release candidate',
  ],
};
fs.mkdirSync(path.dirname(output), { recursive: true });
fs.writeFileSync(output, JSON.stringify(report, null, 2) + '\n', { flag: 'wx' });
process.stdout.write(
  `${JSON.stringify({ output, sha256: fileIdentity(output).sha256, ...report.summary, errors: errors.length })}\n`,
);
if (
  errors.length ||
  combinations.length !== 192 ||
  combinations.some((c) => !c.audio_verified) ||
  probes.some((p) => !p.verified) ||
  wheels.some((w) => !w.verified)
)
  process.exitCode = 2;
