import childProcess from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';
import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';
import { setTimeout as delay } from 'node:timers/promises';
import {
  readPreparationSnapshot,
  releasePreparationGate,
  verifyEmptyPreparation,
} from './preparation-snapshot.mjs';

const TIMEOUT_MS = 180_000;
const POLL_MS = 100;

export async function releaseVerifiedPreparationPause(
  dataRoot,
  scenarioId,
  app,
  baselinePids,
  listPids,
  sourceSha256,
) {
  const deadline = Date.now() + TIMEOUT_MS;
  let preparing;
  let managedPids;
  while (Date.now() < deadline) {
    assertDesktopAlive(app);
    const trace = fs.existsSync(path.join(dataRoot, 'native-e2e-bootstrap.txt'))
      ? fs.readFileSync(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8')
      : '';
    preparing = readPreparationSnapshot(dataRoot, `native-e2e-translation-running:${scenarioId}`);
    if (preparing && trace.includes('translation-preparation-started\n')) {
      managedPids = [...listPids()].filter((pid) => !baselinePids.has(pid));
      if (managedPids.length) {
        assert.equal(managedPids.length, 1, 'Preparation started multiple model children');
        assertModelParent(managedPids[0], app.child.pid);
        verifyEmptyPreparation(preparing, 'requested', sourceSha256);
        releasePreparationGate(
          dataRoot,
          preparing,
          `native-e2e-preparation-pause-now:${scenarioId}`,
        );
        break;
      }
    }
    await delay(POLL_MS);
  }
  assert(managedPids?.length, 'No owned model child was observed before preparation pause');
  const pauseDeadline = Date.now() + TIMEOUT_MS;
  while (Date.now() < pauseDeadline) {
    assertDesktopAlive(app);
    const paused = readPreparationSnapshot(
      dataRoot,
      `native-e2e-preparation-paused:${scenarioId}:`,
    );
    if (paused) {
      verifyEmptyPreparation(paused, 'paused', sourceSha256);
      assert.equal(paused.run.run_id, preparing.run.run_id);
      assert.equal(paused.link.translation_id, preparing.link.translation_id);
      const acknowledgementMs = Number(paused.project.title.split(':').at(-1));
      assert(
        Number.isInteger(acknowledgementMs) && acknowledgementMs > 0 && acknowledgementMs <= 10_000,
      );
      assert(
        [...listPids()].every((pid) => baselinePids.has(pid)),
        'Model survived preparation acknowledgment',
      );
      releasePreparationGate(dataRoot, paused, `native-e2e-preparation-resume:${scenarioId}`);
      return { preparing, paused, managedPids, acknowledgementMs, baselinePids };
    }
    await delay(POLL_MS);
  }
  throw new Error('Native controls did not acknowledge preparation pause');
}

export async function verifyResumedPreparation(dataRoot, snapshot, listPids) {
  assert(
    fs
      .readFileSync(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8')
      .includes('translation-worker-preflight-visible\n'),
    'Native panel did not restore accepted worker preparation after remount',
  );
  if (snapshot.workerPause) {
    assert(
      fs
        .readFileSync(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8')
        .includes('translation-worker-preflight-paused\n'),
      'Native panel did not acknowledge worker-preflight pause',
    );
  }
  const deadline = Date.now() + 10_000;
  while ([...listPids()].some((pid) => !snapshot.baselinePids.has(pid))) {
    assert(Date.now() < deadline, 'Resumed model child was not released');
    await delay(POLL_MS);
  }
  const host = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  const translate = new DatabaseSync(path.join(dataRoot, 'auralis-translate.sqlite'), {
    readOnly: true,
  });
  try {
    const runId = snapshot.paused.run.run_id;
    const attempts = translate.prepare('SELECT * FROM run_attempts WHERE run_id = ?').all(runId);
    const jobs = host
      .prepare(
        'SELECT j.id, j.status, t.ended_at FROM jobs j JOIN translation_host_jobs t ON t.job_id = j.id WHERE t.run_id = ?',
      )
      .all(runId);
    assert.equal(attempts.length, 1);
    assert.equal(jobs.length, snapshot.workerPause ? 2 : 1);
    const completed = jobs.filter((job) => job.status === 'completed');
    assert.equal(completed.length, 1);
    assert(completed[0].ended_at && attempts[0].ended_at);
    assert.equal(attempts[0].host_job_id, completed[0].id);
    if (snapshot.workerPause) {
      const cancelled = jobs.filter((job) => job.status === 'cancelled');
      assert.equal(cancelled.length, 1);
      assert.equal(cancelled[0].id, snapshot.workerPause.jobId);
      assert(cancelled[0].ended_at);
      assert.notEqual(cancelled[0].id, completed[0].id);
    }
    const result = translate.prepare('SELECT * FROM results WHERE run_id = ?').get(runId);
    assert(result);
    process.stdout.write(
      `${JSON.stringify({
        scenario: snapshot.workerPause
          ? 'native_worker_preflight_pause'
          : 'native_preparation_pause',
        format: snapshot.paused.link.source_format,
        run_id: runId,
        translation_id: snapshot.paused.link.translation_id,
        observed_child_pids: snapshot.managedPids,
        acknowledgement_ms: snapshot.acknowledgementMs,
        counts_at_pause: snapshot.paused.counts,
        source_sha256: snapshot.paused.originalSha256,
        resumed_job_id: completed[0].id,
        closed_attempts: attempts.length,
        output_sha256: result.output_sha256,
        child_absent_at_pause_verification: true,
        worker_preflight_ui_restored: true,
        ...(snapshot.workerPause ? { worker_preflight_pause: snapshot.workerPause } : {}),
      })}\n`,
    );
  } finally {
    translate.close();
    host.close();
  }
}

export function assertDesktopAlive(app) {
  assert(
    app.child.exitCode === null && app.child.signalCode === null,
    'Desktop exited during preparation pause',
  );
}

export function assertModelParent(pid, appPid) {
  assert(Number.isInteger(pid) && Number.isInteger(appPid));
  const result = childProcess.spawnSync(
    'powershell.exe',
    [
      '-NoProfile',
      '-NonInteractive',
      '-Command',
      `Get-CimInstance Win32_Process -Filter "ProcessId = ${pid}" | Select-Object ProcessId,ParentProcessId | ConvertTo-Json -Compress`,
    ],
    { encoding: 'utf8', windowsHide: true },
  );
  assert.equal(result.status, 0, 'Could not observe model process ownership');
  const process = JSON.parse(result.stdout.trim());
  assert.equal(process.ProcessId, pid);
  assert.equal(process.ParentProcessId, appPid, 'Observed model does not belong to this desktop');
}
