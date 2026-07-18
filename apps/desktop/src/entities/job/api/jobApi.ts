import { invoke, listen } from '@/shared/api/tauri';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { Job, JobEvent } from '../model/types';

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
  return listen<JobEvent>('job-event', (event) => {
    handler(event.payload);
  });
}

export async function subscribeJobsInvalidated(handler: () => void): Promise<UnlistenFn> {
  return listen('job-events-invalidated', () => {
    handler();
  });
}
