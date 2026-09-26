import fs from 'node:fs';
import path from 'node:path';
import crypto from 'node:crypto';
import assert from 'node:assert/strict';
import { DatabaseSync } from 'node:sqlite';

export const PREPARATION_RECORD_KINDS = Object.freeze([
  'jobs',
  'associations',
  'attempts',
  'checkpoints',
  'results',
  'publications',
  'outputArtifacts',
]);

export function readPreparationSnapshot(dataRoot, titlePrefix) {
  const hostPath = path.join(dataRoot, 'auralis.sqlite');
  const translatePath = path.join(dataRoot, 'auralis-translate.sqlite');
  if (!fs.existsSync(hostPath) || !fs.existsSync(translatePath)) return null;
  const host = new DatabaseSync(hostPath, { readOnly: true });
  const translate = new DatabaseSync(translatePath, { readOnly: true });
  try {
    const project = host
      .prepare('SELECT id, title FROM projects WHERE title LIKE ?')
      .get(`${titlePrefix}%`);
    if (!project) return null;
    const link = host
      .prepare('SELECT * FROM project_translations WHERE project_id = ?')
      .get(project.id);
    if (!link?.active_run_id) return null;
    const run = translate.prepare('SELECT * FROM runs WHERE run_id = ?').get(link.active_run_id);
    if (!run) return null;
    const original = host
      .prepare('SELECT * FROM artifacts WHERE id = ?')
      .get(link.source_artifact_id);
    const managedRoot = path.resolve(dataRoot, 'projects');
    const originalPath = path.resolve(managedRoot, ...original.location_value.split('/'));
    assert(originalPath.startsWith(`${managedRoot}${path.sep}`));
    return {
      project,
      link,
      run,
      hostJobs: host
        .prepare(
          'SELECT j.id, j.project_id, j.kind, j.status, t.run_id, t.ended_at FROM jobs j JOIN translation_host_jobs t ON t.job_id = j.id WHERE t.run_id = ?',
        )
        .all(run.run_id),
      originalSha256: crypto
        .createHash('sha256')
        .update(fs.readFileSync(originalPath))
        .digest('hex'),
      counts: {
        jobs: Number(
          host
            .prepare("SELECT COUNT(*) AS n FROM jobs WHERE project_id = ? AND kind = 'translation'")
            .get(project.id).n,
        ),
        associations: Number(
          host
            .prepare('SELECT COUNT(*) AS n FROM translation_host_jobs WHERE run_id = ?')
            .get(run.run_id).n,
        ),
        attempts: Number(
          translate
            .prepare('SELECT COUNT(*) AS n FROM run_attempts WHERE run_id = ?')
            .get(run.run_id).n,
        ),
        checkpoints: Number(
          translate
            .prepare('SELECT COUNT(*) AS n FROM block_checkpoints WHERE run_id = ?')
            .get(run.run_id).n,
        ),
        results: Number(
          translate.prepare('SELECT COUNT(*) AS n FROM results WHERE run_id = ?').get(run.run_id).n,
        ),
        publications: Number(
          host
            .prepare('SELECT COUNT(*) AS n FROM translation_publications WHERE run_id = ?')
            .get(run.run_id).n,
        ),
        outputArtifacts: Number(
          host
            .prepare(
              "SELECT COUNT(*) AS n FROM artifacts WHERE project_id = ? AND kind = 'TranslatedTranscript'",
            )
            .get(project.id).n,
        ),
      },
    };
  } catch (error) {
    if (/locked|busy|no such table/u.test(String(error))) return null;
    throw error;
  } finally {
    translate.close();
    host.close();
  }
}

export function verifyEmptyPreparation(snapshot, expectedState, sourceSha256) {
  verifyPreparationIdentity(snapshot, expectedState, sourceSha256);
  for (const name of PREPARATION_RECORD_KINDS) {
    assert.equal(snapshot.counts[name], 0, `Preparation created ${name} before admission`);
  }
}

export function verifyPreparationIdentity(snapshot, expectedState, sourceSha256) {
  assert.equal(snapshot.run.state, expectedState);
  assert.equal(snapshot.run.pause_requested, 0);
  assert.equal(snapshot.link.active_run_id, snapshot.run.run_id);
  assert.equal(snapshot.link.translation_id, snapshot.run.translation_id);
  assert.equal(snapshot.link.selected_result_id, null);
  assert.equal(snapshot.link.output_artifact_id, null);
  assert.equal(snapshot.originalSha256, sourceSha256);
  assert.equal(snapshot.link.source_sha256, sourceSha256);
}

export function releasePreparationGate(dataRoot, snapshot, title) {
  const host = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'));
  try {
    const update = host
      .prepare(
        'UPDATE projects SET title = ?, revision = revision + 1, updated_at = ? WHERE id = ? AND title = ?',
      )
      .run(title, new Date().toISOString(), snapshot.project.id, snapshot.project.title);
    assert.equal(Number(update.changes), 1, 'Preparation gate lost its project revision');
  } finally {
    host.close();
  }
}
