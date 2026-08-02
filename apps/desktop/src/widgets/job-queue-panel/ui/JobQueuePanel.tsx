import { Icon } from '../../../shared/ui/icon';
import { isActiveJobStatus, useJobContext } from '@/entities/job';
import type { JobDto, JobStoreState } from '@/entities/job';
import { JobCard } from './JobCard';

const isActiveJob = (job: JobDto) => isActiveJobStatus(job.status);

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
  const syncNotice = getSyncNotice(phase, pendingRefetch, scopeProjectId);
  const hasActiveJobs = activeJobs.some(isActiveJob);

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
            <p className="text-sm font-medium text-text">
              Operation keeps running while you browse
            </p>
            <p className="text-xs">
              You can switch pages safely. Closing the app can interrupt the operation; on next
              launch the queue will show the recovered final state.
            </p>
          </div>
        </div>
      )}
      <div className="flex-1 flex flex-col gap-3 overflow-y-auto min-h-0">
        {activeJobs.length === 0 && completedJobs.length === 0 ? (
          <div className="flex-1 flex flex-col items-center justify-center text-center p-4">
            <Icon name="Inbox" size="lg" className="text-muted/50 mb-3" />
            <p className="text-text font-medium">Queue is empty</p>
            <p className="text-sm text-muted mt-1">Jobs will appear here</p>
          </div>
        ) : (
          <>
            {activeJobs.length > 0 && (
              <section aria-labelledby="active-operations-heading" className="flex flex-col gap-3">
                <h3 id="active-operations-heading" className="text-sm font-semibold text-text">
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

            {completedJobs.length > 0 && (
              <section aria-labelledby="job-history-heading" className="flex flex-col gap-3">
                <h3 id="job-history-heading" className="text-sm font-semibold text-text">
                  History
                </h3>
                <ul className="flex flex-col gap-3" aria-label="Operation history">
                  {completedJobs.map((job) => (
                    <li key={job.id}>
                      <JobCard job={job} />
                    </li>
                  ))}
                </ul>
              </section>
            )}
          </>
        )}
      </div>
    </aside>
  );
};
