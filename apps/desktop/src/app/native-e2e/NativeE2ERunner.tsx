import { useEffect } from 'react';
import { importLocalMedia } from '@/entities/media';
import { createProject, renameProject } from '@/entities/project';
import type { Artifact } from '@/shared/api/contracts';
import { invoke } from '@/shared/api/tauri';

const READY_TIMEOUT_MS = 20_000;
const POLL_INTERVAL_MS = 50;
let started = false;

export function NativeE2ERunner() {
  useEffect(() => {
    if (started) return;
    started = true;
    void runNativeScenario();
  }, []);

  return null;
}

async function runNativeScenario() {
  const runId = __NATIVE_E2E_RUN_ID__;
  let projectId: string | undefined;

  try {
    await invoke('health_check');
    const project = await createProject(`native-e2e-running:${runId}`);
    projectId = project.id;

    await importLocalMedia(project.id, __NATIVE_E2E_MEDIA_PATH__);
    const artifact = await waitForSourceArtifact(project.id);
    const resolvedPath = await invoke('resolve_artifact_path_cmd', { artifactId: artifact.id });

    if (artifact.location.kind !== 'storageKey' || !resolvedPath.trim()) {
      throw new Error('Native artifact did not resolve to managed storage');
    }

    await renameProject(project.id, `native-e2e-complete:${runId}`);
  } catch (error) {
    console.error('Native E2E scenario failed', error);
    if (projectId) {
      await renameProject(projectId, `native-e2e-failed:${runId}`).catch(() => undefined);
    }
  }
}

async function waitForSourceArtifact(projectId: string): Promise<Artifact> {
  const deadline = Date.now() + READY_TIMEOUT_MS;
  while (Date.now() < deadline) {
    const artifacts = await invoke('list_project_artifacts_cmd', {
      projectId,
      kind: 'sourceVideo',
    });
    const ready = artifacts.find((artifact) => artifact.state === 'ready');
    if (ready) return ready;
    await delay(POLL_INTERVAL_MS);
  }
  throw new Error('Timed out waiting for the source artifact to become ready');
}

function delay(durationMs: number) {
  return new Promise<void>((resolve) => setTimeout(resolve, durationMs));
}
