import { useEffect, useState } from 'react';
import {
  getProjectPreferences,
  loadProjectPins,
  listProjects,
  subscribeProjectChanges,
  subscribeProjectPreferences,
  type Project,
} from '@/entities/project';

export function usePinnedProjects() {
  const [projects, setProjects] = useState<Project[]>([]);
  useEffect(() => {
    let cancelled = false;
    let generation = 0;
    const refresh = async () => {
      const sequence = ++generation;
      try {
        await loadProjectPins().catch(() => {});
        const items = await listProjects();
        if (!cancelled && sequence === generation) {
          setProjects((current) =>
            items
              .filter((item) => getProjectPreferences(item.id).pinned)
              .map((item) => {
                const previous = current.find((project) => project.id === item.id);
                return previous && previous.revision > item.revision ? previous : item;
              }),
          );
        }
      } catch {
        // Keep the last confirmed sidebar state when refresh is unavailable.
      }
    };
    const unsubscribe = subscribeProjectChanges((change) => {
      generation += 1;
      setProjects((current) =>
        change.type === 'removed'
          ? current.filter((project) => project.id !== change.projectId)
          : current.map((project) =>
              project.id === change.project.id && project.revision <= change.project.revision
                ? change.project
                : project,
            ),
      );
      void refresh();
    });
    void refresh();
    const unsubscribePreferences = subscribeProjectPreferences(() => void refresh());
    return () => {
      cancelled = true;
      unsubscribe();
      unsubscribePreferences();
    };
  }, []);
  return projects;
}
