import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { requestDigest } from './translation-history-edits.mjs';

const REOPEN_TIMEOUT_MS = 60_000;
const POLL_MS = 100;
const LATER_LINE = 'Поздняя правка второй реплики.';
const BRANCH_LINE = 'Правка первой реплики из исходной версии.';

export async function releaseHistoricalJournalRetry(dataRoot, expected, app, releasePath) {
  const deadline = Date.now() + REOPEN_TIMEOUT_MS;
  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null)
      throw new Error('Native journal restart exited before exact retry');
    const trace = await fsp.readFile(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8');
    if (trace.includes('scenario-failed:')) throw new Error('Native journal restart failed');
    if (trace.includes('translation-history-journal-reopened\n')) {
      const snapshot = await inspectHistoricalJournalGap(dataRoot, expected, true);
      if (JSON.stringify(snapshot) !== JSON.stringify(expected.snapshot))
        throw new Error('Startup synthesized a journal-only result or changed durable state');
      process.stdout.write(`Native journal-only startup verified: ${JSON.stringify(expected)}\n`);
      await fsp.writeFile(releasePath, 'retry');
      return;
    }
    await new Promise((resolve) => setTimeout(resolve, POLL_MS));
  }
  throw new Error('Timed out verifying journal-only startup before request retry');
}

export async function inspectHistoricalJournalGap(dataRoot, expected, beforeRecovery) {
  const host = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  const translate = new DatabaseSync(path.join(dataRoot, 'auralis-translate.sqlite'), {
    readOnly: true,
  });
  try {
    const results = translate
      .prepare('SELECT * FROM results WHERE run_id = ? ORDER BY revision')
      .all(expected.runId);
    if (
      results.length !== (beforeRecovery ? 2 : 3) ||
      (beforeRecovery && results.some((item) => item.result_id === expected.resultId)) ||
      (!beforeRecovery && results[2].result_id !== expected.resultId)
    )
      throw new Error('Journal-only boundary has an unexpected core result');
    const link = host
      .prepare('SELECT * FROM project_translations WHERE selected_result_id = ?')
      .get(results[0].result_id);
    if (!link || link.active_run_id !== null || link.revision !== 4)
      throw new Error('Journal-only recovery overwrote the newer explicit choice');
    const publications = host
      .prepare(
        'SELECT p.*, a.location_value FROM translation_publications p JOIN artifacts a ON a.id = p.artifact_id WHERE p.translation_id = ? ORDER BY p.result_revision',
      )
      .all(link.translation_id);
    if (
      publications.length !== (beforeRecovery ? 2 : 3) ||
      publications.some((item) => item.state !== 'ready') ||
      link.output_artifact_id !== publications[0].artifact_id
    )
      throw new Error('Journal-only boundary has an unexpected publication or attachment');
    const input = expected.input;
    if (
      input.projectId !== link.project_id ||
      input.translationId !== link.translation_id ||
      input.resultId !== expected.resultId ||
      input.baseResultId !== results[0].result_id ||
      input.expectedHeadResultId !== results[1].result_id ||
      input.expectedLinkRevision !== 3 ||
      input.segmentId !== 1 ||
      input.lines.length !== 1 ||
      input.lines[0] !== BRANCH_LINE
    )
      throw new Error('Captured request differs from the actual authored branch and observations');
    const intents = host
      .prepare(
        'SELECT * FROM translation_edit_intents WHERE translation_id = ? ORDER BY expected_link_revision',
      )
      .all(link.translation_id);
    if (intents.length !== 2) throw new Error('Exact retry duplicated or lost a host intent');
    for (const [index, intent] of intents.entries()) {
      if (
        intent.project_id !== link.project_id ||
        intent.translation_id !== link.translation_id ||
        intent.run_id !== expected.runId ||
        intent.base_result_id !== results[0].result_id ||
        intent.expected_head_result_id !== results[index].result_id ||
        intent.expected_link_revision !== index + 2 ||
        intent.result_id !== (index === 0 ? results[1].result_id : expected.resultId) ||
        intent.request_sha256 !==
          requestDigest(intent, index === 0 ? 2 : 1, index === 0 ? [LATER_LINE] : input.lines)
      )
        throw new Error('Journal-only request digest or immutable metadata differs');
    }
    const pendingOrigin = translate
      .prepare('SELECT * FROM result_edit_provenance WHERE result_id = ?')
      .get(expected.resultId);
    const pendingSelections = translate
      .prepare('SELECT segment_id FROM result_edit_selections WHERE result_id = ?')
      .all(expected.resultId);
    if (beforeRecovery && (pendingOrigin || pendingSelections.length !== 0))
      throw new Error(
        'Journal-only request acquired core ancestry or manual selections before retry',
      );
    const origins = translate
      .prepare('SELECT * FROM result_edit_provenance WHERE result_id = ?')
      .all(results[1].result_id);
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
      throw new Error('Journal-only retry replayed inference or lost checkpoints');
    const original = host
      .prepare('SELECT location_value FROM artifacts WHERE id = ?')
      .get(link.source_artifact_id);
    const root = path.resolve(dataRoot, 'projects');
    const files = [];
    for (const item of [original, ...publications.slice(0, 2)]) {
      const file = path.resolve(root, ...item.location_value.split('/'));
      if (!file.startsWith(`${root}${path.sep}`))
        throw new Error('Journal gap file escaped managed storage');
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
      throw new Error('Journal-only recovery changed an existing source/output');
    return {
      link,
      results: results.slice(0, 2),
      intents,
      origins,
      checkpoints,
      attempts,
      jobs,
      files,
      input,
    };
  } finally {
    translate.close();
    host.close();
  }
}
