import { useId, useState } from 'react';
import { cancelJob } from '@/entities/job';
import { toCommandError } from '@/shared/api/contracts';

export interface CancelJobButtonProps {
  jobId: string;
  onCancelled?: () => void;
  className?: string;
}

export function CancelJobButton({ jobId, onCancelled, className }: CancelJobButtonProps) {
  const [isCancelling, setIsCancelling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const errorId = useId();

  const handleCancel = async () => {
    setError(null);
    setIsCancelling(true);
    try {
      await cancelJob(jobId);
      onCancelled?.();
    } catch (e) {
      const commandError = toCommandError(e);
      console.error('Failed to cancel job', commandError);
      setError(commandError.message);
    } finally {
      setIsCancelling(false);
    }
  };

  return (
    <div className="flex flex-col items-end gap-1">
      <button
        type="button"
        className={`px-3 py-1 bg-danger-action hover:bg-danger-action-hover text-white rounded text-sm disabled:opacity-50 ${className || ''}`}
        onClick={handleCancel}
        disabled={isCancelling}
        aria-describedby={error ? errorId : undefined}
      >
        {isCancelling ? 'Cancelling...' : 'Cancel'}
      </button>
      {error && (
        <p id={errorId} className="max-w-48 text-right text-xs text-danger" role="alert">
          Cancel failed: {error}
        </p>
      )}
    </div>
  );
}
