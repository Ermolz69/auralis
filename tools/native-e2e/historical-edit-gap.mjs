import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { verifyHistoryEdits } from './translation-history-edits.mjs';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;
const TIMEOUT_MS = 600_000;
const POLL_MS = 100;

export async function waitForHistoricalEditGap(dataRoot, markerPath, app) {
  const deadline = Date.now() + TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null)
      throw new Error('Native desktop exited before historical commit interruption');
    const marker = await readIfPresent(markerPath);
    const trace = await readIfPresent(path.join(dataRoot, 'native-e2e-bootstrap.txt'));
    if (trace?.includes('scenario-failed:'))
      throw new Error('Native historical gap scenario failed');
    if (marker && trace?.includes('translation-history-gap-choice\n')) {
      const parts = marker.trim().split(':');
      if (parts.length !== 2 || !parts.every((part) => UUID.test(part)))
        throw new Error('Historical commit marker has invalid identifiers');
      const expected = { runId: parts[0], resultId: parts[1] };
      return { ...expected, snapshot: await inspectHistoricalGap(dataRoot, expected, true) };
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_MS));
  }
  throw new Error('Timed out waiting for historical core commit and newer UI choice');
}

export async function verifyHistoricalEditGap(dataRoot, expected, beforeRecovery) {
  const snapshot = await inspectHistoricalGap(dataRoot, expected, beforeRecovery);
  if (JSON.stringify(snapshot) !== JSON.stringify(expected.snapshot))
    throw new Error(
      'Historical interruption/recovery changed committed edits, files or model checkpoints',
    );
  process.stdout.write(
    `Native historical gap verified: ${JSON.stringify({ ...expected, beforeRecovery })}\n`,
  );
}

async function inspectHistoricalGap(dataRoot, expected, beforeRecovery) {
  const host = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  const translate = new DatabaseSync(path.join(dataRoot, 'auralis-translate.sqlite'), {
    readOnly: true,
  });
  try {
    const results = translate
      .prepare('SELECT * FROM results WHERE run_id = ? ORDER BY revision')
      .all(expected.runId);
    if (results.length !== 3 || results[2].result_id !== expected.resultId)
      throw new Error('Historical core gap lost its three immutable results');
    const link = host
      .prepare('SELECT * FROM project_translations WHERE selected_result_id = ?')
      .get(results[0].result_id);
    if (!link || link.revision !== 4 || link.active_run_id !== null)
      throw new Error('Historical core gap lost the newer explicit model choice');
    const publications = host
      .prepare(
        'SELECT p.*, a.location_value FROM translation_publications p JOIN artifacts a ON a.id = p.artifact_id WHERE p.translation_id = ? ORDER BY p.result_revision',
      )
      .all(link.translation_id);
    if (
      publications.length !== (beforeRecovery ? 2 : 3) ||
      publications.some((item) => item.state !== 'ready')
    )
      throw new Error('Historical core gap has the wrong publication boundary');
    if (link.output_artifact_id !== publications[0].artifact_id)
      throw new Error('Historical core gap changed the selected artifact');
    const edits = verifyHistoryEdits(
      host,
      translate,
      link,
      beforeRecovery
        ? [
            ...publications,
            { result_id: expected.resultId, run_id: expected.runId, expected_link_revision: 3 },
          ]
        : publications,
    );
    const checkpoints = translate
      .prepare('SELECT * FROM block_checkpoints WHERE run_id = ? ORDER BY block_index')
      .all(expected.runId);
    const attempts = translate
      .prepare('SELECT * FROM run_attempts WHERE run_id = ? ORDER BY attempt_id')
      .all(expected.runId);
    const jobs = host
      .prepare('SELECT * FROM translation_host_jobs WHERE run_id = ? ORDER BY job_id')
      .all(expected.runId);
    if (checkpoints.length !== 2 || attempts.length !== 1 || jobs.length !== 1 || !jobs[0].ended_at)
      throw new Error('Historical core gap changed model attempts or committed checkpoints');
    const original = host
      .prepare('SELECT location_value FROM artifacts WHERE id = ?')
      .get(link.source_artifact_id);
    const root = path.resolve(dataRoot, 'projects');
    const files = [];
    for (const item of [original, ...publications.slice(0, 2)]) {
      const file = path.resolve(root, ...item.location_value.split('/'));
      if (!file.startsWith(`${root}${path.sep}`))
        throw new Error('Historical gap artifact escaped managed storage');
      files.push(
        crypto
          .createHash('sha256')
          .update(await fsp.readFile(file))
          .digest('hex'),
      );
    }
    if (
      files[0] !== link.source_sha256 ||
      files[1] !== results[0].output_sha256 ||
      files[2] !== results[1].output_sha256
    )
      throw new Error('Historical gap changed an existing source/output file');
    return { link, results, edits, checkpoints, attempts, jobs, files };
  } finally {
    translate.close();
    host.close();
  }
}

async function readIfPresent(file) {
  try {
    return await fsp.readFile(file, 'utf8');
  } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
}
