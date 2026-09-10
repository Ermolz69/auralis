import childProcess from 'node:child_process';
import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import http from 'node:http';
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
  const fixtureRoot = path.join(testRoot, 'fixture');
  const sourcePath = path.join(fixtureRoot, 'source.mp4');
  const frontendBuildRoot = path.join(rootDir, 'apps', 'desktop', 'dist-native-e2e');
  const runId = 'isolated-run';
  let app;
  let fixtureServer;
  let failure;

  try {
    await fsp.mkdir(dataRoot, { recursive: true });
    await fsp.mkdir(fixtureRoot, { recursive: true });
    prepareMediaTools(rootDir);
    generateVideo(rootDir, sourcePath);
    fixtureServer = await startFixtureServer(sourcePath);
    buildTestApplication(rootDir, dataRoot, sourcePath, fixtureServer.url, runId);

    app = launchTestApplication(rootDir, dataRoot, fixtureServer.url);
    const result = await waitForResult(path.join(dataRoot, 'auralis.sqlite'), runId, app);
    await verifyResult({
      dataRoot,
      sourcePath,
      ytdlpUrl: fixtureServer.url,
      ytdlpRequestCount: fixtureServer.requestCount,
      ...result,
    });
  } catch (error) {
    const logs = app ? formatProcessLogs(app) : '';
    const checkpoint = await readBootstrapCheckpoint(dataRoot);
    failure = new Error(`${error.message}${checkpoint}${logs}`, { cause: error });
  } finally {
    const cleanupErrors = [];
    if (app) {
      try {
        await terminate(app.child);
      } catch (error) {
        cleanupErrors.push(error);
      }
    }
    if (fixtureServer) {
      try {
        await fixtureServer.close();
      } catch (error) {
        cleanupErrors.push(error);
      }
    }
    if (process.env.AURALIS_NATIVE_E2E_KEEP_TEMP === '1') {
      process.stdout.write(`Native E2E data kept at ${testRoot}\n`);
    } else {
      try {
        await fsp.rm(testRoot, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
        assert(!fs.existsSync(testRoot), `Native E2E sandbox was not removed: ${testRoot}`);
      } catch (error) {
        cleanupErrors.push(error);
      }
    }
    try {
      await fsp.rm(frontendBuildRoot, {
        recursive: true,
        force: true,
        maxRetries: 10,
        retryDelay: 200,
      });
      assert(
        !fs.existsSync(frontendBuildRoot),
        `Native E2E frontend build was not removed: ${frontendBuildRoot}`,
      );
    } catch (error) {
      cleanupErrors.push(error);
    }
    if (cleanupErrors.length > 0) {
      const cleanupMessage = cleanupErrors.map((error) => error.message).join('; ');
      failure = new Error(
        `${failure?.message ?? 'Native E2E cleanup failed'}\nCleanup failure: ${cleanupMessage}`,
        { cause: failure ?? cleanupErrors[0] },
      );
    }
  }

  if (failure) throw failure;
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

async function startFixtureServer(sourcePath) {
  let requestCount = 0;
  const server = http.createServer((request, response) => {
    const requestUrl = new URL(request.url ?? '/', 'http://127.0.0.1');
    if (!['GET', 'HEAD'].includes(request.method ?? '') || requestUrl.pathname !== '/source.mp4') {
      response.writeHead(404, { connection: 'close' });
      response.end();
      return;
    }

    requestCount += 1;
    const size = fs.statSync(sourcePath).size;
    const range = /^bytes=(\d+)-(\d*)$/.exec(request.headers.range ?? '');
    let start = 0;
    let end = size - 1;
    let status = 200;
    const headers = {
      'accept-ranges': 'bytes',
      'content-type': 'video/mp4',
    };

    if (range) {
      start = Number(range[1]);
      end = range[2] ? Math.min(Number(range[2]), size - 1) : size - 1;
      if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) || start > end || start >= size) {
        response.writeHead(416, { 'content-range': `bytes */${size}`, connection: 'close' });
        response.end();
        return;
      }
      status = 206;
      headers['content-range'] = `bytes ${start}-${end}/${size}`;
    }

    headers['content-length'] = String(end - start + 1);
    response.writeHead(status, headers);
    if (request.method === 'HEAD') {
      response.end();
      return;
    }
    const stream = fs.createReadStream(sourcePath, { start, end });
    stream.on('error', () => response.destroy());
    stream.pipe(response);
  });

  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(0, '127.0.0.1', resolve);
  });
  const address = server.address();
  if (!address || typeof address === 'string') {
    server.close();
    throw new Error('Native E2E fixture server did not expose a TCP port');
  }

  return {
    url: `http://127.0.0.1:${address.port}/source.mp4`,
    get requestCount() {
      return requestCount;
    },
    async close() {
      server.closeAllConnections?.();
      await new Promise((resolve, reject) => {
        server.close((error) => (error ? reject(error) : resolve()));
      });
    },
  };
}

