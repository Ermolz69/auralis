import { useState } from 'react';
import { cancelJob } from '@/entities/job';
import { toCommandError } from '@/shared/api/contracts';

export interface CancelJobButtonProps {
  jobId: string;
  onCancelled?: () => void;
  className?: string;
}

export function CancelJobButton({ jobId, onCancelled, className }: CancelJobButtonProps) {
  const [isCancelling, setIsCancelling] = useState(false);

  const handleCancel = async () => {
    setIsCancelling(true);
    try {
      await cancelJob(jobId);
      onCancelled?.();
    } catch (e) {
      console.error('Failed to cancel job', toCommandError(e));
    } finally {
      setIsCancelling(false);
    }
  };

  return (
    <button
      className={`px-3 py-1 bg-danger hover:bg-danger text-white rounded text-sm disabled:opacity-50 ${className || ''}`}
      onClick={handleCancel}
      disabled={isCancelling}
    >
      {isCancelling ? 'Cancelling...' : 'Cancel'}
    </button>
  );
}
