import { Notice } from '../../../shared/ui/notice';
import { StateView } from '../../../shared/ui/state-view';
import { isActiveJobStatus, useJobContext } from '@/entities/job';
import type { JobDto, JobStoreState } from '@/entities/job';
import { Button } from '@/shared/ui/button';
import { JobCard } from './JobCard';
import { useJobHistory } from './useJobHistory';

const isActiveJob = (job: JobDto) => isActiveJobStatus(job.status);

const getSyncNotice = (phase: JobStoreState['phase'], pendingRefetch: boolean) => {
  if (phase === 'initializing') {
    return {
      role: 'status' as const,
      icon: 'LoaderCircle' as const,
      tone: 'neutral' as const,
      title: 'Opening job sync',
      body: 'Jobs across all projects are being connected.',
    };
  }

  if (phase === 'synchronizing') {
    return {
      role: 'status' as const,
      icon: 'RefreshCw' as const,
      tone: 'neutral' as const,
      title: 'Refreshing jobs',
      body: 'The queue is loading the latest operation state.',
    };
  }

  if (phase === 'stale' || pendingRefetch) {
    return {
      role: 'alert' as const,
      icon: 'CircleAlert' as const,
      tone: 'warning' as const,
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
  const { activeJobs, completedJobs, phase, pendingRefetch } = useJobContext();
  const history = useJobHistory(completedJobs, phase);
  const syncNotice = getSyncNotice(phase, pendingRefetch);
  const hasActiveJobs = activeJobs.some(isActiveJob);
  const hasJobs = activeJobs.length > 0 || history.jobs.length > 0;

  return (
    <aside
      className={`flex h-full min-w-0 flex-col gap-3 overflow-hidden bg-surface p-3 ${className}`}
      aria-label="Job queue"
    >
      <h2 className="shrink-0 text-sm font-semibold text-text">Job Queue</h2>
      {syncNotice && (
        <Notice
          icon={syncNotice.icon}
          title={syncNotice.title}
          tone={syncNotice.tone}
          role={syncNotice.role}
          live={syncNotice.role === 'alert' ? 'assertive' : 'polite'}
        >
          {syncNotice.body}
        </Notice>
      )}
      {hasActiveJobs && (
        <Notice
          icon="Info"
          title="Operation keeps running while you browse"
          tone="accent"
          role="status"
          live="polite"
        >
          You can switch pages safely. Closing the app can interrupt the operation; on next launch
          the queue will show the recovered final state.
        </Notice>
      )}
      <div className="flex-1 flex flex-col gap-3 overflow-y-auto min-h-0">
        {!hasJobs && !history.isLoading && !history.error ? (
          <StateView
            icon="Inbox"
            title="Queue is empty"
            description="Jobs will appear here"
            density="compact"
            className="flex-1 p-4"
          />
        ) : (
          <>
            {activeJobs.length > 0 && (
              <section aria-labelledby="active-operations-heading" className="flex flex-col gap-3">
                <h3
                  id="active-operations-heading"
                  className="text-[10px] font-semibold uppercase tracking-wider text-subtle"
                >
                  Active operation
                </h3>
                <ul className="flex flex-col gap-3" aria-label="Active operations">
                  {activeJobs.map((job) => (
                    <li key={job.id}>
                      <JobCard job={job} />
                    </li>
                  ))}
                </ul>
              </section>
            )}

            {history.jobs.length > 0 && (
              <section aria-labelledby="job-history-heading" className="flex flex-col gap-3">
                <h3
                  id="job-history-heading"
                  className="text-[10px] font-semibold uppercase tracking-wider text-subtle"
                >
                  History
                </h3>
                <ul className="flex flex-col gap-3" aria-label="Operation history">
                  {history.jobs.map((job) => (
                    <li key={job.id}>
                      <JobCard job={job} />
                    </li>
                  ))}
                </ul>
                {history.hasMore && (
                  <Button
                    type="button"
                    variant="secondary"
                    size="sm"
                    loading={history.isLoading}
                    disabled={history.isLoading}
                    onClick={history.loadMore}
                  >
                    {history.isLoading ? 'Loading history...' : 'Load older jobs'}
                  </Button>
                )}
              </section>
            )}
            {history.isLoading && history.jobs.length === 0 && (
              <p className="text-xs text-muted" role="status">
                Loading job history...
              </p>
            )}
            {history.error && (
              <div className="flex items-center justify-between gap-2" role="alert">
                <p className="text-xs text-danger">History unavailable: {history.error}</p>
                <Button type="button" variant="secondary" size="sm" onClick={history.retry}>
                  Retry
                </Button>
              </div>
            )}
          </>
        )}
      </div>
    </aside>
  );
};
