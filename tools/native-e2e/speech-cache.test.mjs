import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { DatabaseSync } from 'node:sqlite';
import {
  corruptSpeechCache,
  verifySpeechCacheRepeat,
  verifySpeechCacheSequence,
} from './speech-cache.mjs';

const PROJECT = '00000000-0000-4000-8000-000000000001';
const TEXT = 'Проверенный текст: каждое слово должно звучать ясно. Вторая часть не меняется.';

async function fixture(outsideCache = false) {
  const temporary = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-speech-cache-'));
  const dataRoot = path.join(temporary, 'app-data');
  const cacheRoot = path.join(dataRoot, 'projects', PROJECT, 'speech', 'cache', 'voxcpm2');
  await fsp.mkdir(cacheRoot, { recursive: true });
  await fsp.writeFile(path.join(dataRoot, 'tts-runtime.json'), '{"version":1}');
  const original = Buffer.from('RIFF-synthetic-cache-audio');
  const audioPath = outsideCache
    ? path.join(dataRoot, 'outside.wav')
    : path.join(cacheRoot, `voxcpm2_${'a'.repeat(64)}.wav`);
  await fsp.writeFile(audioPath, original);
  const database = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'));
  database.exec(`
    CREATE TABLE speech_run_intents(run_id TEXT, plan_json TEXT, plan_sha256 TEXT);
    CREATE TABLE speech_publications(run_id TEXT, attempt_id TEXT, publication_json TEXT, ready INTEGER);
    CREATE TABLE artifacts(id TEXT, state TEXT, location_value TEXT);
    CREATE TABLE speech_host_checkpoints(attempt_id TEXT, sequence INTEGER, origin TEXT, cache_key TEXT, audio_sha256 TEXT);
  `);
  for (let index = 0; index < 11; index++) {
    const runId = `run-${index}`;
    const attemptId = `attempt-${index}`;
    const text = index === 8 || index === 10 ? TEXT : `Other speech ${index}`;
    const plan = {
      project_id: PROJECT,
      profile: { profile_sha256: 'f'.repeat(64) },
      cast: [{ role_id: 'narrator', voice_id: 'new-voice' }],
      utterances: [{ text, role_id: 'narrator', seed_key: 'stable-cue-seed' }],
    };
    const planJson = JSON.stringify(plan);
    const keys = index === 8 || index === 10 ? ['a'.repeat(64), 'b'.repeat(64)] : ['c'.repeat(64)];
    const segments = keys.map((cacheKey, segmentIndex) => ({
      cache_key: cacheKey,
      audio: segmentIndex === 0 ? audioPath : path.join(cacheRoot, 'second.wav'),
      audio_sha256: digest(original),
    }));
    const manifest = Buffer.from(JSON.stringify({ segments }));
    const relativeManifest = path.join(PROJECT, `manifest-${index}.json`);
    const manifestPath = path.join(dataRoot, 'projects', relativeManifest);
    await fsp.mkdir(path.dirname(manifestPath), { recursive: true });
    await fsp.writeFile(manifestPath, manifest);
    const manifestId = `manifest-${index}`;
    const publication = {
      duration_ms: 1000,
      files: [
        { kind: 'manifest', artifact_id: manifestId, sha256: digest(manifest) },
        { kind: 'wav', artifact_id: `wav-${index}`, sha256: 'd'.repeat(64) },
      ],
    };
    database
      .prepare('INSERT INTO speech_run_intents VALUES (?, ?, ?)')
      .run(runId, planJson, digest(Buffer.from(planJson)));
    database
      .prepare('INSERT INTO speech_publications VALUES (?, ?, ?, 1)')
      .run(runId, attemptId, JSON.stringify(publication));
    database
      .prepare('INSERT INTO artifacts VALUES (?, ?, ?)')
      .run(manifestId, 'ready', relativeManifest);
    for (let segmentIndex = 0; segmentIndex < segments.length; segmentIndex++) {
      database
        .prepare('INSERT INTO speech_host_checkpoints VALUES (?, ?, ?, ?, ?)')
        .run(
          attemptId,
          segmentIndex + 1,
          index === 10 ? 'cached' : 'generated',
          segments[segmentIndex].cache_key,
          segments[segmentIndex].audio_sha256,
        );
    }
  }
  database.close();
  return { temporary, dataRoot, audioPath, original };
}

async function removeFixture(temporary) {
  const resolved = await fsp.realpath(temporary);
  const tempRoot = await fsp.realpath(os.tmpdir());
  if (path.dirname(resolved) !== tempRoot)
    throw new Error('Refusing to remove a speech cache fixture outside TEMP');
  await fsp.rm(resolved, { recursive: true, force: true });
}

