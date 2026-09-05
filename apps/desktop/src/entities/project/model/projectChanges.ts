import type { Project } from './types';
import { createWindowEventChannel } from '@/shared/lib';

export type ProjectChange =
  { type: 'updated'; project: Project } | { type: 'removed'; projectId: string };

const projectChangeEvent = 'auralis:project-changed';
const projectChanges = createWindowEventChannel<ProjectChange>(projectChangeEvent);

export function projectUpdated(project: Project) {
  projectChanges.emit({ type: 'updated', project });
}

export function projectRemoved(projectId: string) {
  projectChanges.emit({ type: 'removed', projectId });
}

export function subscribeProjectChanges(listener: (change: ProjectChange) => void) {
  return projectChanges.subscribe(listener);
}
