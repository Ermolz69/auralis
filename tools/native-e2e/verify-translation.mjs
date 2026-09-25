import crypto from 'node:crypto';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

export async function verifyTranslationResult(dataRoot, sourcePath, runId, interrupted = null, format = 'srt') {
  const host = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  const translate = new DatabaseSync(path.join(dataRoot, 'auralis-translate.sqlite'), {
    readOnly: true,
  });
  try {
    const project = host
      .prepare('SELECT id FROM projects WHERE title = ?')
      .get(`native-e2e-translation-complete:${runId}`);
    assert(project, 'Native translation project did not complete');
    const link = host
      .prepare('SELECT * FROM project_translations WHERE project_id = ?')
      .get(project.id);
    assert(link, 'Native translation project has no link');
    assert(link.active_run_id === null, 'Completed native translation kept an active run');
    assert(link.selected_result_id && link.output_artifact_id, 'Native result is not selected');
    const publication = host
      .prepare('SELECT * FROM translation_publications WHERE translation_id = ? AND result_id = ?')
      .get(link.translation_id, link.selected_result_id);
    assert(publication?.state === 'ready', 'Native result publication is not ready');
    assert(publication.needs_review === 1, 'Native result lost the review flag');
    const original = host.prepare('SELECT * FROM artifacts WHERE id = ?').get(link.source_artifact_id);
    const output = host.prepare('SELECT * FROM artifacts WHERE id = ?').get(link.output_artifact_id);
    assert(original?.kind === 'OriginalSubtitle' && original.state === 'ready', 'Original artifact is not ready');
    assert(output?.kind === 'TranslatedTranscript' && output.state === 'ready', 'Translated artifact is not ready');
    assert(original.project_id === project.id && output.project_id === project.id, 'Artifact project ownership differs');
    assert(original.id !== output.id && original.location_value !== output.location_value, 'Translated file replaced the original');

    const job = host
      .prepare("SELECT id, status FROM jobs WHERE project_id = ? AND kind = 'translation' AND status = 'completed'")
      .get(project.id);
    assert(job?.status === 'completed', 'Native translation host job did not complete');
    const hostAttempt = host
      .prepare('SELECT run_id, ended_at FROM translation_host_jobs WHERE job_id = ?')
      .get(job.id);
    assert(hostAttempt?.run_id === publication.run_id && hostAttempt.ended_at, 'Native host attempt was not closed');

    assert(link.source_format === format, 'Auralis source format differs from the requested format');
    const sourceRecord = translate
      .prepare('SELECT source_sha256, source_format FROM translations WHERE translation_id = ?')
      .get(link.translation_id);
    assert(sourceRecord?.source_format === format, 'Translate source format differs from Auralis');
    const run = translate.prepare('SELECT state FROM runs WHERE run_id = ?').get(publication.run_id);
    const result = translate
      .prepare('SELECT * FROM results WHERE result_id = ?')
      .get(publication.result_id);
    assert(run?.state === 'validated', 'Translate run did not validate');
    assert(result?.run_id === publication.run_id, 'Selected result belongs to another run');
    assert(result.revision === publication.result_revision, 'Result revision differs across databases');
    assert(result.review_state === 'needs_review', 'Translate result lost review state');
    if (interrupted) {
      const checkpoints = translate
        .prepare('SELECT COUNT(*) AS count FROM block_checkpoints WHERE run_id = ?')
        .get(publication.run_id);
      const attempts = translate
        .prepare('SELECT COUNT(*) AS count FROM run_attempts WHERE run_id = ?')
        .get(publication.run_id);
      const jobs = host
        .prepare("SELECT COUNT(*) AS count FROM jobs WHERE project_id = ? AND kind = 'translation'")
        .get(project.id);
      assert(Number(checkpoints.count) === 2, 'Recovered run did not retain both blocks');
      const expectedAttempts = interrupted.kind === 'result-commit' ? 1 : 2;
      assert(Number(attempts.count) === expectedAttempts, 'Recovered run has the wrong Translate attempt count');
      assert(Number(jobs.count) === expectedAttempts, 'Recovered run has the wrong host job count');
      if (interrupted.kind === 'result-commit') {
        assert(result.result_id === interrupted.resultId, 'Recovery selected a different Translate result');
      }
      const firstBlock = translate
        .prepare('SELECT input_fingerprint, accepted_json, committed_at FROM block_checkpoints WHERE run_id = ? AND block_index = 0')
        .get(publication.run_id);
      assert(
        JSON.stringify(firstBlock) === JSON.stringify(interrupted.firstBlock),
        'Resume replaced the first committed block',
      );
    }

    const sourceBytes = await fsp.readFile(sourcePath);
    const originalBytes = await fsp.readFile(artifactPath(dataRoot, original.location_value));
    const outputBytes = await fsp.readFile(artifactPath(dataRoot, output.location_value));
    assert(sourceBytes.equals(originalBytes), 'Managed original differs from the external subtitle');
    assert(sha256(sourceBytes) === link.source_sha256, 'Auralis source hash differs');
    assert(sha256(sourceBytes) === sourceRecord?.source_sha256, 'Translate source hash differs');
    assert(sha256(outputBytes) === result.output_sha256, 'Published output differs from Translate result');
    assert(!outputBytes.equals(sourceBytes), 'Native translation output equals its Chinese source');
    if (format === 'vtt') {
      const protectedPrefix = 'WEBVTT\r\n\r\nNOTE provenance\r\nnative fixture\r\n\r\n00:00:01.000 --> 00:00:02.000\r\n';
      assert(sourceBytes.toString('utf8').startsWith(protectedPrefix), 'Native WebVTT fixture changed');
      assert(outputBytes.toString('utf8').startsWith(protectedPrefix), 'Output changed protected WebVTT bytes');
      assert(outputBytes.toString('utf8').endsWith('\r\n'), 'Output lost the WebVTT terminal CRLF');
    } else {
      assert(outputBytes.toString('utf8').includes('00:00:01,000 --> 00:00:02,000'), 'Output lost the cue timing');
    }
    if (interrupted) {
      assert(outputBytes.toString('utf8').includes('00:00:03,000 --> 00:00:04,000'), 'Recovered output lost the second cue timing');
    }

    const finalized = host
      .prepare("SELECT COUNT(*) AS count FROM outbox_messages WHERE aggregate_id = ? AND kind = 'finalize_staged_artifact' AND status = 'done'")
      .get(project.id);
    assert(Number(finalized.count) === 2, 'Original and output finalization were not both acknowledged');
  } finally {
    translate.close();
    host.close();
  }
}

function artifactPath(dataRoot, storageKey) {
  const root = path.resolve(dataRoot, 'projects');
  const resolved = path.resolve(root, ...storageKey.split('/'));
  assert(resolved.startsWith(`${root}${path.sep}`), 'Translation artifact escaped the managed root');
  return resolved;
}

function sha256(bytes) {
  return crypto.createHash('sha256').update(bytes).digest('hex');
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
