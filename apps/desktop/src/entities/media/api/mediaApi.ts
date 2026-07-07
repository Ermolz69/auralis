import { invoke } from '@/shared/api/tauri/invoke';
import type { Project } from '@/shared/api/contracts';
import type { MediaMetadata } from '../model/types';

export const probeLocalMedia = async (path: string): Promise<MediaMetadata> => {
  return invoke('probe_local_media_cmd', { path });
};

export const importLocalMedia = async (projectId: string, path: string): Promise<Project> => {
  return invoke('import_local_media_cmd', { projectId, path });
};
