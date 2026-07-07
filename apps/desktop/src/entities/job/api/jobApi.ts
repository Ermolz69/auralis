import { invoke, listen } from '@/shared/api/tauri';
import type { UnlistenFn } from '@tauri-apps/api/event';
import type { Job, JobEvent } from '../model/types';

export async function listJobs(): Promise<Job[]> {
  return invoke('list_jobs_cmd');
}

export async function startMockDubbingJob(input: string): Promise<Job> {
  return invoke('start_mock_dubbing_job_cmd', { input });
}

export async function cancelJob(jobId: string): Promise<Job> {
  return invoke('cancel_job_cmd', { jobId });
}

export async function subscribeJobEvents(handler: (event: JobEvent) => void): Promise<UnlistenFn> {
  return listen<JobEvent>('job-event', (event) => {
    handler(event.payload);
  });
}
