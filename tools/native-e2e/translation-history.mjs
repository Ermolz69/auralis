import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { verifyHistoryResults } from './translation-history-results.mjs';
import { verifyHistoryEdits } from './translation-history-edits.mjs';

const REOPEN_TIMEOUT_MS = 60_000;
const POLL_INTERVAL_MS = 100;

export async function waitForTranslationHistoryReopened(dataRoot, app) {
  const deadline = Date.now() + REOPEN_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const trace = await fsp.readFile(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8');
    if (trace.includes('translation-history-reopened\n')) return;
    if (trace.includes('scenario-failed:'))
      throw new Error('Reopened native history reported scenario failure');
    if (app.child.exitCode !== null || app.child.signalCode !== null)
      throw new Error('Native history exited before reopening');
    await new Promise((resolve) => setTimeout(resolve, POLL_INTERVAL_MS));
  }
  throw new Error('Timed out waiting for native historical selection after restart');
}

export async function verifyTranslationHistory(dataRoot, scenarioId, previous = null, gap = false) {
  const trace = await fsp.readFile(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8');
  const markers = [
    'translation-history-manual-ready',
    'translation-history-preview',
    ...(gap
      ? ['translation-history-gap-choice', 'translation-history-gap-recovered']
      : ['translation-history-branch-ready', 'translation-history-selected']),
  ];
  if (previous) markers.push('translation-history-reopened');
  let offset = -1;
  for (const marker of markers) {
    const next = trace.indexOf(`${marker}\n`);
    if (next <= offset) throw new Error(`Missing or unordered history marker: ${marker}`);
    offset = next;
  }
  const host = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  const translate = new DatabaseSync(path.join(dataRoot, 'auralis-translate.sqlite'), {
    readOnly: true,
  });
  try {
    const link = host
      .prepare(
        'SELECT t.* FROM project_translations t JOIN projects p ON p.id = t.project_id WHERE p.title = ?',
      )
      .get(`native-e2e-translation-complete:${scenarioId}`);
    if (!link || link.active_run_id !== null || link.revision !== (gap ? 4 : 5))
      throw new Error('History project lost its completed link or exact choice revision');
    const publications = host
      .prepare(
        'SELECT p.*, a.location_value, a.size_bytes, a.project_id, a.state AS artifact_state FROM translation_publications p JOIN artifacts a ON a.id = p.artifact_id WHERE p.translation_id = ? ORDER BY p.result_revision',
      )
      .all(link.translation_id);
    if (
      publications.length !== 3 ||
      link.selected_result_id !== publications[gap ? 0 : 1].result_id ||
      link.output_artifact_id !== publications[gap ? 0 : 1].artifact_id
    )
      throw new Error('Explicit historical choice is not the retained project attachment');
    for (const publication of publications) {
      if (
        publication.state !== 'ready' ||
        publication.artifact_state !== 'ready' ||
        publication.project_id !== link.project_id ||
        publication.source_sha256 !== link.source_sha256 ||
        publication.needs_review !== 1 ||
        publication.run_id !== publications[0].run_id
      )
        throw new Error('History includes a foreign, nonready or mismatched publication');
    }
    const verified = await verifyHistoryResults(dataRoot, translate, publications);
    const edits = verifyHistoryEdits(host, translate, link, publications);
    const attempts = translate
      .prepare('SELECT COUNT(*) AS count FROM run_attempts WHERE run_id = ?')
      .get(publications[0].run_id);
    if (Number(attempts.count) !== 1)
      throw new Error('History editing or reopening unexpectedly started inference');
    const checkpoints = translate
      .prepare(
        'SELECT block_index, input_fingerprint, accepted_json, committed_at FROM block_checkpoints WHERE run_id = ? ORDER BY block_index',
      )
      .all(publications[0].run_id);
    if (checkpoints.length !== 2) throw new Error('Two-cue model checkpoints were lost');
    const observed = {
      translationId: link.translation_id,
      runId: publications[0].run_id,
      selectedResultId: link.selected_result_id,
      linkRevision: link.revision,
      ...verified,
      edits,
      checkpoints,
    };
    if (previous && JSON.stringify(previous) !== JSON.stringify(observed))
      throw new Error(
        'Restart changed selection, outputs, edit requests, provenance or committed checkpoints',
      );
    process.stdout.write(
      `Native translation history verified: ${JSON.stringify({ ...observed, reopened: Boolean(previous) || gap, markers })}\n`,
    );
    return observed;
  } finally {
    translate.close();
    host.close();
  }
}
