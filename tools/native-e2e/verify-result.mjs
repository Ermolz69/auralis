import fs from 'node:fs';
import fsp from 'node:fs/promises';
import path from 'node:path';
import { DatabaseSync } from 'node:sqlite';
import { sha256 } from './fixture.mjs';
import { listModelProcessIds, delay } from './app-lifecycle.mjs';

export const TRANSLATION_RESULT_TIMEOUT_MS = 600_000;
export const POLL_INTERVAL_MS = 100;

export async function waitForTranslationCheckpoint(dataRoot, runId, app, baselineModelPids) {
  const deadline = Date.now() + TRANSLATION_RESULT_TIMEOUT_MS;
  let managedModelPids = [];
  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null) {
      throw new Error(
        `Native application exited before its first translation checkpoint: ${app.child.exitCode}`,
      );
    }
    const interrupted = readTranslationCheckpoint(dataRoot, runId);
    if (interrupted?.runState === 'running' && managedModelPids.length === 0) {
      managedModelPids = [...listModelProcessIds()].filter((pid) => !baselineModelPids.has(pid));
    }
    if (interrupted?.savedBlocks === 1 && interrupted.runState === 'running') {
      return { kind: 'checkpoint', ...interrupted, managedModelPids };
    }
    if (interrupted?.savedBlocks > 1 || interrupted?.runState === 'validated') {
      throw new Error('Native translation completed before the process-kill boundary');
    }
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Timed out waiting for a committed native translation checkpoint');
}

