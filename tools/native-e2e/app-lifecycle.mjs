import childProcess from 'node:child_process';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { run } from './fixture.mjs';

export function buildTestApplication(rootDir, dataRoot, sourcePath, ytdlpUrl, runId, translation) {
  const env = {
    ...process.env,
    AURALIS_NATIVE_E2E: '1',
    AURALIS_NATIVE_E2E_DATA_DIR: dataRoot,
    AURALIS_NATIVE_E2E_MEDIA_PATH: sourcePath,
    AURALIS_NATIVE_E2E_YTDLP_URL: ytdlpUrl,
    AURALIS_NATIVE_E2E_RUN_ID: runId,
    AURALIS_NATIVE_E2E_CASTER: translation.casterEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_TRANSLATION: translation.translationEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_TRANSLATION_VTT: translation.translationVttEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_TRANSLATION_PACKAGE: translation.translationPackageEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_TRANSLATION_CRASH: translation.translationCrashEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_TRANSLATION_PAUSE: translation.translationPauseEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_TRANSLATION_PREPARATION_PAUSE: translation.translationPreparationPauseEnabled
      ? '1'
      : '0',
    AURALIS_NATIVE_E2E_TRANSLATION_WORKER_PREFLIGHT_PAUSE:
      translation.translationWorkerPreflightPauseEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_TRANSLATION_START_RACE: translation.translationStartRaceEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_TRANSLATION_DELETE: translation.translationDeleteEnabled ? '1' : '0',
    AURALIS_NATIVE_E2E_SUBTITLE_PATH: translation.subtitlePath,
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

export function launchTestApplication(
  rootDir,
  dataRoot,
  ytdlpUrl,
  resultCommitMarker = null,
  historicalCommitMarker = null,
  historicalJournal = null,
) {
  const targetDir = path.resolve(rootDir, process.env.CARGO_TARGET_DIR ?? 'target');
  const executable = path.join(targetDir, 'debug', 'auralis-app.exe');
  if (!fs.existsSync(executable))
    throw new Error(`Native E2E executable is missing: ${executable}`);

  const child = childProcess.spawn(executable, [], {
    cwd: rootDir,
    env: {
      ...process.env,
      ...(process.env.AURALIS_NATIVE_E2E_TRANSLATION_INITIAL_HASH_PAUSE === '1'
        ? { RUST_LOG: 'warn,auralis_translation_llamacpp::model_hash=info' }
        : {}),
      AURALIS_NATIVE_E2E_DATA_DIR: dataRoot,
      AURALIS_NATIVE_E2E_YTDLP_URL: ytdlpUrl,
      ...(resultCommitMarker
        ? { AURALIS_NATIVE_E2E_RESULT_COMMITTED_PATH: resultCommitMarker }
        : {}),
      ...(historicalCommitMarker
        ? { AURALIS_NATIVE_E2E_HISTORICAL_COMMITTED_PATH: historicalCommitMarker }
        : {}),
      ...(historicalJournal
        ? {
            AURALIS_NATIVE_E2E_HISTORICAL_REQUEST_PATH: historicalJournal.requestPath,
            AURALIS_NATIVE_E2E_HISTORICAL_RETRY_RELEASE_PATH: historicalJournal.releasePath,
          }
        : {}),
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

export async function crashNativeProcess(child, baselineModelPids, knownModelPids = []) {
  assert(
    child.exitCode === null && child.signalCode === null,
    'Native desktop exited before crash injection',
  );
  const managedModelPids =
    knownModelPids.length > 0
      ? knownModelPids
      : [...listModelProcessIds()].filter((pid) => !baselineModelPids.has(pid));
  assert(managedModelPids.length > 0, 'Native desktop has no managed model process to interrupt');
  await killNativeProcess(child);
  const deadline = Date.now() + 10_000;
  while (Date.now() < deadline) {
    const active = listModelProcessIds();
    if (managedModelPids.every((pid) => !active.has(pid))) return;
    await delay(100);
  }
  throw new Error(
    `Managed model process survived native desktop termination: ${managedModelPids.join(', ')}`,
  );
}

export async function killNativeProcess(child) {
  const exited = new Promise((resolve) => child.once('exit', resolve));
  assert(child.kill('SIGKILL'), 'Could not terminate the native desktop process');
  await Promise.race([exited, delay(10_000)]);
  assert(
    child.exitCode !== null || child.signalCode !== null,
    'Native desktop process survived the crash injection',
  );
}

export function listModelProcessIds() {
  const result = childProcess.spawnSync(
    'tasklist.exe',
    ['/FI', 'IMAGENAME eq llama-server.exe', '/FO', 'CSV', '/NH'],
    { encoding: 'utf8', windowsHide: true },
  );
  assert(result.status === 0, `Could not list model processes: ${result.stderr?.trim()}`);
  return new Set(
    [...result.stdout.matchAll(/"llama-server\.exe","(\d+)"/gi)].map((match) => Number(match[1])),
  );
}

function appendLog(current, chunk) {
  return `${current}${chunk}`.slice(-16_000);
}

export function formatProcessLogs(app) {
  const stdout = app.stdout.trim();
  const stderr = app.stderr.trim();
  return `\nNative app stdout:\n${stdout || '(empty)'}\nNative app stderr:\n${stderr || '(empty)'}`;
}

export async function readBootstrapCheckpoint(dataRoot) {
  try {
    const checkpoint = await fsp.readFile(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8');
    return `\nNative E2E checkpoint trace:\n${checkpoint.trim() || '(empty)'}`;
  } catch (error) {
    if (error?.code === 'ENOENT') return '\nNative E2E checkpoint trace: (not created)';
    return '\nNative E2E checkpoint trace: (unreadable)';
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

export function delay(durationMs) {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}

export async function terminate(child) {
  if (child.exitCode !== null || child.signalCode !== null) return;
  const exited = new Promise((resolve) => child.once('exit', resolve));
  if (process.platform === 'win32') {
    child.kill('SIGKILL');
    await Promise.race([exited, delay(2_000)]);
    if (child.exitCode === null && child.signalCode === null) {
      childProcess.spawnSync(
        'powershell.exe',
        [
          '-NoProfile',
          '-NonInteractive',
          '-Command',
          'Stop-Process',
          '-Id',
          String(child.pid),
          '-Force',
        ],
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

export async function cleanupNativeE2e(
  { app, fixtureServer, testRoot, frontendBuildRoot, failure, historicalExportEnabled, keepTemp },
  adapters = {},
) {
  const terminateProcess = adapters.terminateProcess ?? terminate;
  const remove = adapters.remove ?? fsp.rm;
  const exists = adapters.exists ?? fs.existsSync;
  const write = adapters.write ?? ((message) => process.stdout.write(message));
  const cleanupErrors = [];
  if (app) {
    try {
      await terminateProcess(app.child);
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
  if (keepTemp || (failure && historicalExportEnabled)) {
    write(`Native E2E data kept at ${testRoot}\n`);
  } else {
    try {
      await remove(testRoot, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 });
      assert(!exists(testRoot), `Native E2E sandbox was not removed: ${testRoot}`);
    } catch (error) {
      cleanupErrors.push(error);
    }
  }
  try {
    await remove(frontendBuildRoot, {
      recursive: true,
      force: true,
      maxRetries: 10,
      retryDelay: 200,
    });
    assert(
      !exists(frontendBuildRoot),
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
  return failure;
}
