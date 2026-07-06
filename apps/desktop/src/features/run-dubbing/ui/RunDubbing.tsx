import { useState } from 'react';
import { Button } from '../../../shared/ui/button';
import { startMockDubbingJob } from '@/entities/job';

export const RunDubbing = () => {
  const [isStarting, setIsStarting] = useState(false);

  const handleStart = async () => {
    setIsStarting(true);
    try {
      await startMockDubbingJob('mock://current-project');
    } catch (e) {
      console.error('Failed to start mock dubbing job', e);
    } finally {
      setIsStarting(false);
    }
  };

  return (
    <Button variant="primary" onClick={handleStart} disabled={isStarting}>
      {isStarting ? 'Starting...' : 'Run Dubbing'}
    </Button>
  );
};