function readTranslationCheckpoint(dataRoot, runId) {
  const hostPath = path.join(dataRoot, 'auralis.sqlite');
  const translatePath = path.join(dataRoot, 'auralis-translate.sqlite');
  if (!fs.existsSync(hostPath) || !fs.existsSync(translatePath)) return null;
  let host;
  let translate;
  try {
    host = new DatabaseSync(hostPath, { readOnly: true });
    translate = new DatabaseSync(translatePath, { readOnly: true });
    const link = host
      .prepare(
        'SELECT pt.translation_id, pt.active_run_id FROM project_translations pt JOIN projects p ON p.id = pt.project_id WHERE p.title = ?',
      )
      .get(`native-e2e-translation-running:${runId}`);
    if (!link?.active_run_id) return null;
    const run = translate
      .prepare('SELECT state FROM runs WHERE run_id = ?')
      .get(link.active_run_id);
    const checkpoint = translate
      .prepare('SELECT COUNT(*) AS count FROM block_checkpoints WHERE run_id = ?')
      .get(link.active_run_id);
    const firstBlock = translate
      .prepare(
        'SELECT input_fingerprint, accepted_json, committed_at FROM block_checkpoints WHERE run_id = ? AND block_index = 0',
      )
      .get(link.active_run_id);
    return {
      translationId: link.translation_id,
      runId: link.active_run_id,
      runState: run?.state,
      savedBlocks: Number(checkpoint.count),
      firstBlock,
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
    translate?.close();
    host?.close();
  }
}

export function verifyInterruptedTranslation(dataRoot, runId, expected) {
  const actual = readTranslationCheckpoint(dataRoot, runId);
  assert(
    actual?.translationId === expected.translationId,
    'Crash changed the project translation link',
  );
  assert(actual.runId === expected.runId, 'Crash changed the unfinished project run');
  assert(
    actual.savedBlocks === 1 && actual.runState === 'running',
    `Crash boundary changed: expected one running checkpoint, observed ${actual.savedBlocks} and ${actual.runState}`,
  );
  assert(
    JSON.stringify(actual.firstBlock) === JSON.stringify(expected.firstBlock),
    'Crash changed the committed first block',
  );
}

export async function waitForResult(databasePath, runId, app, timeoutMs) {
  const deadline = Date.now() + timeoutMs;
  const completedTitle = `native-e2e-complete:${runId}`;
  const failedTitle = `native-e2e-failed:${runId}`;

  while (Date.now() < deadline) {
    if (app.child.exitCode !== null || app.child.signalCode !== null) {
      throw new Error(
        `Native application exited early: code ${app.child.exitCode}, signal ${app.child.signalCode}`,
      );
    }
    const result = tryReadResult(databasePath, completedTitle, failedTitle);
    if (result?.failed) throw new Error('React native E2E runner reported a failure');
    if (result) return result;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error(`Timed out after ${timeoutMs} ms waiting for the native E2E result`);
}

function tryReadResult(databasePath, completedTitle, failedTitle) {
  if (!fs.existsSync(databasePath)) return null;
  let database;
  try {
    database = new DatabaseSync(databasePath, { readOnly: true });
    const failedProject = database
      .prepare('SELECT id FROM projects WHERE title = ?')
      .get(failedTitle);
    if (failedProject) return { failed: true };

    const project = database
      .prepare(
        'SELECT id, title, status, source_json, metadata_json, revision FROM projects WHERE title = ?',
      )
      .get(completedTitle);
    const youtubeProject = database
      .prepare(
        'SELECT id, title, status, source_json, metadata_json, revision FROM projects WHERE title = ?',
      )
      .get(`native-e2e-ytdlp-complete:${runIdFromTitle(completedTitle)}`);
    if (!project || !youtubeProject) return null;

    const artifacts = database
      .prepare(
        "SELECT id, project_id, kind, location_kind, location_value, size_bytes, state, ready_at FROM artifacts WHERE project_id = ? AND kind = 'SourceVideo'",
      )
      .all(project.id);
    const youtubeArtifacts = database
      .prepare(
        "SELECT id, project_id, kind, location_kind, location_value, size_bytes, state, ready_at FROM artifacts WHERE project_id = ? AND kind = 'DownloadedVideo'",
      )
      .all(youtubeProject.id);
    const jobs = database
      .prepare(
        'SELECT id, project_id, status, progress_json, finished_at FROM jobs WHERE project_id = ? ORDER BY created_at',
      )
      .all(project.id);
    const outbox = database
      .prepare(
        'SELECT kind, payload_json, status, attempts, last_error, aggregate_id FROM outbox_messages ORDER BY rowid LIMIT 4',
      )
      .all();
    const pendingYoutubeImports = database
      .prepare('SELECT COUNT(*) AS count FROM youtube_imports')
      .get();
    if (
      jobs.length !== 1 ||
      jobs[0].status !== 'cancelled' ||
      !jobs[0].finished_at ||
      artifacts.length !== 1 ||
      artifacts[0].state !== 'ready' ||
      youtubeArtifacts.length !== 1 ||
      youtubeArtifacts[0].state !== 'ready' ||
      outbox.length !== 4 ||
      outbox.some((record) => record.status !== 'done') ||
      Number(pendingYoutubeImports.count) !== 0
    ) {
      return null;
    }
    return { project, artifacts, jobs, outbox, youtubeProject, youtubeArtifacts };
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
    database?.close();
  }
}

function runIdFromTitle(title) {
  return title.slice('native-e2e-complete:'.length);
}

export async function verifyResult({
  dataRoot,
  sourcePath,
  ytdlpUrl,
  ytdlpRequestCount,
  project,
  artifacts,
  jobs,
  outbox: allOutbox,
  deletedTranslationProjectId,
  youtubeProject,
  youtubeArtifacts,
}) {
  const outbox = deletedTranslationProjectId
    ? allOutbox.filter(
        (record) =>
          record.aggregate_id !== deletedTranslationProjectId &&
          ![
            'delete_storage_key',
            'delete_project_artifact_dir',
            'delete_project_translation',
          ].includes(record.kind),
      )
    : allOutbox;
  assert(project.status === 'Cancelled', `Unexpected project status: ${project.status}`);
  assert(project.revision >= 5, `Expected project revision >= 5, got ${project.revision}`);

  const source = JSON.parse(project.source_json);
  const metadata = JSON.parse(project.metadata_json);
  assert(source.ManagedLocalFile, 'Project source is not a managed local file');
  assert(
    source.ManagedLocalFile.original_filename === 'source.mp4',
    'Original filename was not saved',
  );
  assert(metadata.has_video === true, 'ffprobe metadata did not identify the video stream');
  assert(metadata.width === 320 && metadata.height === 180, 'Unexpected probed video dimensions');

  assert(artifacts.length === 1, `Expected one source artifact, found ${artifacts.length}`);
  const artifact = artifacts[0];
  assert(artifact.project_id === project.id, 'Artifact belongs to a different project');
  assert(artifact.location_kind === 'StorageKey', 'Artifact is not stored by managed key');
  assert(artifact.state === 'ready' && artifact.ready_at, 'Artifact was not finalized');
  assert(
    source.ManagedLocalFile.artifact_id === artifact.id,
    'Project source and artifact table reference different artifacts',
  );

  assert(outbox.length === 4, `Expected four outbox records, found ${outbox.length}`);
  const finalizeOutbox = outbox.filter(
    (record) => record.kind === 'finalize_staged_artifact' && record.aggregate_id === project.id,
  );
  const terminalOutbox = outbox.filter((record) => record.kind === 'handle_terminal_job_state');
  assert(finalizeOutbox.length === 1, 'Expected one finalize outbox record');
  assert(terminalOutbox.length === 1, 'Expected one terminal job outbox record');
  for (const record of [...finalizeOutbox, ...terminalOutbox]) {
    assert(record.status === 'done', `${record.kind} outbox status is ${record.status}`);
    assert(record.last_error === null, `${record.kind} outbox record contains an error`);
  }
  assert(
    JSON.parse(terminalOutbox[0].payload_json).outcome === 'cancelled',
    'Terminal outbox did not preserve the cancelled outcome',
  );

  assert(jobs.length === 1, `Expected one pipeline job, found ${jobs.length}`);
  assert(jobs[0].project_id === project.id, 'Job belongs to a different project');
  assert(jobs[0].status === 'cancelled', `Unexpected job status: ${jobs[0].status}`);
  assert(jobs[0].finished_at, 'Cancelled job has no finished timestamp');
  assert(
    JSON.parse(jobs[0].progress_json).message === 'Runtime stopped; job cancelled',
    'Cancelled job retained stale progress text',
  );

  const projectsRoot = path.resolve(dataRoot, 'projects');
  const artifactPath = path.resolve(projectsRoot, ...artifact.location_value.split('/'));
  assert(
    artifactPath.startsWith(`${projectsRoot}${path.sep}`),
    'Artifact storage key escaped the managed project directory',
  );
  const sourceStat = await fsp.stat(sourcePath);
  const artifactStat = await fsp.stat(artifactPath);
  assert(artifactStat.isFile(), 'Final artifact path is not a file');
  assert(
    artifactStat.size === sourceStat.size,
    'Final artifact size differs from the imported file',
  );
  assert(Number(artifact.size_bytes) === sourceStat.size, 'SQLite artifact size is incorrect');
  assert(
    (await sha256(artifactPath)) === (await sha256(sourcePath)),
    'Final artifact bytes differ',
  );

  assert(
    youtubeProject.status === 'ReadyForProcessing',
    `Unexpected yt-dlp project status: ${youtubeProject.status}`,
  );
  const youtubeSource = JSON.parse(youtubeProject.source_json);
  assert(
    youtubeSource.YoutubeUrl?.url === ytdlpUrl,
    'yt-dlp project did not preserve its source URL',
  );
  assert(youtubeArtifacts.length === 1, 'Expected one downloaded yt-dlp artifact');
  const youtubeArtifact = youtubeArtifacts[0];
  assert(youtubeArtifact.project_id === youtubeProject.id, 'yt-dlp artifact ownership mismatch');
  assert(youtubeArtifact.location_kind === 'StorageKey', 'yt-dlp artifact is not managed');
  assert(
    youtubeArtifact.state === 'ready' && youtubeArtifact.ready_at,
    'yt-dlp artifact was not finalized',
  );
  assert(ytdlpRequestCount >= 2, `Expected yt-dlp HTTP traffic, observed ${ytdlpRequestCount}`);

  const youtubeArtifactPath = path.resolve(
    projectsRoot,
    ...youtubeArtifact.location_value.split('/'),
  );
  assert(
    youtubeArtifactPath.startsWith(`${projectsRoot}${path.sep}`),
    'yt-dlp artifact storage key escaped the managed project directory',
  );
  const youtubeArtifactStat = await fsp.stat(youtubeArtifactPath);
  assert(youtubeArtifactStat.isFile(), 'Final yt-dlp artifact path is not a file');
  assert(
    youtubeArtifactStat.size === sourceStat.size,
    'yt-dlp artifact size differs from the served fixture',
  );
  assert(
    (await sha256(youtubeArtifactPath)) === (await sha256(sourcePath)),
    'yt-dlp artifact bytes differ from the served fixture',
  );

  const youtubeFinalizeOutbox = outbox.filter(
    (record) =>
      record.kind === 'finalize_staged_artifact' && record.aggregate_id === youtubeProject.id,
  );
  const workspaceCleanupOutbox = outbox.filter(
    (record) => record.kind === 'delete_workspace_allocation',
  );
  assert(youtubeFinalizeOutbox.length === 1, 'Expected one yt-dlp finalize outbox record');
  assert(workspaceCleanupOutbox.length === 1, 'Expected one workspace cleanup outbox record');
  for (const record of outbox) {
    assert(record.status === 'done', `${record.kind} outbox status is ${record.status}`);
    assert(record.last_error === null, `${record.kind} outbox record contains an error`);
  }

  const stagingFiles = await listFiles(path.join(projectsRoot, '.staging'));
  assert(
    stagingFiles.length === 0,
    `Staging files remain after finalize: ${stagingFiles.join(', ')}`,
  );
  const workspaceFiles = await listFiles(path.join(dataRoot, 'cache', 'workspaces'));
  assert(
    workspaceFiles.length === 0,
    `Workspace files remain after cancellation: ${workspaceFiles.join(', ')}`,
  );
}

async function listFiles(root) {
  if (!fs.existsSync(root)) return [];
  const files = [];
  for (const entry of await fsp.readdir(root, { withFileTypes: true })) {
    const candidate = path.join(root, entry.name);
    if (entry.isDirectory()) files.push(...(await listFiles(candidate)));
    else files.push(candidate);
  }
  return files;
}

function assert(condition, message) {
  if (!condition) throw new Error(message);
}
