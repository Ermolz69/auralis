import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const CORRUPTION_FILE = 'speech-cache-corruption.json';
const ACCEPTANCE_FILE = 'speech-cache-acceptance.json';
const OLD_WORDS = 'каждое слово должно';
const NEW_WORDS = 'каждый текст должен';

export async function verifySpeechCacheRepeat(dataRoot) {
  const runs = await readReadyRuns(dataRoot);
  if (runs.length !== 11) throw new Error(`Expected 11 ready speech runs, found ${runs.length}`);
  const repeat = runs[10];
  const baseline = matchingBaseline(runs.slice(0, 10), repeat);
  assertSameCacheKeys(baseline, repeat);
  assertOrigins(
    repeat,
    repeat.segments.map(() => 'cached'),
  );
  if (baseline.outputSha256 !== repeat.outputSha256)
    throw new Error('Repeated native speech changed the final WAV bytes');
  return { baseline, repeat };
}

export async function corruptSpeechCache(dataRoot) {
  const { repeat } = await verifySpeechCacheRepeat(dataRoot);
  const receiptPath = path.join(dataRoot, CORRUPTION_FILE);
  if (fs.existsSync(receiptPath)) {
    const receipt = JSON.parse(await fsp.readFile(receiptPath, 'utf8'));
    if (
      receipt.run_id !== repeat.runId ||
      digest(await fsp.readFile(receipt.audio_path)) !== receipt.corrupted_sha256
    )
      throw new Error('Retained native cache corruption is not at its expected boundary');
    return receipt;
  }
  const segment = repeat.segments[0];
  if (
    !/^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/.test(repeat.plan.project_id)
  )
    throw new Error('Native cache project identity is not a UUID');
  const projectsRoot = await fsp.realpath(path.join(dataRoot, 'projects'));
  const cacheRoot = await fsp.realpath(
    path.join(dataRoot, 'projects', repeat.plan.project_id, 'speech', 'cache'),
  );
  const audioPath = await fsp.realpath(segment.audio);
  const fileInfo = await fsp.lstat(segment.audio);
  if (
    !cacheRoot.startsWith(`${projectsRoot}${path.sep}`) ||
    !audioPath.startsWith(`${cacheRoot}${path.sep}`) ||
    fileInfo.isSymbolicLink() ||
    fileInfo.nlink !== 1
  )
    throw new Error('Native cache corruption target is outside the project cache');
  const originalSha256 = digest(await fsp.readFile(audioPath));
  if (originalSha256 !== segment.audio_sha256)
    throw new Error('Native cache was already damaged before the controlled corruption');
  const originalSizeBytes = (await fsp.stat(audioPath)).size;
  const damage = Buffer.from('auralis-native-e2e-corrupted-cache\n');
  const handle = await fsp.open(audioPath, 'w');
  try {
    await handle.writeFile(damage);
    await handle.sync();
  } finally {
    await handle.close();
  }
  const receipt = {
    version: 1,
    run_id: repeat.runId,
    project_id: repeat.plan.project_id,
    segment_index: 0,
    cache_key: segment.cache_key,
    audio_path: audioPath,
    original_sha256: originalSha256,
    corrupted_sha256: digest(await fsp.readFile(audioPath)),
    original_size_bytes: originalSizeBytes,
    corrupted_size_bytes: damage.length,
  };
  await fsp.writeFile(receiptPath, JSON.stringify(receipt, null, 2));
  return receipt;
}

export async function verifySpeechCacheSequence(dataRoot) {
  const runs = await readReadyRuns(dataRoot);
  if (runs.length !== 13) throw new Error(`Expected 13 ready speech runs, found ${runs.length}`);
  const baseline = matchingBaseline(runs.slice(0, 10), runs[10]);
  const [repeat, repaired, changed] = runs.slice(10);
  const receipt = JSON.parse(await fsp.readFile(path.join(dataRoot, CORRUPTION_FILE), 'utf8'));
  assertSameCacheKeys(baseline, repeat);
  assertSameCacheKeys(repeat, repaired);
  assertOrigins(
    repeat,
    repeat.segments.map(() => 'cached'),
  );
  if (receipt.run_id !== repeat.runId || receipt.cache_key !== repeat.segments[0].cache_key)
    throw new Error('Cache damage does not identify the repeated native segment');
  assertOrigins(
    repaired,
    repaired.segments.map((_, index) => (index === receipt.segment_index ? 'generated' : 'cached')),
  );
  if (digest(await fsp.readFile(receipt.audio_path)) !== receipt.original_sha256)
    throw new Error('Corrupted native cache audio was not regenerated to the original bytes');
  assertSameIdentity(repeat, changed);
  if (
    changed.plan.utterances[0].text !== repeat.plan.utterances[0].text.replace(OLD_WORDS, NEW_WORDS)
  )
    throw new Error('Native changed-text case did not preserve the intended source edit');
  if (changed.segments.length !== repeat.segments.length)
    throw new Error('Changed speech text unexpectedly changed technical chunk count');
  const changedKeys = changed.segments
    .map((segment, index) => (segment.cache_key !== repeat.segments[index].cache_key ? index : -1))
    .filter((index) => index >= 0);
  if (changedKeys.length !== 1)
    throw new Error(`Expected exactly one changed native chunk, found ${changedKeys.length}`);
  assertOrigins(
    changed,
    changed.segments.map((_, index) => (index === changedKeys[0] ? 'generated' : 'cached')),
  );
  if (new Set([baseline, repeat, repaired].map((run) => run.outputSha256)).size !== 1)
    throw new Error('Repeat or cache repair changed baseline WAV bytes');
  if (changed.outputSha256 === repeat.outputSha256)
    throw new Error('Changed native speech text retained the old WAV bytes');
  const evidence = {
    version: 1,
    node_version: process.version,
    verifier_sha256: digest(await fsp.readFile(new URL(import.meta.url))),
    runtime_config_sha256: digest(await fsp.readFile(path.join(dataRoot, 'tts-runtime.json'))),
    changed_segment_index: changedKeys[0],
    corruption: receipt,
    runs: [baseline, repeat, repaired, changed].map((run) => ({
      run_id: run.runId,
      attempt_id: run.attemptId,
      plan_sha256: run.planSha256,
      text_sha256: digest(Buffer.from(run.plan.utterances[0].text)),
      profile_sha256: run.plan.profile.profile_sha256,
      manifest_sha256: run.manifestSha256,
      output_sha256: run.outputSha256,
      duration_ms: run.durationMs,
      cache_keys: run.segments.map((segment) => segment.cache_key),
      segment_audio_sha256: run.segments.map((segment) => segment.audio_sha256),
      checkpoint_origins: run.checkpoints.map((checkpoint) => checkpoint.origin),
    })),
    status: 'verified_native_cache_repeat_corruption_and_changed_text',
  };
  await fsp.writeFile(path.join(dataRoot, ACCEPTANCE_FILE), JSON.stringify(evidence, null, 2));
  return evidence;
}

