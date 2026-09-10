import { useState, useLayoutEffect } from 'react';
import { Button } from '../../../shared/ui/button';
import {
  useProjectContext,
  useProjectOperation,
  startProjectMockPipeline,
} from '@/entities/project';
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
  const { project, setProject, deletingProjectId, projectId, operationGeneration } =
    useProjectContext();

  const operation = useProjectOperation();

  useLayoutEffect(() => {
    setIsStarting(false);
  }, [operationGeneration, projectId]);

  const handleStart = async () => {
    if (!project?.id || deletingProjectId !== null || isStarting) return;
    const attempt = operation.begin();
    if (!attempt) return;

    setIsStarting(true);
    try {
      const response = subtitleTrack
        ? await startProjectMockPipeline(project.id, subtitleTrack)
        : await startProjectMockPipeline(project.id);
      if (!operation.isCurrent(attempt)) return;

      setIsStarting(false);
      operation.finish(attempt);

      setProject(response.project);
    } catch (e: unknown) {
      if (!operation.isCurrent(attempt)) return;
      const cmdErr = toCommandError(e);
      console.error('Failed to start mock dubbing job', cmdErr);
      toast.error(cmdErr.message);
    } finally {
      if (operation.finish(attempt)) {
        setIsStarting(false);
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
