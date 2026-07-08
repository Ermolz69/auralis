import { invoke } from '@/shared/api/tauri';
import type { CreateProjectResponse } from '../model/types';

export async function createProjectFromYoutube(url: string): Promise<CreateProjectResponse> {
  return invoke('create_project_from_youtube_cmd', { url });
}
