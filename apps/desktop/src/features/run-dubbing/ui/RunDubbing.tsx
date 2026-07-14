import { useState } from 'react';
import { Button } from '../../../shared/ui/button';
import { useProjectContext, startProjectMockPipeline } from '@/entities/project';
import { toast } from '@/shared/ui/toast';

export const RunDubbing = () => {
  const [isStarting, setIsStarting] = useState(false);
  const { project, setProject } = useProjectContext();

  const handleStart = async () => {
    if (!project?.id) return;

    setIsStarting(true);
    try {
      const response = await startProjectMockPipeline(project.id);
      setProject(response.project);
    } catch (e: any) {
      console.error('Failed to start mock dubbing job', e);
      toast.error(e?.message || 'Failed to start pipeline');
    } finally {
      setIsStarting(false);
    }
  };

  const isEligible = project?.status === 'ready_for_processing' || project?.status === 'failed';
  const isDisabled = !project?.id || isStarting || !isEligible;

  return (
    <Button variant="primary" onClick={handleStart} disabled={isDisabled}>
      {isStarting ? 'Starting...' : 'Run Dubbing'}
    </Button>
  );
};
