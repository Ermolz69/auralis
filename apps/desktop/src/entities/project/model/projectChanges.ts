import type { Project } from './types';

export type ProjectChange =
  { type: 'updated'; project: Project } | { type: 'removed'; projectId: string };

const projectChangeEvent = 'auralis:project-changed';

export function projectUpdated(project: Project) {
  window.dispatchEvent(
    new CustomEvent<ProjectChange>(projectChangeEvent, {
      detail: { type: 'updated', project },
    }),
  );
}

export function projectRemoved(projectId: string) {
  window.dispatchEvent(
    new CustomEvent<ProjectChange>(projectChangeEvent, {
      detail: { type: 'removed', projectId },
    }),
  );
}

export function subscribeProjectChanges(listener: (change: ProjectChange) => void) {
  const handle = (event: Event) => listener((event as CustomEvent<ProjectChange>).detail);
  window.addEventListener(projectChangeEvent, handle);
  return () => window.removeEventListener(projectChangeEvent, handle);
}
