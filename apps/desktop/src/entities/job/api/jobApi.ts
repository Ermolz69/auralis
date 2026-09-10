import { invoke, listen } from '@/shared/api/tauri';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { Job, JobEvent } from '../model/types';
import type { JobHistoryCursor, JobHistoryPage } from '@/shared/api/contracts/job';

const JOB_EVENT_NAME = 'job-event';
const JOB_EVENTS_INVALIDATED_NAME = 'job-events-invalidated';

export async function listJobs(): Promise<Job[]> {
  return invoke('list_jobs_cmd');
}

export async function listJobHistoryPage(
  cursor: JobHistoryCursor | null = null,
  limit = 100,
): Promise<JobHistoryPage> {
  return invoke('list_job_history_page_cmd', { cursor, limit });
}

export async function cancelJob(jobId: string): Promise<Job> {
  return invoke('cancel_job_cmd', { jobId });
}

export async function getJobsSnapshot(projectId: string): Promise<Job[]> {
  return invoke('list_jobs_snapshot_cmd', { projectId });
}

export async function subscribeJobEvents(
  handler: (event: JobEvent) => void,
  onInvalidPayload?: () => void,
): Promise<UnlistenFn> {
  const listener = (event: { payload: JobEvent }) => handler(event.payload);
  return onInvalidPayload
    ? listen(JOB_EVENT_NAME, listener, { onInvalidPayload })
    : listen(JOB_EVENT_NAME, listener);
}

export async function subscribeJobsInvalidated(
  handler: () => void,
  onInvalidPayload?: () => void,
): Promise<UnlistenFn> {
  return onInvalidPayload
    ? listen(JOB_EVENTS_INVALIDATED_NAME, handler, { onInvalidPayload })
    : listen(JOB_EVENTS_INVALIDATED_NAME, handler);
}
