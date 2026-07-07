import { invoke as tauriInvoke } from '@tauri-apps/api/core';
import type { InvokeArgs } from '@tauri-apps/api/core';
import type { Project, CreateProjectResponse } from '@/entities/project';
import type { Transcript } from '@/entities/transcript';
import type { Job } from '@/entities/job';
import type { MediaMetadata } from '@/entities/media';

export interface CommandMap {
  health_check: {
    args: undefined;
    result: string;
  };
  create_project_cmd: {
    args: { title: string };
    result: Project;
  };
  create_project_from_youtube_cmd: {
    args: { url: string };
    result: CreateProjectResponse;
  };
  get_transcript_cmd: {
    args: { projectId: string };
    result: Transcript | null;
  };
  start_mock_dubbing_job_cmd: {
    args: { input: string };
    result: Job;
  };
  list_jobs_cmd: {
    args: undefined;
    result: Job[];
  };
  cancel_job_cmd: {
    args: { jobId: string };
    result: Job;
  };
  probe_local_media_cmd: {
    args: { path: string };
    result: MediaMetadata;
  };
  import_local_media_cmd: {
    args: { projectId: string; path: string };
    result: Project;
  };
}

/**
 * Type-safe wrapper around Tauri's invoke.
 * Uses a CommandMap to ensure that command names, arguments, and return types are strongly typed.
 */
export async function invoke<K extends keyof CommandMap>(
  cmd: K,
  ...args: CommandMap[K]['args'] extends undefined ? [] : [CommandMap[K]['args']]
): Promise<CommandMap[K]['result']> {
  return tauriInvoke<CommandMap[K]['result']>(cmd, args[0] as InvokeArgs);
}
