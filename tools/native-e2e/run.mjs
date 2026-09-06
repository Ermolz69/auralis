import childProcess from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { fileURLToPath } from 'node:url';
import { loadManifest, resolveTarget } from '../media-tools/manifest.mjs';

const RESULT_TIMEOUT_MS = 60_000;
const POLL_INTERVAL_MS = 100;

export async function runNativeE2e(rootDir) {
  assertSupportedPlatform();
  const testRoot = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-native-e2e-'));
  const dataRoot = path.join(testRoot, 'app-data');
  const sourcePath = path.join(testRoot, 'source.mp4');
  const runId = crypto.randomUUID();
  let app;

  try {
    await fsp.mkdir(dataRoot, { recursive: true });
    prepareMediaTools(rootDir);
    generateVideo(rootDir, sourcePath);
    buildTestApplication(rootDir, dataRoot, sourcePath, runId);

    app = launchTestApplication(rootDir, dataRoot);
    const result = await waitForResult(path.join(dataRoot, 'auralis.sqlite'), runId, app);
    await verifyResult({ dataRoot, sourcePath, ...result });
  } catch (error) {
    const logs = app ? formatProcessLogs(app) : '';
    throw new Error(`${error.message}${logs}`, { cause: error });
  } finally {
    if (app) await terminate(app.child);
    if (process.env.AURALIS_NATIVE_E2E_KEEP_TEMP === '1') {
      process.stdout.write(`Native E2E data kept at ${testRoot}\n`);
    } else {
      await fsp.rm(testRoot, { recursive: true, force: true });
    }
  }
}

function assertSupportedPlatform() {
  if (process.platform !== 'win32') {
    throw new Error(`Native Tauri E2E is currently supported on Windows, not ${process.platform}`);
  }
}

function prepareMediaTools(rootDir) {
  if (process.env.AURALIS_NATIVE_E2E_MEDIA_READY === '1') return;
  run('task', ['media:prepare'], rootDir, process.env, 600_000);
}

function generateVideo(rootDir, sourcePath) {
  const manifest = loadManifest(path.join(rootDir, 'tools/media-tools/manifest.json'));
  const target = resolveTarget();
  const ffmpeg = path.join(rootDir, 'src-tauri/binaries', manifest.targets[target].ffmpeg.output);
  if (!fs.existsSync(ffmpeg)) throw new Error(`Prepared ffmpeg is missing: ${ffmpeg}`);

  run(
    ffmpeg,
    [
      '-hide_banner',
      '-loglevel',
      'error',
      '-f',
      'lavfi',
      '-i',
      'color=c=black:s=320x180:d=0.5',
      '-c:v',
      'libx264',
      '-pix_fmt',
      'yuv420p',
      '-an',
      '-y',
      sourcePath,
    ],
    rootDir,
    process.env,
    60_000,
  );
}

function buildTestApplication(rootDir, dataRoot, sourcePath, runId) {
  const env = {
    ...process.env,
    AURALIS_NATIVE_E2E: '1',
    AURALIS_NATIVE_E2E_DATA_DIR: dataRoot,
    AURALIS_NATIVE_E2E_MEDIA_PATH: sourcePath,
    AURALIS_NATIVE_E2E_RUN_ID: runId,
  };
  run(
    process.execPath,
    [
      path.join(rootDir, 'node_modules/@tauri-apps/cli/tauri.js'),
      'build',
      '--no-bundle',
      '--config',
      'src-tauri/tauri.native-e2e.conf.json',
    ],
    rootDir,
    env,
    1_200_000,
  );
}

function launchTestApplication(rootDir, dataRoot) {
  const targetDir = path.resolve(rootDir, process.env.CARGO_TARGET_DIR ?? 'target');
  const executable = path.join(targetDir, 'release', 'auralis-app.exe');
  if (!fs.existsSync(executable))
    throw new Error(`Native E2E executable is missing: ${executable}`);

  const child = childProcess.spawn(executable, [], {
    cwd: rootDir,
    env: {
      ...process.env,
      AURALIS_NATIVE_E2E_DATA_DIR: dataRoot,
      AURALIS_OBSERVABILITY_ENABLED: 'false',
    },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true,
  });
  const output = { child, stdout: '', stderr: '' };
  child.stdout.on('data', (chunk) => (output.stdout = appendLog(output.stdout, chunk)));
  child.stderr.on('data', (chunk) => (output.stderr = appendLog(output.stderr, chunk)));
  return output;
}

async function waitForResult(databasePath, runId, app) {
  const deadline = Date.now() + RESULT_TIMEOUT_MS;
  const completedTitle = `native-e2e-complete:${runId}`;
  const failedTitle = `native-e2e-failed:${runId}`;

  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null) {
      throw new Error(
        `Native application exited early: code ${app.child.exitCode}, signal ${app.child.signalCode}`,
      );
    }
    const result = tryReadResult(databasePath, completedTitle, failedTitle);
    if (result?.failed) throw new Error('React native E2E runner reported a failure');
    if (result) return result;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error(`Timed out after ${RESULT_TIMEOUT_MS} ms waiting for the native E2E result`);
}