test('native cache verifier follows saved plans, publications, checkpoints and controlled audio damage', async () => {
  const { temporary, dataRoot, audioPath, original } = await fixture();
  try {
    const { baseline, repeat } = await verifySpeechCacheRepeat(dataRoot);
    assert.equal(baseline.runId, 'run-8');
    assert.equal(repeat.runId, 'run-10');
    const receipt = await corruptSpeechCache(dataRoot);
    assert.equal(receipt.original_sha256, digest(original));
    assert.equal(receipt.original_size_bytes, original.length);
    assert.equal(receipt.corrupted_sha256, digest(await fsp.readFile(audioPath)));
    assert.notEqual(receipt.corrupted_sha256, receipt.original_sha256);
  } finally {
    await removeFixture(temporary);
  }
});

test('native cache corruption refuses a manifest audio path outside the project cache', async () => {
  const { temporary, dataRoot, audioPath, original } = await fixture(true);
  try {
    await assert.rejects(corruptSpeechCache(dataRoot), /outside the project cache/);
    assert.deepEqual(await fsp.readFile(audioPath), original);
  } finally {
    await removeFixture(temporary);
  }
});

test('native cache evidence distinguishes one repaired segment and one changed-text segment', async () => {
  const { temporary, dataRoot, audioPath, original } = await fixture();
  try {
    await corruptSpeechCache(dataRoot);
    await fsp.writeFile(audioPath, original);
    await addOutcomeRun(
      dataRoot,
      audioPath,
      original,
      11,
      TEXT,
      ['a', 'b'],
      ['generated', 'cached'],
      'd',
    );
    await addOutcomeRun(
      dataRoot,
      audioPath,
      original,
      12,
      TEXT.replace('каждое слово должно', 'каждый текст должен'),
      ['e', 'b'],
      ['generated', 'cached'],
      'e',
    );
    const evidence = await verifySpeechCacheSequence(dataRoot);
    assert.equal(evidence.status, 'verified_native_cache_repeat_corruption_and_changed_text');
    assert.equal(evidence.changed_segment_index, 0);
    assert.deepEqual(evidence.runs[2].checkpoint_origins, ['generated', 'cached']);
    assert.deepEqual(evidence.runs[3].checkpoint_origins, ['generated', 'cached']);
    assert.equal(
      JSON.parse(await fsp.readFile(path.join(dataRoot, 'speech-cache-acceptance.json'), 'utf8'))
        .runs.length,
      4,
    );
    const database = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'));
    try {
      database
        .prepare('UPDATE speech_host_checkpoints SET origin=? WHERE attempt_id=? AND sequence=1')
        .run('cached', 'attempt-12');
    } finally {
      database.close();
    }
    await assert.rejects(verifySpeechCacheSequence(dataRoot), /expected origins/);
  } finally {
    await removeFixture(temporary);
  }
});

async function addOutcomeRun(
  dataRoot,
  audioPath,
  original,
  index,
  text,
  keyLetters,
  origins,
  outputLetter,
) {
  const database = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'));
  try {
    const runId = `run-${index}`;
    const attemptId = `attempt-${index}`;
    const planJson = JSON.stringify({
      project_id: PROJECT,
      profile: { profile_sha256: 'f'.repeat(64) },
      cast: [{ role_id: 'narrator', voice_id: 'new-voice' }],
      utterances: [{ text, role_id: 'narrator', seed_key: 'stable-cue-seed' }],
    });
    const segments = keyLetters.map((letter, segmentIndex) => ({
      cache_key: letter.repeat(64),
      audio: segmentIndex === 0 ? audioPath : path.join(path.dirname(audioPath), 'second.wav'),
      audio_sha256: digest(original),
    }));
    const manifest = Buffer.from(JSON.stringify({ segments }));
    const relativeManifest = path.join(PROJECT, `manifest-${index}.json`);
    await fsp.writeFile(path.join(dataRoot, 'projects', relativeManifest), manifest);
    const manifestId = `manifest-${index}`;
    const publication = {
      duration_ms: 1000,
      files: [
        { kind: 'manifest', artifact_id: manifestId, sha256: digest(manifest) },
        { kind: 'wav', artifact_id: `wav-${index}`, sha256: outputLetter.repeat(64) },
      ],
    };
    database
      .prepare('INSERT INTO speech_run_intents VALUES (?, ?, ?)')
      .run(runId, planJson, digest(Buffer.from(planJson)));
    database
      .prepare('INSERT INTO speech_publications VALUES (?, ?, ?, 1)')
      .run(runId, attemptId, JSON.stringify(publication));
    database
      .prepare('INSERT INTO artifacts VALUES (?, ?, ?)')
      .run(manifestId, 'ready', relativeManifest);
    for (let segmentIndex = 0; segmentIndex < segments.length; segmentIndex++) {
      database
        .prepare('INSERT INTO speech_host_checkpoints VALUES (?, ?, ?, ?, ?)')
        .run(
          attemptId,
          segmentIndex + 1,
          origins[segmentIndex],
          segments[segmentIndex].cache_key,
          segments[segmentIndex].audio_sha256,
        );
    }
  } finally {
    database.close();
  }
}

function digest(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}
