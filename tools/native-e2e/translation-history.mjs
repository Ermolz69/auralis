import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

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

export async function verifyTranslationHistory(dataRoot, scenarioId, previous = null) {
  const trace = await fsp.readFile(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8');
  const markers = [
    'translation-history-manual-ready',
    'translation-history-preview',
    'translation-history-selected',
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
    if (!link || link.active_run_id !== null)
      throw new Error('History project lost its completed link');
    const publications = host
      .prepare(
        'SELECT p.*, a.location_value, a.size_bytes, a.project_id, a.state AS artifact_state FROM translation_publications p JOIN artifacts a ON a.id = p.artifact_id WHERE p.translation_id = ? ORDER BY p.rowid',
      )
      .all(link.translation_id);
    if (
      publications.length !== 2 ||
      link.selected_result_id !== publications[0].result_id ||
      link.output_artifact_id !== publications[0].artifact_id
    )
      throw new Error('Historical model result is not the project attachment');
    const outputs = [];
    for (const publication of publications) {
      if (
        publication.state !== 'ready' ||
        publication.artifact_state !== 'ready' ||
        publication.project_id !== link.project_id ||
        publication.source_sha256 !== link.source_sha256
      )
        throw new Error('History includes a non-ready or foreign artifact');
      const result = translate
        .prepare('SELECT * FROM results WHERE result_id = ?')
        .get(publication.result_id);
      const bytes = await fsp.readFile(managedPath(dataRoot, publication.location_value));
      if (
        !result ||
        result.run_id !== publication.run_id ||
        result.revision !== publication.result_revision ||
        result.output_sha256 !== hash(bytes) ||
        Number(publication.size_bytes) !== bytes.length
      )
        throw new Error('Immutable result or file differs across databases');
      if (!bytes.toString('utf8').includes('00:00:01,000 --> 00:00:02,000'))
        throw new Error('Historical output lost subtitle timing');
      outputs.push({
        resultId: result.result_id,
        revision: result.revision,
        artifactId: publication.artifact_id,
        sha256: result.output_sha256,
      });
    }
    if (
      outputs[0].revision !== 1 ||
      outputs[1].revision !== 2 ||
      outputs[0].sha256 === outputs[1].sha256
    )
      throw new Error('Manual result did not create a distinct later version');
    const manual = await fsp.readFile(
      managedPath(dataRoot, publications[1].location_value),
      'utf8',
    );
    if (!manual.includes('Привет, мир.'))
      throw new Error('Actual native editor text is absent from the later output');
    const attempts = translate
      .prepare('SELECT COUNT(*) AS count FROM run_attempts WHERE run_id = ?')
      .get(publications[0].run_id);
    if (Number(attempts.count) !== 1 || publications[1].run_id !== publications[0].run_id)
      throw new Error('Reopening history unexpectedly started inference');
    const observed = {
      translationId: link.translation_id,
      runId: publications[0].run_id,
      selectedResultId: link.selected_result_id,
      linkRevision: link.revision,
      outputs,
    };
    if (previous && JSON.stringify(previous) !== JSON.stringify(observed))
      throw new Error('Restart changed history selection, revision or immutable outputs');
    process.stdout.write(
      `Native translation history verified: ${JSON.stringify({ ...observed, reopened: Boolean(previous), markers })}\n`,
    );
    return observed;
  } finally {
    translate.close();
    host.close();
  }
}

function managedPath(dataRoot, key) {
  const root = path.resolve(dataRoot, 'projects');
  const resolved = path.resolve(root, ...key.split('/'));
  if (!resolved.startsWith(`${root}${path.sep}`))
    throw new Error('History artifact escaped the managed root');
  return resolved;
}
function hash(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}
