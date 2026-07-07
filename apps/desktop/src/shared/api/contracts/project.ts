import type { MediaMetadata, MediaSource } from './media';
import type { Job } from './job';

export type ProjectStatus =
  | 'draft'
  | 'source_imported'
  | 'ready_for_processing'
  | 'processing'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface Project {
  id: string;
  title: string;
  status: ProjectStatus;
  createdAt: string;
  updatedAt: string;
  source: MediaSource | null;
  metadata: MediaMetadata | null;
}

export interface CreateProjectResponse {
  project: Project;
  job: Job;
}
