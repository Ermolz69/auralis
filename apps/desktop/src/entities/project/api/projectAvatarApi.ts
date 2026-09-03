import { invoke } from '@/shared/api/tauri';

export function getProjectAvatar(projectId: string) {
  return invoke('get_project_avatar_cmd', { projectId });
}

export function setProjectAvatar(projectId: string, dataUrl: string | null, onlyIfMissing = false) {
  return invoke('set_project_avatar_cmd', { projectId, dataUrl, onlyIfMissing });
}
