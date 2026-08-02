import { useId, useState } from 'react';
import { cancelJob } from '@/entities/job';
import { toCommandError } from '@/shared/api/contracts';
import { Button } from '@/shared/ui/button';

export interface CancelJobButtonProps {
  jobId: string;
  onCancelled?: () => void;
  className?: string;
}

export function CancelJobButton({ jobId, onCancelled, className }: CancelJobButtonProps) {
  const [isCancelling, setIsCancelling] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState(false);
  const errorId = useId();
  const successId = useId();

  const handleCancel = async () => {
    if (isCancelling) return;

    setError(null);
    setSuccess(false);
    setIsCancelling(true);
    try {
      await cancelJob(jobId);
      setSuccess(true);
      onCancelled?.();
    } catch (e) {
      const commandError = toCommandError(e);
      setError(commandError.message);
    } finally {
      setIsCancelling(false);
    }
  };

  return (
    <div className="flex flex-col items-end gap-1">
      <Button
        type="button"
        variant="danger"
        size="sm"
        loading={isCancelling}
        className={className}
        onClick={handleCancel}
        disabled={isCancelling}
        aria-describedby={error ? errorId : success ? successId : undefined}
      >
        {isCancelling ? 'Cancelling...' : 'Cancel'}
      </Button>
      {success && !error && (
        <p id={successId} className="max-w-48 text-right text-xs text-muted" role="status">
          Cancellation requested.
        </p>
      )}
      {error && (
        <p id={errorId} className="max-w-48 text-right text-xs text-danger" role="alert">
          Cancel failed: {error}
        </p>
      )}
    </div>
  );
}
