import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

export async function verifyTranslationEvents(dataRoot, scenarioId) {
  const trace = await fsp.readFile(path.join(dataRoot, 'native-e2e-bootstrap.txt'), 'utf8');
  const markers = [
    'translation-events-listener-ready',
    'translation-events-committed-progress',
    'translation-events-panel-ready',
  ];
  let previous = -1;
  for (const marker of markers) {
    const offset = trace.indexOf(`${marker}\n`);
    if (offset <= previous)
      throw new Error(`Missing or unordered translation event marker: ${marker}`);
    previous = offset;
  }
  const database = new DatabaseSync(path.join(dataRoot, 'auralis.sqlite'), { readOnly: true });
  try {
    const job = database
      .prepare(
        `SELECT jobs.id, jobs.revision, jobs.progress_json, host.run_id
      FROM jobs JOIN projects ON projects.id = jobs.project_id
      JOIN translation_host_jobs AS host ON host.job_id = jobs.id
      WHERE projects.title = ? AND jobs.kind = 'translation' AND jobs.status = 'completed' AND host.ended_at IS NOT NULL`,
      )
      .get(`native-e2e-translation-complete:${scenarioId}`);
    if (!job) throw new Error('Event-observed translation has no committed terminal host job');
    process.stdout.write(
      `Native translation events verified: ${JSON.stringify({ jobId: job.id, runId: job.run_id, revision: job.revision, progress: JSON.parse(job.progress_json), markers })}\n`,
    );
  } finally {
    database.close();
  }
}
