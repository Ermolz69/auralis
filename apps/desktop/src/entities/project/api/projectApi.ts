import { invoke } from '@/shared/api/tauri';
import type { Project, CreateProjectResponse } from '../model/types';

export async function createProjectFromYoutube(url: string): Promise<CreateProjectResponse> {
  return invoke('create_project_from_youtube_cmd', { url });
}

export async function startProjectMockPipeline(projectId: string): Promise<CreateProjectResponse> {
  return invoke('start_project_mock_pipeline_cmd', { projectId });
}

export async function createProject(title: string): Promise<Project> {
  return invoke('create_project_cmd', { title });
}

export async function listProjects(): Promise<Project[]> {
  return invoke('list_projects_cmd');
}

export async function deleteProject(projectId: string): Promise<void> {
  await invoke('delete_project_cmd', { projectId });
}
