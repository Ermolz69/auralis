import fs from 'node:fs';
import fsp from 'node:fs/promises';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
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
import { readScenarioConfig, successMessage } from './scenario-config.mjs';
import {
  prepareTranslationFixture,
  prepareMediaTools,
  generateVideo,
  startFixtureServer,
  sha256,
} from './fixture.mjs';
import {
  buildTestApplication,
  launchTestApplication,
  crashNativeProcess,
  killNativeProcess,
  listModelProcessIds,
  formatProcessLogs,
  readBootstrapCheckpoint,
  terminate,
  cleanupNativeE2e,
} from './app-lifecycle.mjs';
import {
  TRANSLATION_RESULT_TIMEOUT_MS,
  POLL_INTERVAL_MS,
  waitForTranslationCheckpoint,
  verifyInterruptedTranslation,
  waitForResult,
  verifyResult,
} from './verify-result.mjs';

const RESULT_TIMEOUT_MS = 60_000;

export async function runNativeE2e(rootDir) {
  assertSupportedPlatform();
  const {
    translationEnabled,
    casterEnabled,
    translationEventsEnabled,
    translationHistoryEnabled,
    historicalGapEnabled,
    historicalJournalOnly,
    historicalExportEnabled,
    translationVttEnabled,
    translationCrashEnabled,
    translationPauseEnabled,
    translationPreparationPauseEnabled,
    translationWorkerPreflightPauseEnabled,
    translationInitialHashPauseEnabled,
    translationStartRaceEnabled,
    translationResultGapEnabled,
    translationDeleteEnabled,
    translationPackageEnabled,
    secondOwnerEnabled,
  } = readScenarioConfig();
  const testRoot = await fsp.mkdtemp(path.join(os.tmpdir(), 'auralis-native-e2e-'));
  const dataRoot = path.join(testRoot, 'app-data');
  const fixtureRoot = path.join(testRoot, 'fixture');
  const sourcePath = path.join(fixtureRoot, 'source.mp4');
  const subtitlePath = path.join(fixtureRoot, translationVttEnabled ? 'source.vtt' : 'source.srt');
  const frontendBuildRoot = path.join(rootDir, 'apps', 'desktop', 'dist-native-e2e');
  const runId = 'isolated-run';
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
    failure = await cleanupNativeE2e({
      app,
      fixtureServer,
      testRoot,
      frontendBuildRoot,
      failure,
      historicalExportEnabled,
      keepTemp: process.env.AURALIS_NATIVE_E2E_KEEP_TEMP === '1',
    });
  }

  if (failure) throw failure;
}

function assertSupportedPlatform() {
  if (process.platform !== 'win32') {
    throw new Error(`Native Tauri E2E is currently supported on Windows, not ${process.platform}`);
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

const currentFile = fileURLToPath(import.meta.url);
if (path.resolve(process.argv[1] ?? '') === currentFile) {
  const rootDir = path.resolve(path.dirname(currentFile), '../..');
  runNativeE2e(rootDir)
    .then(() => process.stdout.write(successMessage()))
    .catch((error) => {
      process.stderr.write(`${error.message}\n`);
      process.exitCode = 1;
    });
}
