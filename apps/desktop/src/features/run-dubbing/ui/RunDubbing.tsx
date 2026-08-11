import { useState, useRef, useLayoutEffect } from 'react';
import { Button } from '../../../shared/ui/button';
import { useProjectContext, startProjectMockPipeline } from '@/entities/project';
import { toast } from '@/shared/ui/toast';
import { toCommandError } from '@/shared/api/contracts';
import { supportsSubtitleImport } from '@/entities/media';
import type { SubtitleTrack } from '@/entities/transcript';

export const RunDubbing = ({
  subtitleTrack,
  label,
  disabled = false,
}: {
  subtitleTrack?: SubtitleTrack | null;
  label?: string;
  disabled?: boolean;
} = {}) => {
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
      latestAttemptRef.current === attemptId && activeAttemptRef.current === attemptId;

    const isCurrentAttempt = () => ownsAttempt() && validateToken(token);

    setIsStarting(true);
    try {
      const response = subtitleTrack
        ? await startProjectMockPipeline(project.id, subtitleTrack)
        : await startProjectMockPipeline(project.id);
      if (!isCurrentAttempt()) return;

      setIsStarting(false);
      activeAttemptRef.current = null;

      setProject(response.project);
    } catch (e: any) {
      if (!isCurrentAttempt()) return;
      const cmdErr = toCommandError(e);
      console.error('Failed to start mock dubbing job', cmdErr);
      toast.error(cmdErr.message);
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
  const canImportSubtitles = supportsSubtitleImport(project?.source ?? null);
  if (project?.id && isEligible && !canImportSubtitles) return null;

  const isDisabled =
    disabled ||
    !project?.id ||
    isStarting ||
    !isEligible ||
    !canImportSubtitles ||
    deletingProjectId !== null;

  return (
    <div className="flex flex-col items-end gap-1">
      <Button variant="primary" onClick={handleStart} disabled={isDisabled}>
        {isStarting ? 'Starting subtitle import...' : label || 'Import subtitles'}
      </Button>
    </div>
  );
};
