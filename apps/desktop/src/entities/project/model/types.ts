import type { MediaMetadata, MediaSource } from '@/entities/media';
import type { Job } from '@/entities/job';

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
  source?: MediaSource;
  metadata?: MediaMetadata;
}

export interface CreateProjectResponse {
  project: Project;
  job: Job;
}
