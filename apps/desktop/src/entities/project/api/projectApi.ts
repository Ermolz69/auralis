import { invoke } from '@/shared/api/tauri';
import type { Project, CreateProjectResponse } from '../model/types';

export async function createProjectFromYoutube(url: string): Promise<CreateProjectResponse> {
  return invoke('create_project_from_youtube_cmd', { url });
}

export async function createProject(title: string): Promise<Project> {
  return invoke('create_project_cmd', { title });
}
