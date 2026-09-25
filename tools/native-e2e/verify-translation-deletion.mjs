import crypto from 'node:crypto';
import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';

const CLEANUP_TIMEOUT_MS = 120_000;
const POLL_INTERVAL_MS = 100;

export async function verifyTranslationDeletion(dataRoot, sourcePath, expectedSourceSha256, baselineModelPids, listModelProcessIds) {
  const hostPath = path.join(dataRoot, 'auralis.sqlite');
  const translatePath = path.join(dataRoot, 'auralis-translate.sqlite');
  let identity;
  const deadline = Date.now() + CLEANUP_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const host = new DatabaseSync(hostPath, { readOnly: true });
    try {
      const rows = host
        .prepare("SELECT status, payload_json FROM outbox_messages WHERE kind = 'delete_project_translation'")
        .all();
      if (rows.length > 1) throw new Error('Native deletion created more than one Translate cleanup intent');
      if (rows.length === 1) {
        const payload = JSON.parse(rows[0].payload_json);
        identity = { projectId: payload.project_id, translationId: payload.translation_id };
        const directory = host
          .prepare("SELECT status FROM outbox_messages WHERE kind = 'delete_project_artifact_dir' AND aggregate_id = ?")
          .get(identity.projectId);
        if (rows[0].status === 'done' && directory?.status === 'done') break;
        if (rows[0].status === 'dead') throw new Error('Translate cleanup was dead-lettered');
      }
    } finally {
      host.close();
    }
    await delay(POLL_INTERVAL_MS);
  }
  if (!identity || Date.now() >= deadline) throw new Error('Native project deletion did not finish Translate cleanup');

  const host = new DatabaseSync(hostPath, { readOnly: true });
  const translate = new DatabaseSync(translatePath, { readOnly: true });
  try {
    assertCount(host, 'SELECT COUNT(*) AS count FROM projects WHERE id = ?', identity.projectId, 0);
    assertCount(host, 'SELECT COUNT(*) AS count FROM project_translations WHERE translation_id = ?', identity.translationId, 0);
    assertCount(host, 'SELECT COUNT(*) AS count FROM translation_publications WHERE translation_id = ?', identity.translationId, 0);
    assertCount(host, 'SELECT COUNT(*) AS count FROM jobs WHERE project_id = ?', identity.projectId, 0);
    assertCount(host, 'SELECT COUNT(*) AS count FROM artifacts WHERE project_id = ?', identity.projectId, 0);
    const tombstone = translate
      .prepare('SELECT project_id FROM deleted_translations WHERE translation_id = ?')
      .get(identity.translationId);
    if (tombstone?.project_id !== identity.projectId) throw new Error('Translate deletion tombstone is missing');
    assertCount(translate, 'SELECT COUNT(*) AS count FROM translations WHERE translation_id = ?', identity.translationId, 0);
    assertCount(translate, 'SELECT COUNT(*) AS count FROM runs WHERE translation_id = ?', identity.translationId, 0);
    const remainingOutput = host
      .prepare("SELECT COUNT(*) AS count FROM outbox_messages WHERE kind = 'finalize_staged_artifact' AND aggregate_id = ? AND status IN ('pending', 'processing', 'failed')")
      .get(identity.projectId);
    if (Number(remainingOutput.count) !== 0) throw new Error('Deleted project kept a publishable artifact');
  } finally {
    translate.close();
    host.close();
  }
  if (fs.existsSync(path.join(dataRoot, 'projects', identity.projectId))) {
    throw new Error('Managed original survived project deletion');
  }
  if (!(await fsp.stat(sourcePath)).isFile()) throw new Error('External original was removed');
  const sourceSha256 = crypto.createHash('sha256').update(await fsp.readFile(sourcePath)).digest('hex');
  if (sourceSha256 !== expectedSourceSha256) throw new Error('External original changed during project deletion');
  for (const pid of listModelProcessIds()) {
    if (!baselineModelPids.has(pid)) throw new Error(`Managed model process survived project deletion: ${pid}`);
  }
  return identity.projectId;
}

function assertCount(database, query, identity, expected) {
  const row = database.prepare(query).get(identity);
  if (Number(row.count) !== expected) {
    throw new Error(`Unexpected deleted-project row count for ${query}: ${row.count}`);
  }
}

function delay(durationMs) {
  return new Promise((resolve) => setTimeout(resolve, durationMs));
}
