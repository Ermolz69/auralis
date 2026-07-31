import { invoke, listen } from '@/shared/api/tauri';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { Job, JobEvent } from '../model/types';

const JOB_EVENT_NAME = 'job-event';
const JOB_EVENTS_INVALIDATED_NAME = 'job-events-invalidated';

export async function listJobs(): Promise<Job[]> {
  return invoke('list_jobs_cmd');
}

export async function cancelJob(jobId: string): Promise<Job> {
  return invoke('cancel_job_cmd', { jobId });
}

export async function getJobsSnapshot(projectId: string): Promise<Job[]> {
  return invoke('list_jobs_snapshot_cmd', { projectId });
}

export async function subscribeJobEvents(handler: (event: JobEvent) => void): Promise<UnlistenFn> {
  return listen<JobEvent>(JOB_EVENT_NAME, (event) => {
    handler(event.payload);
  });
}

export async function subscribeJobsInvalidated(handler: () => void): Promise<UnlistenFn> {
  return listen(JOB_EVENTS_INVALIDATED_NAME, () => {
    handler();
  });
}
