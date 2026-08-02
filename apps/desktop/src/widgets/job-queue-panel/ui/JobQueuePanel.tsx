import { Card, CardContent } from '../../../shared/ui/card';
import { Progress } from '../../../shared/ui/progress';
import { Icon } from '../../../shared/ui/icon';
import { useJobContext } from '@/entities/job';
import type { JobDto, JobStoreState } from '@/entities/job';
import { CancelJobButton } from '@/features/cancel-job';

const formatStage = (stage: string | null) => {
  if (!stage) return '';
  const withSpaces = stage.replace(/([A-Z])/g, ' $1');
  return withSpaces.charAt(0).toUpperCase() + withSpaces.slice(1);
};

const formatJobStatus = (job: JobDto) => {
  const stage = job.stage ? ` - ${formatStage(job.stage)}` : '';
  switch (job.status) {
    case 'pending':
      return `Waiting to start${stage}`;
    case 'running':
      return `Running${stage}`;
    case 'completed':
      return 'Completed';
    case 'cancelled':
      return 'Cancelled';
    case 'failed':
      return 'Failed';
  }
};

const getProgressVariant = (status: JobDto['status']) => {
  if (status === 'completed') return 'success';
  if (status === 'failed') return 'danger';
  if (status === 'cancelled') return 'warning';
  return 'default';
};

const isActiveJob = (job: JobDto) => job.status === 'pending' || job.status === 'running';

const isRestartRecoveryFailure = (job: JobDto) =>
  job.status === 'failed' && /application restart|APP_RESTART/i.test(job.error ?? '');

const getSyncNotice = (
  phase: JobStoreState['phase'],
  pendingRefetch: boolean,
  scopeProjectId: string | null,
) => {
  if (!scopeProjectId) {
    return null;
  }

  if (phase === 'initializing') {
    return {
      role: 'status' as const,
      icon: 'LoaderCircle' as const,
      tone: 'text-muted border-muted/40 bg-bg',
      title: 'Opening job sync',
      body: 'Jobs for this project are being connected.',
    };
  }

  if (phase === 'synchronizing') {
    return {
      role: 'status' as const,
      icon: 'RefreshCw' as const,
      tone: 'text-muted border-muted/40 bg-bg',
      title: 'Refreshing jobs',
      body: 'The queue is loading the latest operation state.',
    };
  }

  if (phase === 'stale' || pendingRefetch) {
    return {
      role: 'alert' as const,
      icon: 'CircleAlert' as const,
      tone: 'text-warning border-warning/40 bg-warning/10',
      title: 'Job state may be outdated',
      body: 'A refresh is in progress. Avoid repeating actions until the queue updates.',
    };
  }

  return null;
};

type JobQueuePanelProps = {
  className?: string;
};

export const JobQueuePanel = ({ className = '' }: JobQueuePanelProps) => {
  const { activeJobs, completedJobs, phase, pendingRefetch, scopeProjectId } = useJobContext();
  const jobs = [...activeJobs, ...completedJobs];
  const syncNotice = getSyncNotice(phase, pendingRefetch, scopeProjectId);
  const hasActiveJobs = jobs.some(isActiveJob);

  return (
    <aside
      className={`h-full bg-surface p-6 flex flex-col gap-4 overflow-hidden min-w-0 ${className}`}
      aria-label="Job queue"
    >
      <h2 className="text-lg font-semibold text-text shrink-0">Job Queue</h2>
      {syncNotice && (
        <div
          className={`flex gap-3 rounded-md border p-3 ${syncNotice.tone}`}
          role={syncNotice.role}
          aria-live={syncNotice.role === 'alert' ? 'assertive' : 'polite'}
        >
          <Icon name={syncNotice.icon} size="sm" className="mt-0.5" />
          <div className="min-w-0">
            <p className="text-sm font-medium text-text">{syncNotice.title}</p>
            <p className="text-xs">{syncNotice.body}</p>
          </div>
        </div>
      )}
      {hasActiveJobs && (
        <div
          className="flex gap-3 rounded-md border border-accent/40 bg-accent/10 p-3 text-accent-foreground"
          role="status"
          aria-live="polite"
        >
          <Icon name="Info" size="sm" className="mt-0.5" />
          <div className="min-w-0">
            <p className="text-sm font-medium text-text">Operation keeps running while you browse</p>
            <p className="text-xs">
              You can switch pages safely. Closing the app can interrupt the operation; on next
              launch the queue will show the recovered final state.
            </p>
          </div>
        </div>
      )}
      <div className="flex-1 flex flex-col gap-3 overflow-y-auto min-h-0">
        {jobs.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center text-center p-4">
            <Icon name="Inbox" size="lg" className="text-muted/50 mb-3" />
            <p className="text-text font-medium">Queue is empty</p>
            <p className="text-sm text-muted mt-1">Jobs will appear here</p>
          </div>
        ) : (
          jobs.map((job) => <JobCard key={job.id} job={job} />)
        )}
      </div>
    </aside>
  );
};

function JobCard({ job }: { job: JobDto }) {
  const isActive = isActiveJob(job);
  const wasRecoveredAfterRestart = isRestartRecoveryFailure(job);
  const statusLabel = formatJobStatus(job);
  const progressMessage = job.progress.message || (job.status === 'pending' ? 'Queued' : 'Working');

  return (
    <Card variant="muted" className="shrink-0">
      <CardContent className="p-4 flex flex-col gap-3">
        <div className="flex justify-between items-start gap-3">
          <div className="flex min-w-0 flex-col">
            <p className="truncate text-sm text-text font-medium">{job.title}</p>
            {job.projectId && <p className="truncate text-xs text-muted">Project: {job.projectId}</p>}
            <p className="text-xs text-muted">{statusLabel}</p>
          </div>
          {isActive && <CancelJobButton jobId={job.id} />}
        </div>

        {isActive && (
          <div className="flex flex-col gap-1" role="status" aria-live="polite">
            <Progress
              value={job.progress.percent}
              variant={getProgressVariant(job.status)}
              indeterminate={job.status === 'pending' && job.progress.percent === 0}
              aria-label={`${job.title} progress`}
            />
            <div className="flex justify-between gap-3 text-xs text-muted">
              <span className="min-w-0 truncate">{progressMessage}</span>
              <span>{job.progress.percent}%</span>
            </div>
          </div>
        )}

        {job.status === 'failed' && (
          <div className="flex flex-col gap-1" role="alert">
            <p className="text-xs font-medium text-danger">{job.error || 'Operation failed'}</p>
            {job.progress.message && (
              <p className="text-xs text-muted">Final state: {job.progress.message}</p>
            )}
            {wasRecoveredAfterRestart ? (
              <p className="text-xs text-muted">
                The app was closed before this operation finished. Review the project state, then
                start a supported operation again only if the missing result is still needed.
              </p>
            ) : (
              <p className="text-xs text-muted">
                Check the message above, then start a supported operation again when ready.
              </p>
            )}
          </div>
        )}

        {job.status === 'cancelled' && (
          <div className="flex items-start gap-2 text-xs text-muted" role="status" aria-live="polite">
            <Icon name="CircleStop" size="sm" color="warning" />
            <p>Cancelled before completion. Start a new supported operation when ready.</p>
          </div>
        )}

        {job.status === 'completed' && (
          <div className="flex flex-col gap-1" role="status" aria-live="polite">
            <div className="flex items-center gap-2 text-xs text-success">
              <Icon name="CircleCheck" size="sm" color="success" />
              <span>Completed successfully</span>
            </div>
            {job.progress.message && <p className="text-xs text-muted">{job.progress.message}</p>}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
