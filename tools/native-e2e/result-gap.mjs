import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

export async function waitForResultCommitGap(dataRoot, markerPath, runId, app, timeoutMs, pollMs) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null) {
      throw new Error(`Native application exited before the result-commit boundary: ${app.child.exitCode}`);
    }
    const marker = await readMarker(markerPath);
    if (marker) {
      const gap = inspectResultCommitGap(dataRoot, runId, marker);
      if (gap) return { kind: 'result-commit', ...gap };
    }
    await new Promise((resolve) => setTimeout(resolve, pollMs));
  }
  throw new Error('Timed out waiting for the committed Translate result before Auralis publication');
}

export function verifyResultCommitGap(dataRoot, runId, expected) {
  const actual = inspectResultCommitGap(dataRoot, runId, expected);
  assert(actual, 'Result-commit boundary changed after native desktop termination');
  assert(
    JSON.stringify(actual.firstBlock) === JSON.stringify(expected.firstBlock),
    'Native result-gap interruption changed the committed first block',
  );
}

async function readMarker(markerPath) {
  let value;
  try {
    value = (await fsp.readFile(markerPath, 'utf8')).trim();
  } catch (error) {
    if (error?.code === 'ENOENT') return null;
    throw error;
  }
  const parts = value.split(':');
  if (parts.length !== 2 || !parts.every((part) => UUID.test(part))) return null;
  return { runId: parts[0], resultId: parts[1] };
}

function inspectResultCommitGap(dataRoot, runId, marker) {
  const hostPath = path.join(dataRoot, 'auralis.sqlite');
  const translatePath = path.join(dataRoot, 'auralis-translate.sqlite');
  if (!fs.existsSync(hostPath) || !fs.existsSync(translatePath)) return null;
  let host;
  let translate;
  try {
    host = new DatabaseSync(hostPath, { readOnly: true });
    translate = new DatabaseSync(translatePath, { readOnly: true });
    const link = host
      .prepare('SELECT pt.translation_id, pt.active_run_id, pt.selected_result_id FROM project_translations pt JOIN projects p ON p.id = pt.project_id WHERE p.title = ?')
      .get(`native-e2e-translation-running:${runId}`);
    if (!link || link.active_run_id !== marker.runId || link.selected_result_id !== null) {
      return null;
    }
    const publication = host
      .prepare('SELECT COUNT(*) AS count FROM translation_publications WHERE translation_id = ? AND result_id = ?')
      .get(link.translation_id, marker.resultId);
    const hostJob = host
      .prepare('SELECT j.status, h.ended_at FROM translation_host_jobs h JOIN jobs j ON j.id = h.job_id WHERE h.run_id = ?')
      .get(marker.runId);
    const run = translate.prepare('SELECT state FROM runs WHERE run_id = ?').get(marker.runId);
    const result = translate
      .prepare('SELECT run_id FROM results WHERE result_id = ?')
      .get(marker.resultId);
    const checkpoints = translate
      .prepare('SELECT COUNT(*) AS count FROM block_checkpoints WHERE run_id = ?')
      .get(marker.runId);
    const firstBlock = translate
      .prepare('SELECT input_fingerprint, accepted_json, committed_at FROM block_checkpoints WHERE run_id = ? AND block_index = 0')
      .get(marker.runId);
    assert(Number(publication.count) === 0, 'Auralis published before the injected result-gap boundary');
    assert(hostJob?.status === 'completed' && hostJob.ended_at, 'Host job was not closed before the result gap');
    assert(run?.state === 'validated', 'Translate result was not durably validated');
    assert(result?.run_id === marker.runId, 'Committed result belongs to another run');
    assert(Number(checkpoints.count) === 2 && firstBlock, 'Result gap lost accepted model blocks');
    return { runId: marker.runId, resultId: marker.resultId, firstBlock };
  } catch (error) {
    const message = String(error);
    if (message.includes('locked') || message.includes('busy') || message.includes('no such table')) {
      return null;
    }
    throw error;
  } finally {
    translate?.close();
    host?.close();
  }
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