function matchingBaseline(runs, repeat) {
  const matches = runs.filter(
    (run) =>
      run.plan.utterances.length === 1 &&
      run.plan.utterances[0].text === repeat.plan.utterances[0].text &&
      run.plan.profile.profile_sha256 === repeat.plan.profile.profile_sha256 &&
      JSON.stringify(run.plan.cast) === JSON.stringify(repeat.plan.cast),
  );
  if (matches.length !== 1)
    throw new Error(`Expected one native cache baseline, found ${matches.length}`);
  return matches[0];
}

function assertSameIdentity(left, right) {
  if (
    left.plan.project_id !== right.plan.project_id ||
    left.plan.profile.profile_sha256 !== right.plan.profile.profile_sha256 ||
    JSON.stringify(left.plan.cast) !== JSON.stringify(right.plan.cast) ||
    left.plan.utterances.length !== 1 ||
    right.plan.utterances.length !== 1 ||
    left.plan.utterances[0].role_id !== right.plan.utterances[0].role_id ||
    left.plan.utterances[0].seed_key !== right.plan.utterances[0].seed_key
  )
    throw new Error(
      'Native speech cache comparison changed the project, profile, cast, role, or seed',
    );
}

function assertSameCacheKeys(left, right) {
  assertSameIdentity(left, right);
  if (
    left.plan.utterances[0].text !== right.plan.utterances[0].text ||
    left.segments.length < 2 ||
    left.segments.length !== right.segments.length ||
    left.segments.some((segment, index) => segment.cache_key !== right.segments[index].cache_key)
  )
    throw new Error('Identical native speech input did not retain every segment cache key');
}

function assertOrigins(run, expected) {
  if (
    run.checkpoints.length !== expected.length ||
    run.checkpoints.some(
      (checkpoint, index) =>
        checkpoint.origin !== expected[index] ||
        checkpoint.cache_key !== run.segments[index].cache_key ||
        checkpoint.audio_sha256 !== run.segments[index].audio_sha256,
    )
  )
    throw new Error(
      `Native speech cache checkpoints do not match expected origins for ${run.runId}`,
    );
}

async function readReadyRuns(dataRoot) {
  const database = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  try {
    const rows = database
      .prepare(
        `SELECT i.run_id, i.plan_json, i.plan_sha256, p.attempt_id, p.publication_json
         FROM speech_run_intents i JOIN speech_publications p ON p.run_id=i.run_id
         WHERE p.ready=1 ORDER BY i.rowid`,
      )
      .all();
    const runs = [];
    for (const row of rows) {
      const plan = JSON.parse(row.plan_json);
      const publication = JSON.parse(row.publication_json);
      const manifestFile = publication.files.find((file) => file.kind === 'manifest');
      const wavFile = publication.files.find((file) => file.kind === 'wav');
      if (!manifestFile || !wavFile)
        throw new Error('Native speech publication lacks WAV or manifest');
      const artifact = database
        .prepare("SELECT location_value FROM artifacts WHERE id=? AND state='ready'")
        .get(manifestFile.artifact_id);
      if (!artifact) throw new Error('Native speech manifest is not a ready managed artifact');
      const manifestBytes = await fsp.readFile(
        path.join(dataRoot, 'projects', artifact.location_value),
      );
      if (digest(manifestBytes) !== manifestFile.sha256)
        throw new Error('Native speech manifest bytes changed');
      const manifest = JSON.parse(manifestBytes.toString('utf8'));
      const checkpoints = database
        .prepare(
          'SELECT origin, cache_key, audio_sha256 FROM speech_host_checkpoints WHERE attempt_id=? ORDER BY sequence',
        )
        .all(row.attempt_id);
      if (
        digest(Buffer.from(row.plan_json)) !== row.plan_sha256 ||
        checkpoints.length !== manifest.segments.length
      )
        throw new Error('Native speech saved plan or segment coverage changed');
      runs.push({
        runId: row.run_id,
        attemptId: row.attempt_id,
        plan,
        planSha256: row.plan_sha256,
        manifestSha256: manifestFile.sha256,
        outputSha256: wavFile.sha256,
        durationMs: publication.duration_ms,
        segments: manifest.segments,
        checkpoints,
      });
    }
    return runs;
  } finally {
    database.close();
  }
}

function digest(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}
