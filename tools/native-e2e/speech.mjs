import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { spawn } from 'node:child_process';

export async function seedSpeechRuntime(dataRoot, subtitlePath) {
  const original = path.join(process.env.APPDATA, 'com.auralis.desktop', 'tts-runtime.json');
  const config = JSON.parse(await fsp.readFile(original, 'utf8'));
  for (const voice of ['marina_elevenlabs_ru', 'alex_analyst_elevenlabs_ru']) {
    const manifest = JSON.parse(
      await fsp.readFile(path.join(config.voice_store, voice, 'voice.json'), 'utf8'),
    );
    if (!['voxcpm2', 'espeech'].every((engine) => manifest.references.some((reference) =>
      reference.engine_id === engine && reference.emotion_id === 'neutral' && reference.qualification.status === 'qualified')))
      throw new Error(`Human qualification missing: ${voice}`);
  }
  await fsp.copyFile(original, path.join(dataRoot, 'tts-runtime.json'));
  const longCue =
    'Мы читаем русские субтитры спокойно и последовательно. Каждая реплика сохраняет своё время, голос и порядок. Проверенный текст остаётся неизменным, даже когда длинная реплика разделяется на несколько небольших частей. Эти части звучат подряд и сохраняются вместе с общим результатом. После остановки можно продолжить работу с сохранённого места.';
  await fsp.writeFile(
    subtitlePath,
    `1\n00:00:01,000 --> 00:00:04,000\nДобрый вечер.\n\n2\n00:00:05,000 --> 00:00:09,000\nМы проверяем русскую речь.\n\n3\n00:00:10,000 --> 00:00:35,000\n${longCue}\n`,
    'utf8',
  );
}

export async function waitForSpeechTitle(dataRoot, app, title, runId) {
  const databasePath = path.join(dataRoot, 'auralis.sqlite');
  const deadline = Date.now() + 1_800_000;
  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null)
      throw new Error(`Native Speech app exited early: ${app.child.exitCode}\n${app.stderr}`);
    if (fs.existsSync(databasePath)) {
      let database;
      try {
        database = new DatabaseSync(databasePath, { readOnly: true });
        const failed = database
          .prepare('SELECT id FROM projects WHERE title=?')
          .get(`native-e2e-speech-failed:${runId}`);
        if (failed)
          throw new Error(
            `Native Speech React scenario failed\n${await fsp.readFile(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8').catch(() => '')}\n${app.stderr}`,
          );
        const project = database.prepare('SELECT id FROM projects WHERE title=?').get(title);
        if (project) return project.id;
      } catch (error) {
        if (error.message.startsWith('Native Speech')) throw error;
      } finally {
        database?.close();
      }
    }
    await delay(250);
  }
  throw new Error(`Timed out waiting for Speech project ${title}\n${app.stderr}`);
}

