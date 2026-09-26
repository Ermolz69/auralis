import fs from 'node:fs';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { setTimeout as delay } from 'node:timers/promises';
import assert from 'node:assert/strict';

const PAUSE_TIMEOUT_MS = 600_000;
const POLL_MS = 100;

export async function releaseVerifiedPause(dataRoot, scenarioId, app, baselinePids, listPids) {
  const deadline = Date.now() + PAUSE_TIMEOUT_MS;
  while (Date.now() < deadline) {
    assert(
      app.child.exitCode === null && app.child.signalCode === null,
      'Desktop exited before durable pause',
    );
    const paused = readPaused(dataRoot, scenarioId);
    if (paused) {
      assert.equal(paused.run.state, 'paused');
      assert.equal(paused.run.pause_requested, 0);
      assert.equal(paused.checkpoints.length, 1);
      assert.equal(paused.results, 0);
      assert.equal(paused.publications, 0);
      assert.equal(paused.outputArtifacts, 0);
      assert.equal(paused.jobs.length, 1);
      assert.equal(paused.jobs[0].status, 'cancelled');
      assert(paused.jobs[0].ended_at, 'Cancelled host association remained active');
      assert.equal(paused.jobs[0].run_id, paused.run.run_id);
      assert.equal(paused.link.active_run_id, paused.run.run_id);
      assert.equal(paused.link.translation_id, paused.run.translation_id);
      assert.equal(paused.link.selected_result_id, null);
      assert.equal(paused.link.output_artifact_id, null);
      assert.equal(paused.attempts.length, 1);
      assert(paused.attempts[0].ended_at, 'Paused Translate attempt remained open');
      assert.equal(paused.attempts[0].host_job_id, paused.jobs[0].id);
      if ([...listPids()].some((pid) => !baselinePids.has(pid))) {
        await delay(POLL_MS);
        continue;
      }
      const host = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'));
      try {
        const update = host
          .prepare(
            'UPDATE projects SET title = ?, revision = revision + 1, updated_at = ? WHERE id = ? AND title = ?',
          )
          .run(
            `native-e2e-translation-resume:${scenarioId}`,
            new Date().toISOString(),
            paused.project.id,
            paused.project.title,
          );
        assert.equal(
          Number(update.changes),
          1,
          'Paused scenario release lost its project revision',
        );
      } finally {
        host.close();
      }
      const firstBlock = paused.checkpoints[0];
      return {
        kind: 'pause',
        runId: paused.run.run_id,
        paused,
        firstBlock: {
          input_fingerprint: firstBlock.input_fingerprint,
          accepted_json: firstBlock.accepted_json,
          committed_at: firstBlock.committed_at,
        },
      };
    }
    await delay(POLL_MS);
  }
  throw new Error(
    'Native scenario did not pause, close its host attempt and release the managed process',
  );
}

export function verifyResumedPause(dataRoot, snapshot) {
  const host = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  const translate = new DatabaseSync(path.join(dataRoot, 'auralis-translate.sqlite'), {
    readOnly: true,
  });
  try {
    const first = translate
      .prepare('SELECT * FROM block_checkpoints WHERE run_id = ? AND block_index = 0')
      .get(snapshot.runId);
    assert.deepEqual(
      first,
      snapshot.paused.checkpoints[0],
      'Resume replaced saved checkpoint fields',
    );
    const jobs = host
      .prepare(
        'SELECT j.id, j.status FROM jobs j JOIN translation_host_jobs t ON t.job_id = j.id WHERE t.run_id = ? ORDER BY j.created_at, j.id',
      )
      .all(snapshot.runId);
    assert.equal(jobs.length, 2);
    assert.equal(jobs.filter((job) => job.status === 'cancelled').length, 1);
    assert.equal(jobs.filter((job) => job.status === 'completed').length, 1);
    assert(jobs.some((job) => job.id === snapshot.paused.jobs[0].id && job.status === 'cancelled'));
    const attempts = translate
      .prepare('SELECT * FROM run_attempts WHERE run_id = ?')
      .all(snapshot.runId);
    assert.equal(attempts.length, 2);
    assert(attempts.every((attempt) => attempt.ended_at !== null));
    assert.equal(new Set(attempts.map((attempt) => attempt.host_job_id)).size, 2);
    process.stdout.write(
      `${JSON.stringify({
        scenario: 'native_translation_pause',
        format: snapshot.paused.link.source_format,
        run_id: snapshot.runId,
        first_job_id: snapshot.paused.jobs[0].id,
        resumed_job_id: jobs.find((job) => job.status === 'completed').id,
        checkpoint_retained: true,
        closed_attempts: attempts.length,
        partial_output_artifacts_at_pause: snapshot.paused.outputArtifacts,
      })}\n`,
    );
  } finally {
    translate.close();
    host.close();
  }
}

function readPaused(dataRoot, scenarioId) {
  const hostPath = path.join(dataRoot, 'auralis.sqlite');
  const translatePath = path.join(dataRoot, 'auralis-translate.sqlite');
  if (!fs.existsSync(hostPath) || !fs.existsSync(translatePath)) return null;
  const host = new DatabaseSync(hostPath, { readOnly: true });
  const translate = new DatabaseSync(translatePath, { readOnly: true });
  try {
    const project = host
      .prepare('SELECT id, title FROM projects WHERE title = ?')
      .get(`native-e2e-translation-paused:${scenarioId}`);
    if (!project) return null;
    const link = host
      .prepare('SELECT * FROM project_translations WHERE project_id = ?')
      .get(project.id);
    const run = translate.prepare('SELECT * FROM runs WHERE run_id = ?').get(link.active_run_id);
    const jobs = host
      .prepare(
        'SELECT j.id, j.status, t.run_id, t.ended_at FROM jobs j JOIN translation_host_jobs t ON t.job_id = j.id WHERE t.run_id = ?',
      )
      .all(run.run_id);
    if (jobs.some((job) => job.ended_at === null)) return null;
    return {
      project,
      link,
      run,
      jobs,
      checkpoints: translate
        .prepare('SELECT * FROM block_checkpoints WHERE run_id = ? ORDER BY block_index')
        .all(run.run_id),
      attempts: translate.prepare('SELECT * FROM run_attempts WHERE run_id = ?').all(run.run_id),
      results: Number(
        translate.prepare('SELECT COUNT(*) AS count FROM results WHERE run_id = ?').get(run.run_id)
          .count,
      ),
      publications: Number(
        host
          .prepare('SELECT COUNT(*) AS count FROM translation_publications WHERE run_id = ?')
          .get(run.run_id).count,
      ),
      outputArtifacts: Number(
        host
          .prepare(
            "SELECT COUNT(*) AS count FROM artifacts WHERE project_id = ? AND kind = 'TranslatedTranscript'",
          )
          .get(project.id).count,
      ),
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
    translate.close();
    host.close();
  }
}