function tryReadResult(databasePath, completedTitle, failedTitle) {
  if (!fs.existsSync(databasePath)) return null;
  let database;
  try {
    database = new DatabaseSync(databasePath, { readOnly: true });
    const project = database
      .prepare(
        'SELECT id, title, status, source_json, metadata_json, revision FROM projects WHERE title IN (?, ?)',
      )
      .get(completedTitle, failedTitle);
    if (!project) return null;
    if (project.title === failedTitle) return { failed: true };

    const artifacts = database
      .prepare(
        "SELECT id, project_id, kind, location_kind, location_value, size_bytes, state, ready_at FROM artifacts WHERE project_id = ? AND kind = 'SourceVideo'",
      )
      .all(project.id);
    const outbox = database
      .prepare(
        "SELECT kind, status, attempts, last_error FROM outbox_messages WHERE aggregate_id = ? AND kind = 'finalize_staged_artifact'",
      )
      .all(project.id);
    return { project, artifacts, outbox };
  } catch (error) {
    if (String(error).includes('locked') || String(error).includes('busy')) return null;
    throw error;
  } finally {
    database?.close();
  }
}

async function verifyResult({ dataRoot, sourcePath, project, artifacts, outbox }) {
  assert(project.status === 'ReadyForProcessing', `Unexpected project status: ${project.status}`);
  assert(project.revision >= 3, `Expected project revision >= 3, got ${project.revision}`);

  const source = JSON.parse(project.source_json);
  const metadata = JSON.parse(project.metadata_json);
  assert(source.ManagedLocalFile, 'Project source is not a managed local file');
  assert(
    source.ManagedLocalFile.original_filename === 'source.mp4',
    'Original filename was not saved',
  );
  assert(metadata.has_video === true, 'ffprobe metadata did not identify the video stream');
  assert(metadata.width === 320 && metadata.height === 180, 'Unexpected probed video dimensions');

  assert(artifacts.length === 1, `Expected one source artifact, found ${artifacts.length}`);
  const artifact = artifacts[0];
  assert(artifact.project_id === project.id, 'Artifact belongs to a different project');
  assert(artifact.location_kind === 'StorageKey', 'Artifact is not stored by managed key');
  assert(artifact.state === 'ready' && artifact.ready_at, 'Artifact was not finalized');
  assert(
    source.ManagedLocalFile.artifact_id === artifact.id,
    'Project source and artifact table reference different artifacts',
  );

  assert(outbox.length === 1, `Expected one finalize outbox record, found ${outbox.length}`);
  assert(outbox[0].status === 'done', `Finalize outbox status is ${outbox[0].status}`);
  assert(outbox[0].last_error === null, 'Finalize outbox record contains an error');

  const projectsRoot = path.resolve(dataRoot, 'projects');
  const artifactPath = path.resolve(projectsRoot, ...artifact.location_value.split('/'));
  assert(
    artifactPath.startsWith(`${projectsRoot}${path.sep}`),
    'Artifact storage key escaped the managed project directory',
  );
  const sourceStat = await fsp.stat(sourcePath);
  const artifactStat = await fsp.stat(artifactPath);
  assert(artifactStat.isFile(), 'Final artifact path is not a file');
  assert(
    artifactStat.size === sourceStat.size,
    'Final artifact size differs from the imported file',
  );
  assert(Number(artifact.size_bytes) === sourceStat.size, 'SQLite artifact size is incorrect');
  assert(
    (await sha256(artifactPath)) === (await sha256(sourcePath)),
    'Final artifact bytes differ',
  );

  const stagingFiles = await listFiles(path.join(projectsRoot, '.staging'));
  assert(
    stagingFiles.length === 0,
    `Staging files remain after finalize: ${stagingFiles.join(', ')}`,
  );
}

function run(command, args, cwd, env, timeout) {
  const result = childProcess.spawnSync(command, args, {
    cwd,
    env,
    encoding: 'utf8',
    timeout,
    windowsHide: true,
    stdio: ['ignore', 'inherit', 'inherit'],
  });
  if (result.error || result.status !== 0) {
    throw new Error(`${command} failed: ${result.error?.message ?? `exit code ${result.status}`}`);
  }
}

async function sha256(file) {
  return crypto
    .createHash('sha256')
    .update(await fsp.readFile(file))
    .digest('hex');
}

async function listFiles(root) {
  if (!fs.existsSync(root)) return [];
  const files = [];
  for (const entry of await fsp.readdir(root, { withFileTypes: true })) {
    const candidate = path.join(root, entry.name);
    if (entry.isDirectory()) files.push(...(await listFiles(candidate)));
    else files.push(candidate);
  }
  return files;
}

function appendLog(current, chunk) {
  return `${current}${chunk}`.slice(-16_000);
}

function formatProcessLogs(app) {
  const stdout = app.stdout.trim();
  const stderr = app.stderr.trim();
  return `\nNative app stdout:\n${stdout || '(empty)'}\nNative app stderr:\n${stderr || '(empty)'}`;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function delay(durationMs) {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}

async function terminate(child) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  childProcess.spawnSync('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
    stdio: 'ignore',
    windowsHide: true,
  });
  await Promise.race([
    new Promise((resolve) => child.once('exit', resolve)),
    new Promise((resolve) => setTimeout(resolve, 5_000)),
  ]);
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  runNativeE2e(rootDir)
    .then(() =>
      process.stdout.write(
        'Native Tauri E2E passed: React -> IPC -> Rust -> SQLite -> filesystem.\n',
      ),
    )
    .catch((error) => {
      process.stderr.write(`${error.message}\n`);
      process.exitCode = 1;
    });
}