function buildTestApplication(rootDir, dataRoot, sourcePath, ytdlpUrl, runId) {
  const env = {
    ...process.env,
    AURALIS_NATIVE_E2E: '1',
    AURALIS_NATIVE_E2E_DATA_DIR: dataRoot,
    AURALIS_NATIVE_E2E_MEDIA_PATH: sourcePath,
    AURALIS_NATIVE_E2E_YTDLP_URL: ytdlpUrl,
    AURALIS_NATIVE_E2E_RUN_ID: runId,
  };
  run(
    process.execPath,
    [
      path.join(rootDir, 'node_modules/@tauri-apps/cli/tauri.js'),
      'build',
      '--no-bundle',
      '--debug',
      '--features',
      'native-e2e',
      '--config',
      'src-tauri/tauri.native-e2e.conf.json',
    ],
    rootDir,
    env,
    1_200_000,
  );
}

function launchTestApplication(rootDir, dataRoot, ytdlpUrl) {
  const targetDir = path.resolve(rootDir, process.env.CARGO_TARGET_DIR ?? 'target');
  const executable = path.join(targetDir, 'debug', 'auralis-app.exe');
  if (!fs.existsSync(executable))
    throw new Error(`Native E2E executable is missing: ${executable}`);

  const child = childProcess.spawn(executable, [], {
    cwd: rootDir,
    env: {
      ...process.env,
      AURALIS_NATIVE_E2E_DATA_DIR: dataRoot,
      AURALIS_NATIVE_E2E_YTDLP_URL: ytdlpUrl,
      AURALIS_OBSERVABILITY_ENABLED: 'false',
      // Some CI hosts already run inside an OS sandbox, which makes WebView2's
      // nested sandbox crash before navigation. This applies only to the E2E
      // child; its profile and every reachable fixture live under testRoot.
      WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: '--no-sandbox',
      WEBVIEW2_USER_DATA_FOLDER: path.join(dataRoot, 'webview2'),
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
    const failedProject = database
      .prepare(
        'SELECT id FROM projects WHERE title = ?',
      )
      .get(failedTitle);
    if (failedProject) return { failed: true };

    const project = database
      .prepare(
        'SELECT id, title, status, source_json, metadata_json, revision FROM projects WHERE title = ?',
      )
      .get(completedTitle);
    const youtubeProject = database
      .prepare(
        'SELECT id, title, status, source_json, metadata_json, revision FROM projects WHERE title = ?',
      )
      .get(`native-e2e-ytdlp-complete:${runIdFromTitle(completedTitle)}`);
    if (!project || !youtubeProject) return null;

    const artifacts = database
      .prepare(
        "SELECT id, project_id, kind, location_kind, location_value, size_bytes, state, ready_at FROM artifacts WHERE project_id = ? AND kind = 'SourceVideo'",
      )
      .all(project.id);
    const youtubeArtifacts = database
      .prepare(
        "SELECT id, project_id, kind, location_kind, location_value, size_bytes, state, ready_at FROM artifacts WHERE project_id = ? AND kind = 'DownloadedVideo'",
      )
      .all(youtubeProject.id);
    const jobs = database
      .prepare(
        'SELECT id, project_id, status, progress_json, finished_at FROM jobs WHERE project_id = ? ORDER BY created_at',
      )
      .all(project.id);
    const outbox = database
      .prepare(
        'SELECT kind, payload_json, status, attempts, last_error, aggregate_id FROM outbox_messages ORDER BY created_at',
      )
      .all();
    const pendingYoutubeImports = database.prepare('SELECT COUNT(*) AS count FROM youtube_imports').get();
    if (
      jobs.length !== 1 ||
      jobs[0].status !== 'cancelled' ||
      !jobs[0].finished_at ||
      artifacts.length !== 1 ||
      artifacts[0].state !== 'ready' ||
      youtubeArtifacts.length !== 1 ||
      youtubeArtifacts[0].state !== 'ready' ||
      outbox.length !== 4 ||
      outbox.some((record) => record.status !== 'done') ||
      Number(pendingYoutubeImports.count) !== 0
    ) {
      return null;
    }
    return { project, artifacts, jobs, outbox, youtubeProject, youtubeArtifacts };
  } catch (error) {
    const message = String(error);
    if (
      message.includes('locked') ||
      message.includes('busy') ||
      message.includes('no such table')
    ) {
      return null;
    }
    throw error;
  } finally {
    database?.close();
  }
}

function runIdFromTitle(title) {
  return title.slice('native-e2e-complete:'.length);
}

async function verifyResult({
  dataRoot,
  sourcePath,
  ytdlpUrl,
  ytdlpRequestCount,
  project,
  artifacts,
  jobs,
  outbox,
  youtubeProject,
  youtubeArtifacts,
}) {
  assert(project.status === 'Cancelled', `Unexpected project status: ${project.status}`);
  assert(project.revision >= 5, `Expected project revision >= 5, got ${project.revision}`);

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

  assert(outbox.length === 4, `Expected four outbox records, found ${outbox.length}`);
  const finalizeOutbox = outbox.filter(
    (record) => record.kind === 'finalize_staged_artifact' && record.aggregate_id === project.id,
  );
  const terminalOutbox = outbox.filter((record) => record.kind === 'handle_terminal_job_state');
  assert(finalizeOutbox.length === 1, 'Expected one finalize outbox record');
  assert(terminalOutbox.length === 1, 'Expected one terminal job outbox record');
  for (const record of [...finalizeOutbox, ...terminalOutbox]) {
    assert(record.status === 'done', `${record.kind} outbox status is ${record.status}`);
    assert(record.last_error === null, `${record.kind} outbox record contains an error`);
  }
  assert(
    JSON.parse(terminalOutbox[0].payload_json).outcome === 'cancelled',
    'Terminal outbox did not preserve the cancelled outcome',
  );

  assert(jobs.length === 1, `Expected one pipeline job, found ${jobs.length}`);
  assert(jobs[0].project_id === project.id, 'Job belongs to a different project');
  assert(jobs[0].status === 'cancelled', `Unexpected job status: ${jobs[0].status}`);
  assert(jobs[0].finished_at, 'Cancelled job has no finished timestamp');
  assert(
    JSON.parse(jobs[0].progress_json).message === 'Runtime stopped; job cancelled',
    'Cancelled job retained stale progress text',
  );

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

  assert(
    youtubeProject.status === 'ReadyForProcessing',
    `Unexpected yt-dlp project status: ${youtubeProject.status}`,
  );
  const youtubeSource = JSON.parse(youtubeProject.source_json);
  assert(youtubeSource.YoutubeUrl?.url === ytdlpUrl, 'yt-dlp project did not preserve its source URL');
  assert(youtubeArtifacts.length === 1, 'Expected one downloaded yt-dlp artifact');
  const youtubeArtifact = youtubeArtifacts[0];
  assert(youtubeArtifact.project_id === youtubeProject.id, 'yt-dlp artifact ownership mismatch');
  assert(youtubeArtifact.location_kind === 'StorageKey', 'yt-dlp artifact is not managed');
  assert(
    youtubeArtifact.state === 'ready' && youtubeArtifact.ready_at,
    'yt-dlp artifact was not finalized',
  );
  assert(ytdlpRequestCount >= 2, `Expected yt-dlp HTTP traffic, observed ${ytdlpRequestCount}`);

  const youtubeArtifactPath = path.resolve(
    projectsRoot,
    ...youtubeArtifact.location_value.split('/'),
  );
  assert(
    youtubeArtifactPath.startsWith(`${projectsRoot}${path.sep}`),
    'yt-dlp artifact storage key escaped the managed project directory',
  );
  const youtubeArtifactStat = await fsp.stat(youtubeArtifactPath);
  assert(youtubeArtifactStat.isFile(), 'Final yt-dlp artifact path is not a file');
  assert(
    youtubeArtifactStat.size === sourceStat.size,
    'yt-dlp artifact size differs from the served fixture',
  );
  assert(
    (await sha256(youtubeArtifactPath)) === (await sha256(sourcePath)),
    'yt-dlp artifact bytes differ from the served fixture',
  );

  const youtubeFinalizeOutbox = outbox.filter(
    (record) =>
      record.kind === 'finalize_staged_artifact' && record.aggregate_id === youtubeProject.id,
  );
  const workspaceCleanupOutbox = outbox.filter(
    (record) => record.kind === 'delete_workspace_allocation',
  );
  assert(youtubeFinalizeOutbox.length === 1, 'Expected one yt-dlp finalize outbox record');
  assert(workspaceCleanupOutbox.length === 1, 'Expected one workspace cleanup outbox record');
  for (const record of outbox) {
    assert(record.status === 'done', `${record.kind} outbox status is ${record.status}`);
    assert(record.last_error === null, `${record.kind} outbox record contains an error`);
  }

  const stagingFiles = await listFiles(path.join(projectsRoot, '.staging'));
  assert(
    stagingFiles.length === 0,
    `Staging files remain after finalize: ${stagingFiles.join(', ')}`,
  );
  const workspaceFiles = await listFiles(path.join(dataRoot, 'cache', 'workspaces'));
  assert(
    workspaceFiles.length === 0,
    `Workspace files remain after cancellation: ${workspaceFiles.join(', ')}`,
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

async function readBootstrapCheckpoint(dataRoot) {
  try {
    const checkpoint = await fsp.readFile(
      path.join(dataRoot, 'native-e2e-bootstrap.txt'),
      'utf8',
    );
    return `\nNative E2E checkpoint trace:\n${checkpoint.trim() || '(empty)'}`;
  } catch (error) {
    if (error?.code === 'ENOENT') return '\nNative E2E checkpoint trace: (not created)';
    return '\nNative E2E checkpoint trace: (unreadable)';
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function delay(durationMs) {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}

async function terminate(child) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  const exited = new Promise((resolve) => child.once('exit', resolve));
  if (process.platform === 'win32') {
    child.kill('SIGKILL');
    await Promise.race([exited, delay(2_000)]);
    if (child.exitCode === null && child.signalCode === null) {
      childProcess.spawnSync(
        'powershell.exe',
        ['-NoProfile', '-NonInteractive', '-Command', 'Stop-Process', '-Id', String(child.pid), '-Force'],
        {
          stdio: 'ignore',
          windowsHide: true,
        },
      );
    }
  } else {
    child.kill('SIGKILL');
  }
  await Promise.race([exited, delay(5_000)]);
  if (process.platform === 'win32' && child.exitCode === null && child.signalCode === null) {
    childProcess.spawnSync('taskkill.exe', ['/pid', String(child.pid), '/t', '/f'], {
      stdio: 'ignore',
      windowsHide: true,
    });
    await Promise.race([exited, delay(3_000)]);
  }
  assert(
    child.exitCode !== null || child.signalCode !== null,
    `Native E2E process tree did not stop: ${child.pid}`,
  );
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  runNativeE2e(rootDir)
    .then(() =>
      process.stdout.write(
        'Native Tauri E2E passed: React -> IPC -> Rust -> SQLite -> ffprobe/yt-dlp -> filesystem; sandbox cleaned.\n',
      ),
    )
    .catch((error) => {
      process.stderr.write(`${error.message}\n`);
      process.exitCode = 1;
    });
}
