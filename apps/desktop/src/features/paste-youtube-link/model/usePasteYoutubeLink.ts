import { useState, useLayoutEffect } from 'react';
import {
  createProjectFromYoutube,
  useProjectContext,
  useProjectOperation,
} from '@/entities/project';
import type { Project } from '@/entities/project';
import { useNavigation } from '@/shared/router';
import { toCommandError } from '@/shared/api/contracts';
import type { YoutubeImportStatus } from './status';

export function usePasteYoutubeLink() {
  const [url, setUrl] = useState('');
  const [status, setStatus] = useState<YoutubeImportStatus>('Idle');
  const isStarting = status === 'Downloading';
  const [error, setError] = useState<string | null>(null);
  const { deletingProjectId, setProject, projectId, operationGeneration } = useProjectContext();
  const { setCurrentView, setPipelineStep = () => undefined } = useNavigation();

  const operation = useProjectOperation();

  useLayoutEffect(() => {
    setStatus('Idle');
    setError(null);
  }, [operationGeneration, projectId]);

  const isBlockedByDeletion = deletingProjectId !== null;

  const startProject = async (): Promise<Project | null> => {
    const trimmedUrl = url.trim();
    if (!trimmedUrl || isStarting || deletingProjectId !== null) return null;

    const attempt = operation.begin();
    if (!attempt) return null;

    setStatus('Downloading');
    setError(null);
    try {
      const project = projectId
        ? await createProjectFromYoutube(trimmedUrl, projectId)
        : await createProjectFromYoutube(trimmedUrl);
      if (!operation.isCurrent(attempt)) return null;

      setStatus('Ready');
      operation.finish(attempt);

      setUrl('');
      setProject(project);
      setPipelineStep('source');
      setCurrentView('project');
      return project;
    } catch (err: unknown) {
      if (!operation.isCurrent(attempt)) return null;
      const cmdErr = toCommandError(err);
      setStatus('DownloadFailed');
      setError(cmdErr.message);
      return null;
    } finally {
      operation.finish(attempt);
    }
  };

  return {
    url,
    setUrl,
    startProject,
    isStarting,
    status,
    isBlockedByDeletion,
    error,
  };
}
