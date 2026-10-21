import assert from 'node:assert/strict';
import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import test from 'node:test';
import { DatabaseSync } from 'node:sqlite';
import { verifySpeechArtifacts } from './speech.mjs';

test('native publication verifier checks saved WAV duration and records exact run evidence', async () => {
  const temporary = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-speech-artifacts-'));
  try {
    const dataRoot = path.join(temporary, 'app-data');
    const projectId = '00000000-0000-4000-8000-000000000001';
    const managed = path.join(dataRoot, 'projects', projectId, 'ready');
    await fsp.mkdir(managed, { recursive: true });
    await fsp.writeFile(path.join(dataRoot, 'tts-runtime.json'), '{"version":1}');
    const wav = wavOneSecond();
    const manifest = Buffer.from('{"segments":[{"cache_key":"synthetic"}]}');
    const segment = Buffer.from('synthetic-segment');
    const files = [];
    for (const [kind, bytes] of [
      ['wav', wav],
      ['manifest', manifest],
      ['segment', segment],
    ]) {
      const relative = path.join(projectId, 'ready', `${kind}.bin`);
      await fsp.writeFile(path.join(dataRoot, 'projects', relative), bytes);
      files.push({
        kind,
        artifact_id: `${kind}-artifact`,
        sha256: digest(bytes),
        size_bytes: bytes.length,
        relative,
      });
    }
    const planJson = JSON.stringify({
      project_id: projectId,
      mode: 'continuous',
      profile: { model_revision: 'model-revision', profile_sha256: 'f'.repeat(64) },
      utterances: [{ text: 'Проверенный текст.', seed_key: 'stable-seed' }],
    });
    const db = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'));
    try {
      db.exec(`
        CREATE TABLE speech_publications(run_id TEXT, attempt_id TEXT, publication_json TEXT, ready INTEGER);
        CREATE TABLE speech_run_intents(run_id TEXT, project_id TEXT, plan_json TEXT, plan_sha256 TEXT, expected_utterances INTEGER, expected_chunks INTEGER);
        CREATE TABLE jobs(id TEXT, status TEXT, progress_json TEXT);
        CREATE TABLE speech_host_attempts(attempt_id TEXT, run_id TEXT, job_id TEXT, created_at TEXT, ended_at TEXT);
        CREATE TABLE speech_host_checkpoints(attempt_id TEXT, sequence INTEGER, utterance_id TEXT, cue_id TEXT, chunk_id TEXT, cache_key TEXT, audio_sha256 TEXT, duration_ms INTEGER, origin TEXT, saved_at TEXT);
        CREATE TABLE artifacts(id TEXT, state TEXT, location_kind TEXT, location_value TEXT);
      `);
      db.prepare('INSERT INTO speech_run_intents VALUES (?, ?, ?, ?, 1, 1)').run(
        'run-1',
        projectId,
        planJson,
        digest(Buffer.from(planJson)),
      );
      db.prepare('INSERT INTO jobs VALUES (?, ?, ?)').run('job-1', 'completed', '{"percent":100}');
      db.prepare('INSERT INTO speech_host_attempts VALUES (?, ?, ?, ?, ?)').run(
        'attempt-1',
        'run-1',
        'job-1',
        '2026-10-21T00:00:00Z',
        '2026-10-21T00:00:02Z',
      );
      db.prepare(
        'INSERT INTO speech_host_checkpoints VALUES (?, 1, ?, NULL, ?, ?, ?, 1000, ?, ?)',
      ).run(
        'attempt-1',
        'utterance-1',
        'chunk-1',
        'a'.repeat(64),
        digest(segment),
        'generated',
        '2026-10-21T00:00:01Z',
      );
      for (const file of files)
        db.prepare('INSERT INTO artifacts VALUES (?, ?, ?, ?)').run(
          file.artifact_id,
          'ready',
          'StorageKey',
          file.relative,
        );
      const publication = {
        duration_ms: 1000,
        files: files.map(({ relative: _relative, ...file }) => file),
      };
      db.prepare('INSERT INTO speech_publications VALUES (?, ?, ?, 1)').run(
        'run-1',
        'attempt-1',
        JSON.stringify(publication),
      );
      await verifySpeechArtifacts(dataRoot, 1);
      const evidence = JSON.parse(
        await fsp.readFile(path.join(dataRoot, 'speech-acceptance.json'), 'utf8'),
      );
      assert.equal(evidence.version, 2);
      assert.equal(evidence.runs[0].plan.utterances[0].text, 'Проверенный текст.');
      assert.equal(evidence.runs[0].independent_wav_duration_ms, 1000);
      assert.equal(evidence.runs[0].attempts[0].elapsed_ms, 2000);
      db.prepare('UPDATE speech_publications SET publication_json=? WHERE run_id=?').run(
        JSON.stringify({ ...publication, duration_ms: 2000 }),
        'run-1',
      );
      await assert.rejects(verifySpeechArtifacts(dataRoot, 1), /Independent WAV duration differs/);
    } finally {
      db.close();
    }
  } finally {
    const resolved = await fsp.realpath(temporary);
    const tempRoot = await fsp.realpath(os.tmpdir());
    if (path.dirname(resolved) !== tempRoot)
      throw new Error('Refusing to remove an artifact fixture outside TEMP');
    await fsp.rm(resolved, { recursive: true, force: true });
  }
});

function wavOneSecond() {
  const channels = 1;
  const sampleRate = 48000;
  const bitsPerSample = 16;
  const dataLength = sampleRate * channels * (bitsPerSample / 8);
  const result = Buffer.alloc(44 + dataLength);
  result.write('RIFF', 0, 'ascii');
  result.writeUInt32LE(result.length - 8, 4);
  result.write('WAVEfmt ', 8, 'ascii');
  result.writeUInt32LE(16, 16);
  result.writeUInt16LE(1, 20);
  result.writeUInt16LE(channels, 22);
  result.writeUInt32LE(sampleRate, 24);
  result.writeUInt32LE(sampleRate * channels * (bitsPerSample / 8), 28);
  result.writeUInt16LE(channels * (bitsPerSample / 8), 32);
  result.writeUInt16LE(bitsPerSample, 34);
  result.write('data', 36, 'ascii');
  result.writeUInt32LE(dataLength, 40);
  return result;
}

function digest(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}
