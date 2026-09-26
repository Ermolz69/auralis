import assert from 'node:assert/strict';
import { setTimeout as delay } from 'node:timers/promises';
import { assertDesktopAlive, assertModelParent } from './preparation-pause.mjs';
import { readPreparationSnapshot, releasePreparationGate } from './preparation-snapshot.mjs';
import { verifyWorkerPreflight } from './worker-preflight-snapshot.mjs';

const TIMEOUT_MS = 180_000;
const POLL_MS = 100;

export async function releaseVerifiedWorkerPreflightPause(
  dataRoot,
  scenarioId,
  app,
  initial,
  listPids,
) {
  const before = await waitForBoundary(dataRoot, `native-e2e-worker-preflight:${scenarioId}`, app);
  const jobId = before.hostJobs[0]?.id;
  assert(jobId, 'Accepted worker has no host job');
  verifyWorkerPreflight(before, initial.paused.originalSha256, jobId, 'running');
  assert.equal(before.run.run_id, initial.paused.run.run_id);
  assert.equal(before.link.translation_id, initial.paused.link.translation_id);
  const managedPids = [...listPids()].filter((pid) => !initial.baselinePids.has(pid));
  assert.equal(managedPids.length, 1, 'Accepted worker lost or duplicated its model child');
  assertModelParent(managedPids[0], app.child.pid);
  releasePreparationGate(dataRoot, before, `native-e2e-worker-pause-now:${scenarioId}`);
  const paused = await waitForBoundary(dataRoot, `native-e2e-worker-paused:${scenarioId}:`, app);
  verifyWorkerPreflight(paused, initial.paused.originalSha256, jobId, 'cancelled');
  assert.equal(paused.run.run_id, before.run.run_id);
  assert.equal(paused.link.translation_id, before.link.translation_id);
  assert(
    paused.run.control_revision > before.run.control_revision,
    'Worker pause did not invalidate its captured guard',
  );
  const acknowledgementMs = Number(paused.project.title.split(':').at(-1));
  assert(
    Number.isInteger(acknowledgementMs) && acknowledgementMs > 0 && acknowledgementMs <= 10_000,
  );
  assert(
    [...listPids()].every((pid) => initial.baselinePids.has(pid)),
    'Paused worker model child survived verification',
  );
  releasePreparationGate(dataRoot, paused, `native-e2e-worker-resume:${scenarioId}`);
  return {
    jobId,
    observedChildPids: managedPids,
    acknowledgementMs,
    countsAtPause: paused.counts,
    priorControlRevision: before.run.control_revision,
    pausedControlRevision: paused.run.control_revision,
    childAbsentAtPauseVerification: true,
  };
}

async function waitForBoundary(dataRoot, prefix, app) {
  const deadline = Date.now() + TIMEOUT_MS;
  while (Date.now() < deadline) {
    assertDesktopAlive(app);
    const snapshot = readPreparationSnapshot(dataRoot, prefix);
    if (snapshot) return snapshot;
    await delay(POLL_MS);
  }
  throw new Error(`Native controls missed the worker boundary: ${prefix}`);
}
