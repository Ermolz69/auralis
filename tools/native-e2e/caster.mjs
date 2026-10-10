import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const POLL_MS = 100;
const TIMEOUT_MS = 90_000;

export async function seedCasterRuntime(dataRoot) {
  const python = process.env.AURALIS_TEST_CASTER_PYTHON;
  const voiceStore = process.env.AURALIS_TEST_CASTER_VOICE_STORE;
  if (!python || !voiceStore || !path.isAbsolute(python) || !path.isAbsolute(voiceStore)) {
    throw new Error(
      'Native Caster E2E requires absolute installed Python and isolated voice-store paths',
    );
  }
  if (!fs.statSync(python).isFile() || !fs.statSync(voiceStore).isDirectory()) {
    throw new Error('Native Caster E2E runtime or fixture store is unavailable');
  }
  const manifest = path.join(path.dirname(path.dirname(python)), 'caster-wheelhouse-manifest.json');
  const wheelManifestSha256 = digest(await fsp.readFile(manifest));
  await fsp.writeFile(
    path.join(dataRoot, 'caster-runtime.json'),
    JSON.stringify({
      version: 1,
      python_executable: python,
      voice_store: voiceStore,
      caster_version: '0.2.0',
      contract_version: '0.2.0',
      wheel_manifest_sha256: wheelManifestSha256,
    }),
  );
}

export async function waitForCasterTitle(dataRoot, app, title, runId) {
  const databasePath = path.join(dataRoot, 'auralis.sqlite');
  const deadline = Date.now() + TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null) {
      throw new Error(`Native Caster application exited early: ${app.child.exitCode}`);
    }
    if (fs.existsSync(databasePath)) {
      let database;
      try {
        database = new DatabaseSync(databasePath, { readOnly: true });
        const failed = database
          .prepare('SELECT id FROM projects WHERE title = ?')
          .get(`native-e2e-caster-failed:${runId}`);
        if (failed) throw new Error('Native Caster React scenario failed');
        const project = database.prepare('SELECT id FROM projects WHERE title = ?').get(title);
        if (project) return project.id;
      } catch (error) {
        if (error.message === 'Native Caster React scenario failed') throw error;
      } finally {
        database?.close();
      }
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_MS));
  }
  throw new Error(`Timed out waiting for native Caster project ${title}`);
}

export async function verifyCasterArtifact(dataRoot, projectId) {
  const database = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  let artifacts;
  try {
    artifacts = database
      .prepare(
        "SELECT location_kind, location_value, state FROM artifacts WHERE project_id = ? AND kind = 'ManualCast'",
      )
      .all(projectId);
  } finally {
    database.close();
  }
  if (
    artifacts.length !== 1 ||
    artifacts[0].state !== 'ready' ||
    artifacts[0].location_kind !== 'StorageKey'
  ) {
    throw new Error('Native cast artifact is not ready in managed SQLite storage');
  }
  const revisionPath = path.join(dataRoot, 'projects', artifacts[0].location_value);
  const revisionBytes = await fsp.readFile(revisionPath);
  const revisionSha256 = digest(revisionBytes);
  if (path.basename(revisionPath) !== `${revisionSha256}.json`)
    throw new Error('Managed cast digest differs from its storage key');
  const pointer = JSON.parse(
    await fsp.readFile(
      path.join(dataRoot, 'projects', projectId, 'casting', 'current.json'),
      'utf8',
    ),
  );
  if (pointer.revision_sha256 !== revisionSha256)
    throw new Error('Native cast pointer differs from the managed artifact');
  const audioCache = path.join(dataRoot, 'cache', 'caster-audio');
  const audioFiles = (await fsp.readdir(audioCache)).filter((file) => file.endsWith('.wav'));
  if (audioFiles.length < 1) throw new Error('Native reference audio was not prepared');
  for (const file of audioFiles) {
    const bytes = await fsp.readFile(path.join(audioCache, file));
    if (file !== `${digest(bytes)}.wav`) throw new Error('Prepared reference audio digest differs');
  }
  return revisionSha256;
}

function digest(value) {
  return crypto.createHash('sha256').update(value).digest('hex');
}
