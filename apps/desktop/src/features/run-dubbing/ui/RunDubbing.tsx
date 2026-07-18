import { useState, useRef, useLayoutEffect } from 'react';
import { Button } from '../../../shared/ui/button';
import { useProjectContext, startProjectMockPipeline } from '@/entities/project';
import { toast } from '@/shared/ui/toast';

export const RunDubbing = () => {
  const [isStarting, setIsStarting] = useState(false);
  const {
    project,
    setProject,
    deletingProjectId,
    projectId,
    operationGeneration,
    captureToken,
    validateToken,
  } = useProjectContext();

  const latestAttemptRef = useRef(0);
  const activeAttemptRef = useRef<number | null>(null);

  useLayoutEffect(() => {
    setIsStarting(false);
    activeAttemptRef.current = null;
    latestAttemptRef.current += 1;
  }, [operationGeneration, projectId]);

  const handleStart = async () => {
    if (!project?.id || deletingProjectId !== null || isStarting) return;
    if (activeAttemptRef.current !== null) return;

    const token = captureToken();
    if (!validateToken(token)) return;

    const attemptId = ++latestAttemptRef.current;
    activeAttemptRef.current = attemptId;

    const ownsAttempt = () =>
      latestAttemptRef.current === attemptId &&
      activeAttemptRef.current === attemptId;

    const isCurrentAttempt = () => ownsAttempt() && validateToken(token);

    setIsStarting(true);
    try {
      const response = await startProjectMockPipeline(project.id);
      if (!isCurrentAttempt()) return;

      setIsStarting(false);
      activeAttemptRef.current = null;

      setProject(response.project);
    } catch (e: any) {
      if (!isCurrentAttempt()) return;
      console.error('Failed to start mock dubbing job', e);
      toast.error(e?.message || 'Failed to start pipeline');
    } finally {
      if (ownsAttempt()) {
        activeAttemptRef.current = null;
        if (validateToken(token)) {
          setIsStarting(false);
        }
      }
    }
  };

  const isEligible = project?.status === 'ready_for_processing' || project?.status === 'failed';
  const isDisabled = !project?.id || isStarting || !isEligible || deletingProjectId !== null;

  return (
    <Button variant="primary" onClick={handleStart} disabled={isDisabled}>
      {isStarting ? 'Starting...' : 'Run Dubbing'}
    </Button>
  );
};

