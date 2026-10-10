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
import {
  seedPackageCache,
  verifyInstalledPackage,
  waitForPackageSelection,
} from './package-install.mjs';
import { waitForResultCommitGap, verifyResultCommitGap } from './result-gap.mjs';
import { verifySecondOwnerRejected } from './second-owner.mjs';
import { verifyTranslationDeletion } from './verify-translation-deletion.mjs';
import { verifyTranslationResult } from './verify-translation.mjs';
import { verifyTranslationEvents } from './translation-events.mjs';
import {
  verifyTranslationHistory,
  waitForTranslationHistoryReopened,
} from './translation-history.mjs';
import { releaseVerifiedPause, verifyResumedPause } from './translation-pause.mjs';
import { releaseVerifiedPreparationPause, verifyResumedPreparation } from './preparation-pause.mjs';
import { releaseVerifiedWorkerPreflightPause } from './worker-preflight-pause.mjs';
import { waitForHistoricalEditGap, verifyHistoricalEditGap } from './historical-edit-gap.mjs';
import { releaseHistoricalJournalRetry } from './historical-journal-gap.mjs';
import { seedCasterRuntime, verifyCasterArtifact, waitForCasterTitle } from './caster.mjs';

const RESULT_TIMEOUT_MS = 60_000;
const TRANSLATION_RESULT_TIMEOUT_MS = 600_000;
const POLL_INTERVAL_MS = 100;

