import { invoke } from '@/shared/api/tauri';
import type { CreateProjectResponse } from '../model/types';
import type { Job } from '@/entities/job';

/**
 * Enhanced response typing including the Job type
 */
export interface CreateProjectFromYoutubeResponse extends CreateProjectResponse {
  job: Job;
}

export async function createProjectFromYoutube(url: string): Promise<CreateProjectFromYoutubeResponse> {
  return invoke<CreateProjectFromYoutubeResponse>('create_project_from_youtube_cmd', { url });
}
