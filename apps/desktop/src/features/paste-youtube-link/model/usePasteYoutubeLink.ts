import { useState, useRef, useLayoutEffect } from 'react';
import { createProjectFromYoutube, useProjectContext } from '@/entities/project';
import type { Job } from '@/entities/job';
import type { Project } from '@/entities/project';
import { useNavigation } from '@/shared/router';

export function usePasteYoutubeLink() {
  const [url, setUrl] = useState('');
  const [isStarting, setIsStarting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const {
    deletingProjectId,
    setProjectId,
    setProject,
    projectId,
    operationGeneration,
    captureToken,
    validateToken,
  } = useProjectContext();
  const { setCurrentView } = useNavigation();

  const latestAttemptRef = useRef(0);
  const activeAttemptRef = useRef<number | null>(null);

  useLayoutEffect(() => {
    setIsStarting(false);
    activeAttemptRef.current = null;
    latestAttemptRef.current += 1;
  }, [operationGeneration, projectId]);

  const isBlockedByDeletion = deletingProjectId !== null;

  const startProject = async (): Promise<{ project: Project; job: Job } | null> => {
    if (!url || isStarting || deletingProjectId !== null) return null;

    if (activeAttemptRef.current !== null) return null;
    const token = captureToken();
    if (!validateToken(token)) return null;

    const attemptId = ++latestAttemptRef.current;
    activeAttemptRef.current = attemptId;

    const ownsAttempt = () =>
      latestAttemptRef.current === attemptId &&
      activeAttemptRef.current === attemptId;

    const isCurrentAttempt = () => ownsAttempt() && validateToken(token);

    setIsStarting(true);
    setError(null);
    try {
      const response = await createProjectFromYoutube(url);
      if (!isCurrentAttempt()) return null;

      setIsStarting(false);
      activeAttemptRef.current = null;

      setUrl(''); // clear input
      setProjectId(response.project.id);
      setProject(response.project);
      setCurrentView('project');
      return response;
    } catch (err: any) {
      if (!isCurrentAttempt()) return null;
      setError(err?.toString() || 'Failed to start project');
      return null;
    } finally {
      if (ownsAttempt()) {
        activeAttemptRef.current = null;
        if (validateToken(token)) {
          setIsStarting(false);
        }
      }
    }
  };

  return {
    url,
    setUrl,
    startProject,
    isStarting,
    isBlockedByDeletion,
    error,
  };
}