export async function runNativeE2e(rootDir) {
  assertSupportedPlatform();
  const translationEnabled = process.env.AURALIS_NATIVE_E2E_TRANSLATION === '1';
  const casterEnabled = process.env.AURALIS_NATIVE_E2E_CASTER === '1';
  if (casterEnabled && translationEnabled)
    throw new Error('Caster native E2E runs without Translate');
  const translationEventsEnabled = process.env.AURALIS_NATIVE_E2E_TRANSLATION_EVENTS === '1';
  const translationHistoryEnabled = process.env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY === '1';
  const historicalGapEnabled = process.env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY_GAP === '1';
  const historicalJournalOnly =
    process.env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY_JOURNAL_GAP === '1';
  if (historicalJournalOnly && !historicalGapEnabled)
    throw new Error('Journal-only interruption requires the historical gap mode');
  const historicalExportRequested =
    process.env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY_EXPORT === '1';
  if (historicalExportRequested && (!translationHistoryEnabled || historicalGapEnabled))
    throw new Error('Historical export requires the ordinary SRT history mode');
  const historicalExportEnabled = translationHistoryEnabled && historicalExportRequested;
  if (historicalGapEnabled && !translationHistoryEnabled)
    throw new Error('Historical core gap requires the standalone history mode');
  if (
    translationHistoryEnabled &&
    (!translationEnabled ||
      ['EVENTS', 'CRASH', 'PAUSE', 'PREPARATION_PAUSE', 'DELETE', 'PACKAGE'].some(
        (mode) => process.env[`AURALIS_NATIVE_E2E_TRANSLATION_${mode}`] === '1',
      ))
  )
    throw new Error('Translation history requires a separate ordinary SRT invocation');
  if (
    translationEventsEnabled &&
    (!translationEnabled ||
      ['VTT', 'CRASH', 'PAUSE', 'PREPARATION_PAUSE', 'DELETE', 'PACKAGE'].some(
        (mode) => process.env[`AURALIS_NATIVE_E2E_TRANSLATION_${mode}`] === '1',
      ))
  ) {
    throw new Error('Translation events require a separate ordinary SRT invocation');
  }
  const translationVttEnabled =
    translationEnabled && process.env.AURALIS_NATIVE_E2E_TRANSLATION_VTT === '1';
  const translationCrashEnabled =
    translationEnabled && process.env.AURALIS_NATIVE_E2E_TRANSLATION_CRASH === '1';
  const translationPauseEnabled =
    translationEnabled && process.env.AURALIS_NATIVE_E2E_TRANSLATION_PAUSE === '1';
  const translationPreparationPauseEnabled =
    translationEnabled && process.env.AURALIS_NATIVE_E2E_TRANSLATION_PREPARATION_PAUSE === '1';
  const translationWorkerPreflightPauseEnabled =
    process.env.AURALIS_NATIVE_E2E_TRANSLATION_WORKER_PREFLIGHT_PAUSE === '1';
  const translationInitialHashPauseEnabled =
    process.env.AURALIS_NATIVE_E2E_TRANSLATION_INITIAL_HASH_PAUSE === '1';
  const translationStartRaceEnabled = process.env.AURALIS_NATIVE_E2E_TRANSLATION_START_RACE === '1';
  if (translationStartRaceEnabled && !translationInitialHashPauseEnabled) {
    throw new Error('Concurrent start observation requires the initial-hash preparation mode');
  }
  if (
    translationInitialHashPauseEnabled &&
    (!translationPreparationPauseEnabled || translationWorkerPreflightPauseEnabled)
  ) {
    throw new Error('Initial hash pause requires a separate preparation-pause invocation');
  }
  if (translationWorkerPreflightPauseEnabled && !translationPreparationPauseEnabled) {
    throw new Error('Worker preflight pause requires the preparation-pause mode');
  }
  const translationResultGapEnabled =
    translationCrashEnabled && process.env.AURALIS_NATIVE_E2E_TRANSLATION_RESULT_GAP === '1';
  const translationDeleteEnabled =
    translationEnabled && process.env.AURALIS_NATIVE_E2E_TRANSLATION_DELETE === '1';
  const translationPackageEnabled =
    translationEnabled && process.env.AURALIS_NATIVE_E2E_TRANSLATION_PACKAGE === '1';
  if (
    translationPreparationPauseEnabled &&
    (translationPauseEnabled ||
      translationCrashEnabled ||
      translationDeleteEnabled ||
      translationPackageEnabled)
  ) {
    throw new Error(
      'Native preparation pause runs separately from inference pause, crash, deletion and package modes',
    );
  }
  if (
    translationPauseEnabled &&
    (translationCrashEnabled || translationDeleteEnabled || translationPackageEnabled)
  ) {
    throw new Error(
      'Native pause acceptance does not combine with crash, deletion or package modes',
    );
  }
  if (translationVttEnabled && (translationResultGapEnabled || translationDeleteEnabled)) {
    throw new Error('Native WebVTT acceptance does not combine with result-gap or deletion modes');
  }
  if (
    translationPackageEnabled &&
    (translationVttEnabled || translationCrashEnabled || translationDeleteEnabled)
  ) {
    throw new Error('Native package acceptance runs as a standalone translation mode');
  }
  const testRoot = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-native-e2e-'));
  const dataRoot = path.join(testRoot, 'app-data');
  const fixtureRoot = path.join(testRoot, 'fixture');
  const sourcePath = path.join(fixtureRoot, 'source.mp4');
  const subtitlePath = path.join(fixtureRoot, translationVttEnabled ? 'source.vtt' : 'source.srt');
  const frontendBuildRoot = path.join(rootDir, 'apps', 'desktop', 'dist-native-e2e');
  const runId = 'isolated-run';
  const secondOwnerEnabled = process.env.AURALIS_NATIVE_E2E_SECOND_OWNER === '1';
  const resultCommitMarker = path.join(dataRoot, 'native-result-committed.txt');
  const historicalCommitMarker = path.join(dataRoot, 'native-historical-edit-committed.txt');
  const historicalRequestPath = path.join(dataRoot, 'native-historical-edit-request.json');
  const historicalRetryRelease = path.join(dataRoot, 'native-historical-edit-retry.txt');
  let historicalGap;
  let app;
  let fixtureServer;
  let failure;
  let interrupted;
  let baselineModelPids;
  let subtitleDigest;
  let packageRelease;
  let preparationPause;

  try {
    await fsp.mkdir(dataRoot, { recursive: true });
    await fsp.mkdir(fixtureRoot, { recursive: true });
    if (casterEnabled) await seedCasterRuntime(dataRoot);
    prepareMediaTools(rootDir);
    generateVideo(rootDir, sourcePath);
    if (translationEnabled) {
      await prepareTranslationFixture(
        dataRoot,
        subtitlePath,
        translationCrashEnabled ||
          translationDeleteEnabled ||
          translationPauseEnabled ||
          translationHistoryEnabled,
        translationVttEnabled,
        translationPackageEnabled,
        translationPauseEnabled,
      );
      subtitleDigest = await sha256(subtitlePath);
      if (translationPackageEnabled) {
        packageRelease = await seedPackageCache(
          rootDir,
          dataRoot,
          process.env.AURALIS_TEST_PACKAGE_ASSET_DIR,
        );
      }
    }
    fixtureServer = await startFixtureServer(sourcePath);
    buildTestApplication(rootDir, dataRoot, sourcePath, fixtureServer.url, runId, {
      casterEnabled,
      translationEnabled,
      translationCrashEnabled,
      translationPauseEnabled,
      translationPreparationPauseEnabled,
      translationWorkerPreflightPauseEnabled,
      translationStartRaceEnabled,
      translationDeleteEnabled,
      translationVttEnabled,
      translationPackageEnabled,
      subtitlePath,
    });

    if (
      translationCrashEnabled ||
      translationDeleteEnabled ||
      translationPauseEnabled ||
      translationPreparationPauseEnabled ||
      translationHistoryEnabled
    )
      baselineModelPids = listModelProcessIds();
    app = launchTestApplication(
      rootDir,
      dataRoot,
      fixtureServer.url,
      translationResultGapEnabled ? resultCommitMarker : null,
      historicalGapEnabled ? historicalCommitMarker : null,
      historicalJournalOnly
        ? { requestPath: historicalRequestPath, releasePath: historicalRetryRelease }
        : null,
    );
    if (casterEnabled) {
      const projectId = await waitForCasterTitle(
        dataRoot,
        app,
        `native-e2e-caster-complete:${runId}`,
        runId,
      );
      const revision = await verifyCasterArtifact(dataRoot, projectId);
      await terminate(app.child);
      app = launchTestApplication(rootDir, dataRoot, fixtureServer.url);
      const reopenedId = await waitForCasterTitle(
        dataRoot,
        app,
        `native-e2e-caster-reopened:${runId}`,
        runId,
      );
      assert(reopenedId === projectId, 'Caster restart changed the project identity');
      assert(
        (await verifyCasterArtifact(dataRoot, projectId)) === revision,
        'Caster restart changed the active revision',
      );
      return;
    }
    if (translationPackageEnabled) {
      await waitForPackageSelection(dataRoot, app);
      await terminate(app.child);
      await verifyInstalledPackage(dataRoot, packageRelease);
      app = launchTestApplication(rootDir, dataRoot, fixtureServer.url);
    }
    if (secondOwnerEnabled) {
      await verifySecondOwnerRejected(dataRoot, app, () =>
        launchTestApplication(rootDir, dataRoot, fixtureServer.url),
      );
    }
    if (translationPauseEnabled) {
      interrupted = await releaseVerifiedPause(
        dataRoot,
        runId,
        app,
        baselineModelPids,
        listModelProcessIds,
      );
    }
    if (translationPreparationPauseEnabled) {
      preparationPause = await releaseVerifiedPreparationPause(
        dataRoot,
        runId,
        app,
        baselineModelPids,
        listModelProcessIds,
        subtitleDigest,
        translationInitialHashPauseEnabled
          ? {
              initialHashBytes: fs.statSync(process.env.AURALIS_TEST_GGUF).size,
              concurrentStarts: translationStartRaceEnabled,
            }
          : {},
      );
    }
    if (translationWorkerPreflightPauseEnabled) {
      preparationPause.workerPause = await releaseVerifiedWorkerPreflightPause(
        dataRoot,
        runId,
        app,
        preparationPause,
        listModelProcessIds,
      );
    }
    if (translationCrashEnabled) {
      if (translationResultGapEnabled) {
        interrupted = await waitForResultCommitGap(
          dataRoot,
          resultCommitMarker,
          runId,
          app,
          TRANSLATION_RESULT_TIMEOUT_MS,
          POLL_INTERVAL_MS,
        );
        assert(
          [...listModelProcessIds()].every((pid) => baselineModelPids.has(pid)),
          'Managed model remained alive after Translate result commit',
        );
        await killNativeProcess(app.child);
        verifyResultCommitGap(dataRoot, runId, interrupted);
      } else {
        interrupted = await waitForTranslationCheckpoint(dataRoot, runId, app, baselineModelPids);
        await crashNativeProcess(app.child, baselineModelPids, interrupted.managedModelPids);
        verifyInterruptedTranslation(dataRoot, runId, interrupted);
      }
      app = launchTestApplication(rootDir, dataRoot, fixtureServer.url);
    }
    if (historicalGapEnabled) {
      historicalGap = await waitForHistoricalEditGap(dataRoot, historicalCommitMarker, app, {
        journalOnly: historicalJournalOnly,
        requestPath: historicalRequestPath,
      });
      assert(
        [...listModelProcessIds()].every((pid) => baselineModelPids.has(pid)),
        'Managed model remained alive before historical commit interruption',
      );
      await killNativeProcess(app.child);
      await verifyHistoricalEditGap(dataRoot, historicalGap, true);
      app = launchTestApplication(
        rootDir,
        dataRoot,
        fixtureServer.url,
        null,
        null,
        historicalJournalOnly
          ? { requestPath: historicalRequestPath, releasePath: historicalRetryRelease }
          : null,
      );
      if (historicalJournalOnly)
        await releaseHistoricalJournalRetry(dataRoot, historicalGap, app, historicalRetryRelease);
    }
    const result = await waitForResult(
      path.join(dataRoot, 'auralis.sqlite'),
      runId,
      app,
      translationPackageEnabled
        ? TRANSLATION_RESULT_TIMEOUT_MS * 2
        : translationEnabled
          ? TRANSLATION_RESULT_TIMEOUT_MS
          : RESULT_TIMEOUT_MS,
    );
    const deletedTranslationProjectId = translationDeleteEnabled
      ? await verifyTranslationDeletion(
          dataRoot,
          subtitlePath,
          subtitleDigest,
          baselineModelPids,
          listModelProcessIds,
        )
      : null;
    await verifyResult({
      dataRoot,
      sourcePath,
      ytdlpUrl: fixtureServer.url,
      ytdlpRequestCount: fixtureServer.requestCount,
      deletedTranslationProjectId,
      ...result,
    });
    if (translationEnabled && !translationDeleteEnabled) {
      await verifyTranslationResult(
        dataRoot,
        subtitlePath,
        runId,
        interrupted,
        translationVttEnabled ? 'vtt' : 'srt',
        translationHistoryEnabled ? 4 : 2,
      );
      if (translationHistoryEnabled) {
        const beforeRestart = await verifyTranslationHistory(
          dataRoot,
          runId,
          null,
          historicalGapEnabled,
          historicalJournalOnly,
          historicalExportEnabled,
        );
        assert(
          [...listModelProcessIds()].every((pid) => baselineModelPids.has(pid)),
          'Managed model remained alive after historical edits',
        );
        if (historicalGapEnabled) {
          await verifyHistoricalEditGap(dataRoot, historicalGap, false);
        } else {
          await terminate(app.child);
          app = launchTestApplication(rootDir, dataRoot, fixtureServer.url);
          await waitForTranslationHistoryReopened(dataRoot, app);
          await verifyTranslationHistory(
            dataRoot,
            runId,
            beforeRestart,
            false,
            false,
            historicalExportEnabled,
          );
          await verifyTranslationResult(
            dataRoot,
            subtitlePath,
            runId,
            null,
            translationVttEnabled ? 'vtt' : 'srt',
            4,
          );
        }
      }
      if (translationPauseEnabled) verifyResumedPause(dataRoot, interrupted);
      if (translationEventsEnabled) await verifyTranslationEvents(dataRoot, runId);
      if (translationPreparationPauseEnabled)
        await verifyResumedPreparation(dataRoot, preparationPause, listModelProcessIds);
      assert(
        (await sha256(subtitlePath)) === subtitleDigest,
        'Translation changed the external source after its initial digest',
      );
    }
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
    if (process.env.AURALIS_NATIVE_E2E_KEEP_TEMP === '1' || (failure && historicalExportEnabled)) {
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

async function prepareTranslationFixture(
  dataRoot,
  subtitlePath,
  translationCrashEnabled,
  translationVttEnabled,
  translationPackageEnabled,
  translationPauseEnabled = false,
) {
  const firstCue = '1\n00:00:01,000 --> 00:00:02,000\n你好。\n';
  const secondLine = translationPauseEnabled
    ? '我们先检查原来的字幕文件，再把需要翻译的文字取出来。翻译完成以后，程序会创建一个新的文件，保留原来的时间和顺序。如果中途停止，已经保存的部分不需要重新翻译。'
    : '再见。';
  const secondCue = `2\n00:00:03,000 --> 00:00:04,000\n${secondLine}\n`;
  const vttHeader = 'WEBVTT\r\n\r\nNOTE provenance\r\nnative fixture\r\n\r\n';
  const vttFirstCue = '00:00:01.000 --> 00:00:02.000\r\n你好。\r\n';
  const vttSecondCue = `\r\n00:00:03.000 --> 00:00:04.000\r\n${secondLine}\r\n`;
  const vttSource = `${vttHeader}${vttFirstCue}${translationCrashEnabled ? vttSecondCue : ''}`;
  await fsp.writeFile(
    subtitlePath,
    translationVttEnabled
      ? vttSource
      : translationCrashEnabled
        ? `${firstCue}\n${secondCue}`
        : firstCue,
  );
  if (translationPackageEnabled) return;
  const executable = process.env.AURALIS_TEST_LLAMA_SERVER;
  const modelFile = process.env.AURALIS_TEST_GGUF;
  if (!executable || !modelFile || !path.isAbsolute(executable) || !path.isAbsolute(modelFile)) {
    throw new Error(
      'Translation native E2E requires absolute AURALIS_TEST_LLAMA_SERVER and AURALIS_TEST_GGUF paths',
    );
  }
  for (const asset of [executable, modelFile]) {
    if (!(await fsp.stat(asset)).isFile())
      throw new Error(`Translation asset is not a file: ${asset}`);
  }
  await fsp.writeFile(
    path.join(dataRoot, 'translation-runtime.json'),
    JSON.stringify({
      executable,
      model_file: modelFile,
      context_tokens: 2048,
      gpu_layers: 99,
      startup_timeout_seconds: 120,
      readiness_interval_milliseconds: 250,
    }),
  );
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
      if (
        !Number.isSafeInteger(start) ||
        !Number.isSafeInteger(end) ||
        start > end ||
        start >= size
      ) {
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

function buildTestApplication(rootDir, dataRoot, sourcePath, ytdlpUrl, runId, translation) {
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

function launchTestApplication(
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

async function waitForTranslationCheckpoint(dataRoot, runId, app, baselineModelPids) {
  const deadline = Date.now() + TRANSLATION_RESULT_TIMEOUT_MS;
  let managedModelPids = [];
  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null) {
      throw new Error(
        `Native application exited before its first translation checkpoint: ${app.child.exitCode}`,
      );
    }
    const interrupted = readTranslationCheckpoint(dataRoot, runId);
    if (interrupted?.runState === 'running' && managedModelPids.length === 0) {
      managedModelPids = [...listModelProcessIds()].filter((pid) => !baselineModelPids.has(pid));
    }
    if (interrupted?.savedBlocks === 1 && interrupted.runState === 'running') {
      return { kind: 'checkpoint', ...interrupted, managedModelPids };
    }
    if (interrupted?.savedBlocks > 1 || interrupted?.runState === 'validated') {
      throw new Error('Native translation completed before the process-kill boundary');
    }
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Timed out waiting for a committed native translation checkpoint');
}

function readTranslationCheckpoint(dataRoot, runId) {
  const hostPath = path.join(dataRoot, 'auralis.sqlite');
  const translatePath = path.join(dataRoot, 'auralis-translate.sqlite');
  if (!fs.existsSync(hostPath) || !fs.existsSync(translatePath)) return null;
  let host;
  let translate;
  try {
    host = new DatabaseSync(hostPath, { readOnly: true });
    translate = new DatabaseSync(translatePath, { readOnly: true });
    const link = host
      .prepare(
        'SELECT pt.translation_id, pt.active_run_id FROM project_translations pt JOIN projects p ON p.id = pt.project_id WHERE p.title = ?',
      )
      .get(`native-e2e-translation-running:${runId}`);
    if (!link?.active_run_id) return null;
    const run = translate
      .prepare('SELECT state FROM runs WHERE run_id = ?')
      .get(link.active_run_id);
    const checkpoint = translate
      .prepare('SELECT COUNT(*) AS count FROM block_checkpoints WHERE run_id = ?')
      .get(link.active_run_id);
    const firstBlock = translate
      .prepare(
        'SELECT input_fingerprint, accepted_json, committed_at FROM block_checkpoints WHERE run_id = ? AND block_index = 0',
      )
      .get(link.active_run_id);
    return {
      translationId: link.translation_id,
      runId: link.active_run_id,
      runState: run?.state,
      savedBlocks: Number(checkpoint.count),
      firstBlock,
    };
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
    translate?.close();
    host?.close();
  }
}

async function crashNativeProcess(child, baselineModelPids, knownModelPids = []) {
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

async function killNativeProcess(child) {
  const exited = new Promise((resolve) => child.once('exit', resolve));
  assert(child.kill('SIGKILL'), 'Could not terminate the native desktop process');
  await Promise.race([exited, delay(10_000)]);
  assert(
    child.exitCode !== null || child.signalCode !== null,
    'Native desktop process survived the crash injection',
  );
}

function listModelProcessIds() {
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

function verifyInterruptedTranslation(dataRoot, runId, expected) {
  const actual = readTranslationCheckpoint(dataRoot, runId);
  assert(
    actual?.translationId === expected.translationId,
    'Crash changed the project translation link',
  );
  assert(actual.runId === expected.runId, 'Crash changed the unfinished project run');
  assert(
    actual.savedBlocks === 1 && actual.runState === 'running',
    `Crash boundary changed: expected one running checkpoint, observed ${actual.savedBlocks} and ${actual.runState}`,
  );
  assert(
    JSON.stringify(actual.firstBlock) === JSON.stringify(expected.firstBlock),
    'Crash changed the committed first block',
  );
}

async function waitForResult(databasePath, runId, app, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
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
  throw new Error(`Timed out after ${timeoutMs} ms waiting for the native E2E result`);
}

function tryReadResult(databasePath, completedTitle, failedTitle) {
  if (!fs.existsSync(databasePath)) return null;
  let database;
  try {
    database = new DatabaseSync(databasePath, { readOnly: true });
    const failedProject = database
      .prepare('SELECT id FROM projects WHERE title = ?')
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
        'SELECT kind, payload_json, status, attempts, last_error, aggregate_id FROM outbox_messages ORDER BY rowid LIMIT 4',
      )
      .all();
    const pendingYoutubeImports = database
      .prepare('SELECT COUNT(*) AS count FROM youtube_imports')
      .get();
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
  outbox: allOutbox,
  deletedTranslationProjectId,
  youtubeProject,
  youtubeArtifacts,
}) {
  const outbox = deletedTranslationProjectId
    ? allOutbox.filter(
        (record) =>
          record.aggregate_id !== deletedTranslationProjectId &&
          ![
            'delete_storage_key',
            'delete_project_artifact_dir',
            'delete_project_translation',
          ].includes(record.kind),
      )
    : allOutbox;
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
  assert(
    youtubeSource.YoutubeUrl?.url === ytdlpUrl,
    'yt-dlp project did not preserve its source URL',
  );
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

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  runNativeE2e(rootDir)
    .then(() =>
      process.stdout.write(
        process.env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY_EXPORT === '1'
          ? 'Native Tauri history export E2E passed: exact historical IPC exports, cold retry and offline re-export; sandbox cleaned.\n'
          : process.env.AURALIS_NATIVE_E2E_TRANSLATION_HISTORY === '1'
            ? 'Native Tauri history E2E passed: actual editing, historical preview and selection, retained attachment after restart and immutable outputs; sandbox cleaned.\n'
            : process.env.AURALIS_NATIVE_E2E_SECOND_OWNER === '1'
              ? 'Native Tauri single-owner E2E passed: a second process was rejected before storage setup, while the first completed its workflow; sandbox cleaned.\n'
              : process.env.AURALIS_NATIVE_E2E_TRANSLATION_START_RACE === '1'
                ? 'Native concurrent starts E2E passed: parallel requests were busy during observed initial hashing, reused one accepted host job during worker preflight, and preserved pause, fresh resume and the separate ready result; sandbox cleaned.\n'
                : process.env.AURALIS_NATIVE_E2E_TRANSLATION_INITIAL_HASH_PAUSE === '1'
                  ? 'Native initial hash pause E2E passed: actual controls stopped an observed incomplete model hash before any child/job/attempt, then freshly verified and resumed the same run to a separate ready result; sandbox cleaned.\n'
                  : process.env.AURALIS_NATIVE_E2E_TRANSLATION_WORKER_PREFLIGHT_PAUSE === '1'
                    ? 'Native worker preflight pause E2E passed: actual controls stopped initial admission and accepted host preflight without a Translate attempt, released both children, and resumed the same run to a separate ready result; sandbox cleaned.\n'
                    : process.env.AURALIS_NATIVE_E2E_TRANSLATION_PREPARATION_PAUSE === '1'
                      ? 'Native preparation pause E2E passed: actual controls paused before any job/attempt, released the observed model child and resumed the same run to a separate ready result; sandbox cleaned.\n'
                      : process.env.AURALIS_NATIVE_E2E_TRANSLATION_PAUSE === '1'
                        ? 'Native Tauri pause E2E passed: retained checkpoint and active run, cancelled host job, released managed child and resumed in the same desktop; sandbox cleaned.\n'
                        : process.env.AURALIS_NATIVE_E2E_TRANSLATION_DELETE === '1'
                          ? 'Native Tauri deletion E2E passed: active checked-model translation stopped and both databases removed the project state; sandbox cleaned.\n'
                          : process.env.AURALIS_NATIVE_E2E_TRANSLATION_RESULT_GAP === '1'
                            ? 'Native Tauri result-gap E2E passed: a committed Translate result was recovered into a ready separate Auralis artifact after restart; sandbox cleaned.\n'
                            : process.env.AURALIS_NATIVE_E2E_TRANSLATION_VTT === '1' &&
                                process.env.AURALIS_NATIVE_E2E_TRANSLATION_CRASH === '1'
                              ? 'Native Tauri WebVTT crash E2E passed: first checkpoint survived desktop termination and the resumed run published a separate copy; sandbox cleaned.\n'
                              : process.env.AURALIS_NATIVE_E2E_TRANSLATION_CRASH === '1'
                                ? 'Native Tauri crash E2E passed: committed checkpoint survived desktop termination and resumed through a second checked-model attempt; sandbox cleaned.\n'
                                : process.env.AURALIS_NATIVE_E2E_TRANSLATION_PACKAGE === '1'
                                  ? 'Native Tauri package E2E passed: React installed and selected the pinned CPU package, desktop restart restored it, and a real subtitle translation published a separate output; sandbox cleaned.\n'
                                  : process.env.AURALIS_NATIVE_E2E_TRANSLATION_VTT === '1'
                                    ? 'Native Tauri WebVTT E2E passed: checked local translation preserved the separate source, protected spans, and both databases; sandbox cleaned.\n'
                                    : process.env.AURALIS_NATIVE_E2E_TRANSLATION === '1'
                                      ? 'Native Tauri E2E passed: React -> IPC -> Rust -> checked local translation -> two SQLite files -> separate output; sandbox cleaned.\n'
                                      : process.env.AURALIS_NATIVE_E2E_CASTER === '1'
                                        ? 'Native Caster E2E passed: React catalog controls -> typed IPC -> installed worker -> managed cast and prepared WAV -> restart; sandbox cleaned.\n'
                                        : 'Native Tauri E2E passed: React -> IPC -> Rust -> SQLite -> ffprobe/yt-dlp -> filesystem; sandbox cleaned.\n',
      ),
    )
    .catch((error) => {
      process.stderr.write(`${error.message}\n`);
      process.exitCode = 1;
    });
}