export async function verifySpeechArtifacts(dataRoot, expectedCount) {
  const db = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  try {
    const publications = db
      .prepare('SELECT * FROM speech_publications WHERE ready=1 ORDER BY run_id')
      .all();
    if (publications.length !== expectedCount)
      throw new Error(`Expected ${expectedCount} ready speech runs, found ${publications.length}`);
    const hashes = {};
    let splitPlans = 0;
    for (const row of publications) {
      const published = JSON.parse(row.publication_json);
      const intent = db.prepare('SELECT * FROM speech_run_intents WHERE run_id=?').get(row.run_id);
      if (digest(Buffer.from(intent.plan_json)) !== intent.plan_sha256)
        throw new Error('Saved plan bytes changed');
      const plan = JSON.parse(intent.plan_json);
      if (plan.utterances.length !== intent.expected_utterances)
        throw new Error('Saved cue coverage changed');
      if (intent.expected_chunks > intent.expected_utterances) splitPlans++;
      if (plan.mode === 'subtitle_utterances') {
        if (
          plan.utterances.length !== 3 ||
          plan.utterances[0].start_ms !== 1000 ||
          plan.utterances[2].end_ms !== 35000
        )
          throw new Error('Native subtitle windows or order changed');
      }
      if (published.files.filter((f) => f.kind === 'segment').length !== intent.expected_chunks)
        throw new Error('Published segment coverage is incomplete');
      const job = db
        .prepare(
          'SELECT j.* FROM jobs j JOIN speech_host_attempts a ON a.job_id=j.id WHERE a.attempt_id=?',
        )
        .get(row.attempt_id);
      if (job.status !== 'completed' || JSON.parse(job.progress_json).percent !== 100)
        throw new Error('Ready speech did not complete its host job');
      let wav;
      let manifest;
      for (const file of published.files) {
        const artifact = db.prepare('SELECT * FROM artifacts WHERE id=?').get(file.artifact_id);
        if (artifact.state !== 'ready' || artifact.location_kind !== 'StorageKey')
          throw new Error('Speech result is outside ready managed storage');
        const bytes = await fsp.readFile(path.join(dataRoot, 'projects', artifact.location_value));
        if (digest(bytes) !== file.sha256 || bytes.length !== file.size_bytes)
          throw new Error('Managed speech file hash or length changed');
        hashes[file.artifact_id] = file.sha256;
        if (file.kind === 'wav') wav = bytes;
        if (file.kind === 'manifest') manifest = JSON.parse(bytes.toString('utf8'));
      }
      if (
        !wav ||
        !manifest ||
        wav.toString('ascii', 0, 4) !== 'RIFF' ||
        wav.toString('ascii', 8, 12) !== 'WAVE'
      )
        throw new Error('Final audio is not WAV');
      const fmt = wav.indexOf(Buffer.from('fmt '));
      const data = wav.indexOf(Buffer.from('data'));
      const rate = wav.readUInt32LE(fmt + 12);
      const bytesPerSecond = wav.readUInt32LE(fmt + 16);
      const durationMs = Math.round((wav.readUInt32LE(data + 4) / bytesPerSecond) * 1000);
      if (rate !== 48000 || Math.abs(durationMs - published.duration_ms) > 1 || durationMs <= 0)
        throw new Error('Independent WAV duration differs');
    }
    if (expectedCount === 10) {
      if (splitPlans < 6) throw new Error('Technical splitting did not survive native publication');
      const attempts = db
        .prepare(
          'SELECT run_id, COUNT(*) AS count FROM speech_host_attempts GROUP BY run_id HAVING COUNT(*)>1',
        )
        .all();
      if (attempts.length !== 2 || attempts.some((a) => a.count !== 2))
        throw new Error('Cancel/crash resume did not create two distinct attempts');
      for (const run of attempts) {
        const cached = db
          .prepare(
            "SELECT COUNT(*) AS count FROM speech_host_checkpoints c JOIN speech_host_attempts a ON a.attempt_id=c.attempt_id WHERE a.run_id=? AND c.origin='cached'",
          )
          .get(run.run_id);
        if (!cached.count) throw new Error('Resume did not reuse verified saved audio');
      }
      const active = db
        .prepare('SELECT COUNT(*) AS count FROM speech_host_attempts WHERE ended_at IS NULL')
        .get();
      if (active.count) throw new Error('Finished native run retained an active attempt');
    }
    const proof = {
      version: 1,
      ready_runs: expectedCount,
      split_plans: splitPlans,
      artifact_hashes: hashes,
    };
    await fsp.writeFile(
      path.join(dataRoot, 'speech-acceptance.json'),
      JSON.stringify(proof, null, 2),
    );
    return JSON.stringify(hashes);
  } finally {
    db.close();
  }
}

export async function waitForSpeechRelease(rootDir) {
  const deadline = Date.now() + 60_000;
  let reason = '';
  while (Date.now() < deadline) {
    const inspected = await inspectSpeechRelease(rootDir, deadline - Date.now());
    if (inspected.code === 0) return;
    reason = inspected.output;
    await delay(500);
  }
  throw new Error(`Model resources were not released: ${reason}`);
}

async function inspectSpeechRelease(rootDir, timeoutMs) {
  const child = spawn(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      path.join(rootDir, 'crates/common/src/model_admission.ps1'),
    ],
    { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true },
  );
  let output = '';
  let released = false;
  child.stdout.on('data', (bytes) => {
    output += bytes.toString();
    if (!released && output.includes('READY')) {
      released = true;
      child.stdin.end('release\n');
    }
  });
  const code = await new Promise((resolve, reject) => {
    const timer = setTimeout(() => {
      if (!released) child.kill();
      else {
        child.unref();
        child.stdout.destroy();
        child.stderr.destroy();
      }
      reject(new Error('GPU/resource release did not complete'));
    }, timeoutMs);
    child.on('error', reject);
    child.on('exit', (code) => {
      clearTimeout(timer);
      resolve(code);
    });
  });
  return { code, output };
}
function digest(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}
function delay(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
