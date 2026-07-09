import type { Project, CreateProjectResponse } from './project';
import type { Transcript } from './transcript';
import type { Job } from './job';
import type { MediaMetadata } from './media';

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
  get_project_cmd: {
    args: { projectId: string };
    result: Project;
  };
  list_projects_cmd: {
    args: undefined;
    result: Project[];
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
