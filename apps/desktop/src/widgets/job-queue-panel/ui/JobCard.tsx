import { useContext, useId, useRef, useState } from 'react';
import { Card, CardContent } from '../../../shared/ui/card';
import { Progress } from '../../../shared/ui/progress';
import { Icon } from '../../../shared/ui/icon';
import { formatJobStatus, getJobStatusTone, isActiveJobStatus } from '@/entities/job';
import type { JobDto } from '@/entities/job';
import { ProjectContext, startProjectMockPipeline } from '@/entities/project';
import { formatProjectTitle, supportsSubtitleImport } from '@/entities/media';
import { CancelJobButton } from '@/features/cancel-job';
import { Button } from '@/shared/ui/button';
import { toCommandError } from '@/shared/api/contracts';

const isRestartRecoveryFailure = (job: JobDto) =>
  job.status === 'failed' && /application restart|APP_RESTART/i.test(job.error ?? '');

const getBackendPercent = (job: JobDto) =>
  Number.isFinite(job.progress.percent) ? job.progress.percent : null;

export function JobCard({ job }: { job: JobDto }) {
  const [retryingJobId, setRetryingJobId] = useState<string | null>(null);
  const [retryError, setRetryError] = useState<string | null>(null);
  const retryAttemptRef = useRef(0);
  const retryErrorId = useId();
  const projectContext = useContext(ProjectContext);
  const project = projectContext?.project ?? null;
  const deletingProjectId = projectContext?.deletingProjectId ?? null;
  const isActive = isActiveJobStatus(job.status);
  const wasRecoveredAfterRestart = isRestartRecoveryFailure(job);
  const statusLabel = formatJobStatus(job);
  const progressMessage = job.progress.message || (job.status === 'pending' ? 'Queued' : 'Working');
  const backendPercent = getBackendPercent(job);
  const isIndeterminate =
    backendPercent === null || (job.status === 'pending' && backendPercent === 0);
  const projectLabel =
    project && job.projectId === project.id
      ? formatProjectTitle(project.title, project.source)
      : job.projectId
        ? `Project ${job.projectId}`
        : 'Project not attached';
  const canRetry =
    job.status === 'failed' &&
    project?.id === job.projectId &&
    deletingProjectId === null &&
    (project.status === 'ready_for_processing' || project.status === 'failed') &&
    supportsSubtitleImport(project.source);
  const isRetrying = retryingJobId === job.id;

  const handleRetry = async () => {
    if (!canRetry || isRetrying || !project?.id || !projectContext) return;

    const token = projectContext.captureToken();
    if (!projectContext.validateToken(token)) return;

    const attemptId = ++retryAttemptRef.current;
    setRetryingJobId(job.id);
    setRetryError(null);

    try {
      const response = await startProjectMockPipeline(project.id);
      if (retryAttemptRef.current !== attemptId || !projectContext.validateToken(token)) return;
      projectContext.setProject(response.project);
    } catch (e) {
      if (retryAttemptRef.current !== attemptId || !projectContext.validateToken(token)) return;
      setRetryError(toCommandError(e).message);
    } finally {
      if (retryAttemptRef.current === attemptId) setRetryingJobId(null);
    }
  };

  return (
    <Card variant="muted" className="shrink-0">
      <CardContent className="p-4 flex flex-col gap-3">
        <div className="flex justify-between items-start gap-3">
          <div className="flex min-w-0 flex-col">
            <p className="truncate text-sm text-text font-medium">{job.title}</p>
            <p className="truncate text-xs text-muted">Project: {projectLabel}</p>
            <p className="text-xs text-muted">{statusLabel}</p>
          </div>
          {isActive && <CancelJobButton jobId={job.id} />}
        </div>

        {isActive && (
          <div className="flex flex-col gap-1" role="status" aria-live="polite">
            <Progress
              value={backendPercent ?? 0}
              variant={getJobStatusTone(job.status)}
              indeterminate={isIndeterminate}
              aria-label={`${job.title} progress`}
            />
            <div className="flex justify-between gap-3 text-xs text-muted">
              <span className="min-w-0 truncate">{progressMessage}</span>
              <span>{backendPercent === null ? 'In progress' : `${backendPercent}%`}</span>
            </div>
          </div>
        )}

        {job.status === 'failed' && (
          <div className="flex flex-col gap-1" role="alert">
            <p className="text-xs font-medium text-danger">{job.error || 'Operation failed'}</p>
            {job.progress.message && (
              <p className="text-xs text-muted">Final state: {job.progress.message}</p>
            )}
            <FailureRecoveryMessage wasRecoveredAfterRestart={wasRecoveredAfterRestart} />
            {canRetry && (
              <div className="flex flex-col items-start gap-1 pt-1">
                <Button
                  type="button"
                  variant="secondary"
                  size="sm"
                  loading={isRetrying}
                  disabled={isRetrying}
                  onClick={handleRetry}
                  aria-describedby={retryError ? retryErrorId : undefined}
                >
                  {isRetrying ? 'Retrying...' : 'Retry subtitle import'}
                </Button>
                {retryError && (
                  <p id={retryErrorId} className="text-xs text-danger" role="alert">
                    Retry failed: {retryError}
                  </p>
                )}
              </div>
            )}
          </div>
        )}

        {job.status === 'cancelled' && <CancelledState />}
        {job.status === 'completed' && <CompletedState message={job.progress.message} />}
      </CardContent>
    </Card>
  );
}

function FailureRecoveryMessage({
  wasRecoveredAfterRestart,
}: {
  wasRecoveredAfterRestart: boolean;
}) {
  if (wasRecoveredAfterRestart) {
    return (
      <p className="text-xs text-muted">
        This operation was recovered after restart as interrupted. Review the project state, then
        retry only if the missing result is still needed.
      </p>
    );
  }

  return (
    <p className="text-xs text-muted">
      Check the message above, then retry when this project has a supported source.
    </p>
  );
}

function CancelledState() {
  return (
    <div className="flex items-start gap-2 text-xs text-muted" role="status" aria-live="polite">
      <Icon name="CircleStop" size="sm" color="warning" />
      <p>Cancelled before completion. Start a new supported operation when ready.</p>
    </div>
  );
}

function CompletedState({ message }: { message: string }) {
  return (
    <div className="flex flex-col gap-1" role="status" aria-live="polite">
      <div className="flex items-center gap-2 text-xs text-success">
        <Icon name="CircleCheck" size="sm" color="success" />
        <span>Completed successfully</span>
      </div>
      {message && <p className="text-xs text-muted">{message}</p>}
    </div>
  );
}
